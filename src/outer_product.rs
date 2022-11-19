use std::collections::{BTreeMap, HashMap};

use crate::op::{CrossPOp, OpTrait, TransOp, VecOp};
use crate::util::{closest_factor, Tik};

#[derive(Debug)]
pub struct Slice {
    k: usize,
    m: usize,
    n: usize,
}

impl Slice {
    pub fn new(k: usize, m: usize, n: usize) -> Slice {
        Slice { k, m, n }
    }

    pub fn new_empty() -> Slice {
        Slice { k: 0, m: 0, n: 0 }
    }

    pub fn size(&self) -> usize {
        self.k * self.m * self.n
    }
}

pub struct OuterProduct {
    mapper_num: usize,
    reducer_num: usize,
    mapper_buf_size: usize,
    reducer_buf_size: usize,
    pub op_list: Vec<Box<dyn OpTrait>>,
    tik: Tik,
    mids: Vec<usize>,
    rids: Vec<usize>,
    mid_ofst: usize,
    rid_ofst: usize,
    amorph_sram: bool,
    mult_array: [usize; 2],
    add_array: [usize; 2],
    local_srams: Vec<usize>,
    reducer_remote_sram_size: usize,
    remote_alloc: BTreeMap<usize, Vec<[usize; 2]>>,
    remote_hold: BTreeMap<usize, Vec<[usize; 2]>>,
    m: usize,
    k: usize,
    n: usize,
    mapper_workload: Slice,
    reducer_workload: Slice,
    mapper_unit: Slice,
    reducer_unit: Slice,
}

impl OuterProduct {
    pub fn new(
        mapper_num: usize,
        reducer_num: usize,
        tile_sram_size: usize,
        amorph_sram: bool,
        mult_array: [usize; 2],
        add_array: [usize; 2],
    ) -> OuterProduct {
        let mid_ofst = 0;
        let rid_ofst = mapper_num;
        let midx2pid = (mid_ofst..mid_ofst + mapper_num).collect::<Vec<usize>>();
        let ridx2pid = (rid_ofst..rid_ofst + reducer_num).collect::<Vec<usize>>();
        let mut local_srams = vec![tile_sram_size; mapper_num + reducer_num];
        let mut remote_alloc: BTreeMap<usize, Vec<[usize; 2]>> = BTreeMap::new();
        let mut remote_hold: BTreeMap<usize, Vec<[usize; 2]>> = BTreeMap::new();
        let mut reducer_remote_sram_size = 0;
        // If amorphous, allocate remote sram to reducer.
        //      Specifically, mapper's comp density is (m*n)/(m+n) mult/element, reducer's comp density is 1 add/element.
        //      Therefore, we let each mapper & reducer to balance its computation & storage by renting/borrowing from each other.
        if amorph_sram {
            // Calc each reducer's remote sram size.
            let mapper_minimum_sram = mult_array.iter().sum::<usize>();
            let adder_maximum_sram = add_array.iter().product::<usize>();
            let rentable_sram = (tile_sram_size - mapper_minimum_sram).max(0) * mapper_num;
            // let demand_sram = (adder_maximum_sram - tile_sram_size).max(0) * reducer_num;
            // let remote_sram_size = rentable_sram.min(demand_sram);
            let remote_sram_size = rentable_sram;
            reducer_remote_sram_size = remote_sram_size / reducer_num;
            println!("Reducer remote sram size: {}", reducer_remote_sram_size);
            // Perform remote allocation.
            let mut mid = 0;
            for rid in ridx2pid.iter() {
                let mut remain_unalloc = reducer_remote_sram_size;
                while remain_unalloc > 0 {
                    if local_srams[mid] == 0 {
                        mid += 1;
                    }
                    let alloc_size = local_srams[mid].min(remain_unalloc);
                    local_srams[mid] -= alloc_size;
                    remain_unalloc -= alloc_size;
                    remote_alloc
                        .entry(mid)
                        .or_default()
                        .push([*rid, alloc_size]);
                    remote_hold.entry(*rid).or_default().push([mid, alloc_size]);
                    // println!("Remote hold: {:?}", remote_hold);
                    // println!("Local sram {} size {} alloc {}", mid, local_srams[mid], alloc_size);
                }
            }
        } else {
            for mid in midx2pid.iter() {
                remote_alloc.entry(*mid).or_default();
            }
            for rid in ridx2pid.iter() {
                remote_hold.entry(*rid).or_default();
            }
        }
        // Calc the unit computation for mapper and reducer.
        // Mapper performs a cross-product each
        let mapper_unit = Slice::new(1, mult_array[0], mult_array[1]);
        let reducer_unit = Slice::new(2, add_array[0], add_array[1]);

        OuterProduct {
            mapper_num,
            reducer_num,
            mapper_buf_size: tile_sram_size,
            reducer_buf_size: tile_sram_size,
            op_list: vec![],
            tik: Tik::new(),
            mids: midx2pid,
            rids: ridx2pid,
            mid_ofst,
            rid_ofst,
            amorph_sram,
            mult_array,
            add_array,
            local_srams,
            reducer_remote_sram_size,
            remote_alloc,
            remote_hold,
            // Initialize GEMM.
            m: 0,
            k: 0,
            n: 0,
            mapper_workload: Slice::new_empty(),
            reducer_workload: Slice::new_empty(),
            mapper_unit,
            reducer_unit,
        }
    }

