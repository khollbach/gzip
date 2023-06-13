mod bufread;
mod gzip;

use std::io::prelude::*;

use crate::{bufread::BufReadAdapter, gzip::Decoder};

pub fn decode(compressed_input: impl BufRead) -> impl BufRead {
    let decoder = Decoder::new(compressed_input);
    BufReadAdapter::new(decoder)
}
