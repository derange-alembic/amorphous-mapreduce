use std::{io::{BufReader, BufRead}, collections::VecDeque, fs::File, error::Error};

struct WordCount {
    reader: BufReader<File>,
    mapper_buffer: Vec<VecDeque<String>>,
    reducer_buffer: Vec<VecDeque<String>>,
    mapper_num: usize,
    reducer_num: usize,
    mapper_buf_size: usize,
    reducer_buf_size: usize,
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
                    let words = line
                        .trim()
                        .split_whitespace()
                        .collect::<Vec<_>>();
                    for word in words {
                        let lc_word = word.to_lowercase();
                    }
                } else {
                    valid = valid | true;
                }
            }
        }
    }
}

