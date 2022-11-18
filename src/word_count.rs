use crate::op::{OpTrait, TransOp, VecOp};
use crate::util::Tik;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct WordCount {
    reader: BufReader<File>,
    mapper_buffer: Vec<VecDeque<String>>,
    reducer_buffer: Vec<VecDeque<String>>,
    mapper_num: usize,
    reducer_num: usize,
    mapper_buf_size: usize,
    reducer_buf_size: usize,
    scoreboard: HashMap<String, usize>,
    pub op_list: Vec<Box<dyn OpTrait>>,
    tik: Tik,
    pub mid2pid: Vec<usize>,
    pub rid2pid: Vec<usize>,
}

impl WordCount {
    pub fn new(
        file_path: &'static str,
        mapper_num: usize,
        reducer_num: usize,
        mapper_buf_size: usize,
        reducer_buf_size: usize,
    ) -> Result<WordCount, Box<dyn Error>> {
        let f = File::open(file_path)?;
        let mid2pid = (0..mapper_num).collect::<Vec<usize>>();
        let rid2pid = (mapper_num..mapper_num + reducer_num).collect::<Vec<usize>>();
        Ok(WordCount {
            reader: BufReader::new(f),
            mapper_buffer: vec![VecDeque::new(); mapper_num],
            reducer_buffer: vec![VecDeque::new(); reducer_num],
            mapper_num,
            reducer_num,
            mapper_buf_size,
            reducer_buf_size,
            scoreboard: HashMap::new(),
            op_list: vec![],
            tik: Tik::new(),
            mid2pid,
            rid2pid,
        })
    }

    pub fn fill_mapper(&mut self) -> Result<usize, ()> {
        let mut bytes_num = 0;
        for map_idx in 0..self.mapper_num {
            let words = self.read_file(self.mapper_buf_size - self.mapper_buffer[map_idx].len());
            if words.len() == 0 {
                break;
            }
            bytes_num += words.len();
            println!("fill_mapper {}: {} bytes", map_idx, words.len());
            let op = TransOp::new(
                self.tik.tik(),
                -1,
                map_idx as i32,
                words.len(),
                vec![],
                "Memory to mapper.".to_string(),
            );
            self.op_list.push(Box::new(op));
            self.mapper_buffer[map_idx].push_back(words);
        }
        Ok(bytes_num)
    }

    fn read_file(&mut self, read_size: usize) -> String {
        let mut readouts = String::new();
        for _ in 0..read_size {
            let mut string = String::new();
            match self.reader.read_line(&mut string) {
                Ok(num_bytes) => {
                    println!("Read {} bytes.", num_bytes);
                }
                Err(_) => {
                    println!("Read file error.");
                }
            }
            readouts = readouts + &string;
        }
        readouts
    }

    pub fn map(&mut self) {
        let mut valid = true;
        while valid {
            valid = false;
            for map_idx in 0..self.mapper_num {
                let line = self.mapper_buffer[map_idx].pop_front();
                if let Some(line) = line {
                    let mut tokens: HashMap<usize, Vec<String>> = HashMap::new();
                    // Perform word splitting.
                    let words = line.trim().split_whitespace().collect::<Vec<_>>();
                    // Perform lowering & binning.
                    for word in words {
                        let mut lc_word = word.to_lowercase();
                        lc_word.retain(|c| c != ',' && c != '.');
                        let last_char = lc_word.chars().last().unwrap() as usize;
                        let bin_idx = last_char % self.reducer_num;
                        tokens.entry(bin_idx).or_default().push(lc_word);
                    }
                    println!("mapper {}: {:?}", map_idx, &tokens);
                    // Send tokens to corresponding reducers.
                    for (r_idx, ts) in tokens.into_iter() {
                        self.reducer_buffer[r_idx].extend(ts);
                    }
                    valid = true
                }
            }
        }
    }

    pub fn reduce(&mut self) {
        for (r_idx, buffer) in self.reducer_buffer.iter_mut().enumerate() {
            let mut board: HashMap<String, usize> = HashMap::new();
            while let Some(token) = buffer.pop_front() {
                board.entry(token).and_modify(|e| *e += 1).or_insert(1);
            }
            println!("reducer {}: {:?}", r_idx, &board);
            self.scoreboard.extend(board);
        }
    }
}
