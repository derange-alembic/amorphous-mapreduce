pub struct Tik {
    val: usize,
}

impl Tik {
    pub fn new() -> Tik {
        Tik { val: 0 }
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

pub fn closest_factor(value: usize, factor: usize) -> usize {
    println!("closest_factor: {} {}", value, factor);
    let mut res = vec![];
    if factor >= 1 {
        let mut f = factor + 1;
        while f > factor {
            f -= 1;
        }
        loop {
            if f != 0 && value % f == 0 {
                break;
            }
            f -= 1;
        }
        println!("closest_factor: {}", f);
        res.push(f);
    }

    if factor <= value {
        let mut f = factor - 1;
        while f < factor {
            f += 1;
        }
        loop {
            if f != 0 && value % f == 0 {
                break;
            }
            f += 1;
        }
        println!("closest_factor: {}", f);
        res.push(f);
    }

    let a = res[0] - factor;
    let b = factor - res[1];
    if a > b {
        return res[1];
    } else {
        return res[0];
    }
}
