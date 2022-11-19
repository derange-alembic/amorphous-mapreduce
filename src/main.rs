mod op;
mod outer_product;
mod util;
mod word_count;

use outer_product::OuterProduct;
use word_count::WordCount;
use serde_json::{Value, Map};
use std::fs::File;
use std::io::{BufWriter, Write};

fn main() {
    let mut outp = OuterProduct::new(8, 8, 4096, true, [32, 32], [64, 32]);
    outp.set_gemm(128, 128, 512);
    outp.exec();

    let mut json_list = vec![];
    for op in outp.op_list.iter() {
        println!("{}", op.format_op());
        json_list.push(op.dump2json());
    }
    let file = File::create("result/outp.json").unwrap();
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &json_list).unwrap();
    writer.flush().unwrap();
    println!("{:?}", json_list);

}

pub fn word_count() {
    let file_path = "article/1.txt";
    let mut wordcount = WordCount::new(file_path, 4, 2, 2, 4).unwrap();
    while let Ok(byte_size) = wordcount.fill_mapper() {
        println!("byte_size: {}", byte_size);
        if byte_size == 0 {
            break;
        }
        wordcount.map();
        wordcount.reduce();
        println!("----");
    }
}
