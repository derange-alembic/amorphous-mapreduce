use std::{io::{BufReader, BufRead}, collections::{VecDeque, HashMap}, fs::File, error::Error, hash::Hash, ops::Add};

struct WordCount {
    reader: BufReader<File>,
    mapper_buffer: Vec<VecDeque<String>>,
    reducer_buffer: Vec<VecDeque<String>>,
    mapper_num: usize,
    reducer_num: usize,
    mapper_buf_size: usize,
    reducer_buf_size: usize,
    scoreboard: HashMap<String, usize>
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
        Ok(WordCount {
            reader: BufReader::new(f),
            mapper_buffer: vec![VecDeque::new(); mapper_num],
            reducer_buffer: vec![VecDeque::new(); reducer_num],
            mapper_num,
            reducer_num,
            mapper_buf_size,
            reducer_buf_size,
            scoreboard: HashMap::new(),
        })
    }

    pub fn fill_mapper(&mut self) -> Result<(), Box<dyn Error>> {
        for map_idx in 0..self.mapper_num {
            let words = self.read_file(
                self.mapper_buf_size - self.mapper_buffer[map_idx].len())?;
            self.mapper_buffer[map_idx].extend(words);
        }

        Ok(())
    }

    pub fn read_file(&mut self, read_size: usize) -> Result<Vec<String>, Box<dyn Error>> {
        let mut readouts = vec![];
        for _ in 0..read_size {
            let mut string = String::new();
            self.reader.read_line(&mut string)?;
            readouts.push(string);
        }

        Ok(readouts)
    }

    pub fn map(&mut self) {
        let mut valid = true;
        while !valid {
            valid = false;
            for map_idx in 0..self.mapper_num {
                let line = self.mapper_buffer[map_idx].pop_front();
                if let Some(line) = line {
                    let mut tokens: HashMap<usize, Vec<String>> = HashMap::new();
                    // Perform word splitting.
                    let words = line
                        .trim()
                        .split_whitespace()
                        .collect::<Vec<_>>();
                    // Perform lowering & binning.
                    for word in words {
                        let lc_word = word.to_lowercase();
                        let last_char = lc_word.chars().last().unwrap() as usize;
                        let bin_idx = last_char % self.reducer_num;
                        tokens
                            .entry(bin_idx)
                            .or_default()
                            .push(lc_word);
                    }
                    // Send tokens to corresponding reducers.
                    for (r_idx, ts) in tokens.into_iter() {
                        self.reducer_buffer[r_idx].extend(ts);
                    }
                } else {
                    valid = valid | true;
                }
            }   
        }
    }

    pub fn reduce(&mut self) {
        for reduce_idx in 0..self.reducer_num {
            for buffer in self.reducer_buffer.iter_mut() {
                let mut board: HashMap<String, usize> = HashMap::new();
                while let Some(token) = buffer.pop_front() {
                    board.entry(token)
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }
                self.scoreboard.extend(board);
            }
        }
    }
}

