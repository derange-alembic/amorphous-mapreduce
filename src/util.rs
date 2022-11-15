pub struct Tik {
    val: usize,
}

impl Tik {
    pub fn new() -> Tik {
        Tik {
            val: 0,
        }
    }

    pub fn tik(&mut self) -> usize {
        let cur = self.val;
        self.val += 1;
        cur
    }

    pub fn init(&mut self) {
        self.val = 0;
    }
}