    pub fn set_gemm(&mut self, m: usize, n: usize, k: usize) {
        self.m = m;
        self.k = k;
        self.n = n;
        // Perform gemm division onto mapper & reducer.
        // Generally, mappers divide on the k dim, reducers divide on the x/y dim.
        // Currently restrict k > mapper num and m*n > reducer num.
        assert!(
            self.k >= self.mapper_num && self.m * self.n > self.reducer_num,
            "K dim should be larger-equal than mapper num."
        );
        // mapper_k controls the granularity of execution. Each mapper_k are reduced by mapper,
        // The reduced fmap is further sent to reducer for reduction.
        // let mapper_k = (self.k + self.mapper_num - 1) / self.mapper_num;
        let mapper_k = 4;
        self.mapper_workload = Slice::new(mapper_k, self.m, self.n);
        let para_m = closest_factor(self.reducer_num, (self.reducer_num as f32).sqrt() as usize);
        let para_n = self.reducer_num / para_m;
        let reducer_m = (self.m + para_m - 1) / para_m;
        let reducer_n = (self.n + para_n - 1) / para_n;
        self.reducer_workload = Slice::new(self.mapper_num, reducer_m, reducer_n);
        println!("mapper_workload: {:?}", &self.mapper_workload);
        println!("reducer_workload: {:?}", &self.reducer_workload);
    }

