pub enum OpType {
    TransOp,
    VecOp,
}

pub trait OpTrait {

}

pub struct TransOp {
    pub idx: usize,
    pub deps: Vec<usize>,
    pub op_type: OpType,
    src: i32,
    dst: i32,
    length: usize,
    content: String,
}

impl TransOp {
    pub fn new(idx: usize, src: i32, dst: i32, length: usize, deps: Vec<usize>, content: String)
        -> TransOp {
        TransOp {
            idx,
            deps,
            op_type: OpType::TransOp,
            src,
            dst,
            length,
            content,
        }
    }
}

impl OpTrait for TransOp {

}

pub struct VecOp {
    pub idx: usize,
    pub deps: Vec<usize>,
    pub op_type: OpType,
    length: usize,
    pid: usize,
    content: String,
}

impl VecOp {
    pub fn new(idx: usize, pid: usize, length: usize, deps: Vec<usize>, content: String) -> VecOp {
        VecOp {
            idx,
            deps,
            op_type: OpType::VecOp,
            length,
            pid,
            content,
        }
    }
}

impl OpTrait for VecOp {

}