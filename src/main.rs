mod word_count;

use word_count::WordCount;

fn main() {
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
    }
}