    pub fn exec(&mut self) {
        let mut map_output_ops: Vec<usize> = vec![];
        let mut reduce_output_ops: Vec<usize> = vec![];
        let mut map2red_local_ops: BTreeMap<usize, (Vec<usize>, usize)> = BTreeMap::new();
        let mut map2red_remote_ops: BTreeMap<usize, Vec<[usize; 3]>> = BTreeMap::new();
        let mut map2red_memory_ops: BTreeMap<usize, (Vec<usize>, usize)> = BTreeMap::new();
        for k_ofst in (0..self.k).step_by(self.mapper_workload.k*self.mapper_num) {
            // Maper operations.
            for mid in self.mids.iter() {
                if k_ofst + mid >= self.k {
                    break;
                }
                // 1. Mapper fetch a single unit from memory.
                let trans_op = TransOp::new(
                    self.tik.tik(),
                    -1,
                    *mid as i32,
                    self.mapper_workload.size(),
                    map_output_ops.clone(),
                    format!("{} load map workload of k {} from memory.", mid, k_ofst+mid*self.mapper_workload.k),
                );
                map_output_ops.clear();
                // 2. Mapper calc m * n.
                let crossp_op = CrossPOp::new(
                    self.tik.tik(),
                    *mid,
                    self.mapper_workload.k,
                    self.mapper_workload.m,
                    self.mapper_workload.n,
                    vec![trans_op.idx],
                    format!(
                        "{} performs cross-product of {} x {} with k {}",
                        mid, self.mapper_workload.m, self.mapper_workload.n, self.mapper_workload.k
                    ),
                );
                let crossp_op_idx = crossp_op.idx;
                self.op_list.push(Box::new(trans_op));
                self.op_list.push(Box::new(crossp_op));
                for rid in self.rids.iter() {
                    let mut deps = vec![crossp_op_idx];
                    // 3. Mapper send results to reducer's local sram.
                    deps.extend(reduce_output_ops.clone());
                    let to_local_size = self.reducer_workload.size().min(self.local_srams[*rid]);
                    let map2red_local_op = TransOp::new(
                        self.tik.tik(),
                        *mid as i32,
                        *rid as i32,
                        to_local_size,
                        deps.clone(),
                        format!(
                            "Transfer from {} to local {}, data size {}",
                            mid, rid, to_local_size
                        ),
                    );
                    map2red_local_ops
                        .entry(*rid)
                        .and_modify(|e| {
                            e.0.push(map2red_local_op.idx);
                            e.1 += to_local_size;
                        })
                        .or_insert((vec![map2red_local_op.idx], to_local_size));
                    map_output_ops.push(map2red_local_op.idx);
                    self.op_list.push(Box::new(map2red_local_op));
                    // 4. Mapper send results to reducer's remote srams.
                    let mut map_remain_size = self.reducer_workload.size() - to_local_size;
                    // println!("remote_hold: {:?}", &self.remote_hold);
                    for remote_sram in self.remote_hold[rid].iter() {
                        if map_remain_size == 0 {
                            break;
                        }
                        let store_size = map_remain_size.min(remote_sram[1]);
                        map_remain_size -= store_size;
                        let map2red_remote_op = TransOp::new(
                            self.tik.tik(),
                            *mid as i32,
                            remote_sram[0] as i32,
                            store_size,
                            deps.clone(),
                            format!(
                                "Transfer from {} to remote {} of {}, data size {}",
                                mid, remote_sram[0], rid, store_size
                            ),
                        );
                        map2red_remote_ops.entry(*rid).or_default().push([
                            map2red_remote_op.idx,
                            remote_sram[0],
                            store_size,
                        ]);
                        map_output_ops.push(map2red_remote_op.idx);
                        self.op_list.push(Box::new(map2red_remote_op));
                    }
                    // 5. Transfer the rest to memory.
                    let map2red_remote_op = TransOp::new(
                        self.tik.tik(),
                        *mid as i32,
                        -1,
                        map_remain_size,
                        deps.clone(),
                        format!(
                            "Transfer from {} to {}, data size {}",
                            mid, -1, map_remain_size
                        ),
                    );
                    map2red_memory_ops
                        .entry(*rid)
                        .and_modify(|e| {
                            e.0.push(map2red_remote_op.idx);
                            e.1 += map_remain_size;
                        })
                        .or_insert((vec![map2red_remote_op.idx], map_remain_size));
                    map_output_ops.push(map2red_remote_op.idx);
                    self.op_list.push(Box::new(map2red_remote_op));
                }
            }
            // Reducer operations.
            for rid in self.rids.iter() {
                let mut output_op_deps = vec![];
                // 6. Reducer calc local data
                let local_size = map2red_local_ops[rid].1;
                let red_calc_local_op = VecOp::new(
                    self.tik.tik(),
                    *rid,
                    local_size,
                    map2red_local_ops[rid].0.clone(),
                    format!("Reducer {} calc local of size {}", rid, local_size),
                );
                output_op_deps.push(red_calc_local_op.idx);
                self.op_list.push(Box::new(red_calc_local_op));
                if map2red_remote_ops.contains_key(rid) {
                    for remote_data in map2red_remote_ops[rid].iter() {
                        // 7. Reducer fetch remote sram
                        let deps = vec![remote_data[0]];
                        let srcid = remote_data[1];
                        let remote_size = remote_data[2];
                        let red_fetch_remote_op = TransOp::new(
                            self.tik.tik(),
                            srcid as i32,
                            *rid as i32,
                            remote_size,
                            deps,
                            format!(
                                "Reducer {} fetch from {} of size {}",
                                rid, srcid, remote_size
                            ),
                        );
                        // 8. Reducer calc remote data
                        let red_remote_calc_op = VecOp::new(
                            self.tik.tik(),
                            *rid,
                            remote_size,
                            vec![red_fetch_remote_op.idx],
                            format!("Reducer {} calc size {}", rid, remote_size),
                        );
                        output_op_deps.push(red_remote_calc_op.idx);
                        self.op_list.push(Box::new(red_fetch_remote_op));
                        self.op_list.push(Box::new(red_remote_calc_op));
                    }
                }
                // 9. Reducer fetch from memory
                if map2red_memory_ops.contains_key(rid) {
                    let deps = map2red_memory_ops[rid].0.clone();
                    let mem_size = map2red_memory_ops[rid].1;
                    let red_fetch_mem_op = TransOp::new(
                        self.tik.tik(),
                        -1,
                        *rid as i32,
                        mem_size,
                        deps,
                        format!("Reducer {} fetch from memory of size {}", rid, mem_size),
                    );
                    // 10. Reducer calc memory data
                    let red_mem_calc_op = VecOp::new(
                        self.tik.tik(),
                        *rid,
                        mem_size,
                        vec![red_fetch_mem_op.idx],
                        format!("Reducer {} calc size {}", rid, mem_size),
                    );
                    output_op_deps.push(red_mem_calc_op.idx);
                    self.op_list.push(Box::new(red_fetch_mem_op));
                    self.op_list.push(Box::new(red_mem_calc_op));
                }
                // 11. Reducer output data.
                let output_size = self.reducer_workload.m * self.reducer_workload.n;
                let red_output_op = TransOp::new(
                    self.tik.tik(),
                    *rid as i32,
                    -1,
                    output_size,
                    output_op_deps,
                    format!("Reducer {} output of size {}", rid, output_size),
                );
                reduce_output_ops.push(red_output_op.idx);
                self.op_list.push(Box::new(red_output_op));
            }
        }
    }
}
