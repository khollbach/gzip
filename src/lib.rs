mod bufread;
mod gzip;

use bufread::BufReadAdapter;
use gzip::Decoder;
use std::io::prelude::*;

pub fn decode(compressed_input: impl BufRead) -> impl BufRead {
    let decoder = Decoder::new(compressed_input);
    BufReadAdapter::new(decoder)
}
