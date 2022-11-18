mod op;
mod outer_product;
mod util;
mod word_count;

use outer_product::OuterProduct;
use word_count::WordCount;

fn main() {
    let mut outp = OuterProduct::new(16, 8, 2048, false, [32, 32], [64, 32]);
    outp.set_gemm(16, 16, 16);
    outp.exec();
    for op in outp.op_list.iter() {
        println!("{}", op.format_op());
    }
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
