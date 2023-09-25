use std::{
    fs::File,
    io::{self, BufReader},
};

use gzip::start_decoding;

fn main() -> io::Result<()> {
    let reader = File::open("hello.txt.gz")?;
    let decoder = start_decoding(BufReader::new(reader))?;

    dbg!(decoder.header());

    io::copy(&mut decoder.decode_body(), &mut io::stdout())?;

    Ok(())
}
