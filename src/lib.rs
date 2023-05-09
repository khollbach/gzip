mod bufread;
mod gzip;

use crate::bufread::BufReadAdapter;
use crate::gzip::Decoder;
use std::io::prelude::*;

pub fn decode(compressed_input: impl BufRead) -> impl BufRead {
    let decoder = Decoder::new(compressed_input);
    BufReadAdapter::new(decoder)
}
