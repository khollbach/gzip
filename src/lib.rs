#![feature(generators, generator_trait, try_trait_v2)]

mod bufread;
mod gzip;

use std::io::prelude::*;

use bufread::BufReadAdapter;

pub fn decode(compressed_input: impl BufRead) -> impl BufRead {
    BufReadAdapter::new(gzip::decode(compressed_input))
}
