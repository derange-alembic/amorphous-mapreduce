mod word_count;
mod outer_product;
mod op;
mod util;

use word_count::WordCount;
use outer_product::OuterProduct;

fn main() {
    let mut outp = OuterProduct::new(4, 2, 64, true, [8, 8], [8, 16]);
    outp.set_gemm(1024, 1024, 1024);
    outp.exec();
    for op in outp.op_list.iter() {
        println!("{}", op.format_op());
    }
}

pub fn word_count() {
    let file_path = "article/1.txt";
    let mut wordcount = WordCount::new(
        file_path,
        4,
        2,
        2,
        4
    ).unwrap();
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
