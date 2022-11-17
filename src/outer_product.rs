use std::collections::HashMap;

use crate::op::{OpTrait, TransOp, VecOp};
use crate::util::{Tik, closest_factor};

pub struct Slice {
    k: usize,
    m: usize,
    n: usize,
}

impl Slice {
    pub fn new(k: usize, m: usize, n: usize) -> Slice {
        Slice {
            k,
            m,
            n,
        }
    }

    pub fn new_empty() -> Slice {
        Slice {
            k: 0,
            m: 0,
            n: 0,
        }
    }
}

pub struct OuterProduct {
    mapper_num: usize,
    reducer_num: usize,
    mapper_buf_size: usize,
    reducer_buf_size: usize,
    pub op_list: Vec<Box<dyn OpTrait>>,
    tik: Tik,
    midx2pid: Vec<usize>,
    ridx2pid: Vec<usize>,
    mid_ofst: usize,
    rid_ofst: usize,
    amorph_sram: bool,
    mult_array: [usize; 2],
    add_array: [usize; 2],
    local_srams: Vec<usize>,
    reducer_remote_sram_size: usize,
    remote_alloc: HashMap<usize, Vec<[usize; 2]>>,
    remote_hold: HashMap<usize, Vec<[usize; 2]>>,
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
        let midx2pid = (mid_ofst..mid_ofst+mapper_num).collect::<Vec<usize>>();
        let ridx2pid = (rid_ofst..rid_ofst+reducer_num).collect::<Vec<usize>>();
        let mut local_srams = vec![tile_sram_size; mapper_num+reducer_num];
        let mut remote_alloc: HashMap<usize, Vec<[usize; 2]>> = HashMap::new();
        let mut remote_hold: HashMap<usize, Vec<[usize; 2]>> = HashMap::new();
        let mut reducer_remote_sram_size = 0;
        // If amorphous, allocate remote sram to reducer.
        //      Specifically, mapper's comp density is (m*n)/(m+n) mult/element, reducer's comp density is 1 add/element.
        //      Therefore, we let each mapper & reducer to balance its computation & storage by renting/borrowing from each other.
        if amorph_sram {
            // Calc each reducer's remote sram size.
            let mapper_minimum_sram = mult_array.iter().sum::<usize>();
            let adder_maximum_sram = add_array.iter().product::<usize>();
            let rentable_sram = (tile_sram_size - mapper_minimum_sram).max(0) * mapper_num;
            let demand_sram = (adder_maximum_sram - tile_sram_size).max(0) * reducer_num;
            let remote_sram_size = rentable_sram.min(demand_sram);
            reducer_remote_sram_size = remote_sram_size / reducer_num;
            // Perform remote allocation. 
            let mut midx = 0;
            let mut remain_rentable = rentable_sram;
            let mut remain_unalloc = reducer_remote_sram_size;
            for rid in ridx2pid.iter() {
                let mid = midx2pid[midx];
                while remain_unalloc > 0 {
                    if remain_rentable == 0 {
                        midx += 1;
                        remain_rentable = rentable_sram;
                    }
                    let alloc_size = remain_rentable.min(remain_unalloc);
                    remain_rentable -= alloc_size;
                    remain_unalloc -= alloc_size;
                    remote_alloc
                        .entry(mid)
                        .or_default()
                        .push([*rid, alloc_size]);
                    remote_hold
                        .entry(*rid)
                        .or_default()
                        .push([mid, alloc_size]);
                    local_srams[mid] -= alloc_size;
                }
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
            midx2pid,
            ridx2pid,
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
        // TODO: perform gemm division onto mapper & reducer.
        // Generally, mappers divide on the k dim, reducers divide on the x/y dim.
        // Currently restrict k > mapper num and m*n > reducer num.
        assert!(self.k > self.mapper_num && self.m * self.n > self.reducer_num,
            "K dim should be larger than mapper num.");
        let mapper_k = (self.k + self.mapper_num) / self.mapper_num;
        self.mapper_workload = Slice::new(mapper_k, self.m, self.n);
        let reducer_m = closest_factor(self.m * self.n, self.m * self.n / 2);
        let reducer_n = self.m * self.n / reducer_m;
        self.reducer_workload = Slice::new(self.k, reducer_m, reducer_n);
    }

}