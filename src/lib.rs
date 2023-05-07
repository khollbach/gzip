/*

API notes:
* take a "Read" as input (maybe a BufRead? todo: think about this...)
* produce a BufRead as output (I think)

Conceptually, at a high level, we want to:
* read bytes until we have stripped all the gzip headers
    * for the most part, we're just discarding these.
* enter our main "read loop" where we're serving up decoded
    bytes and eating up the input as needed.

Since we'll be implementating BufRead on our returned value... how does one do this?
* fill_buf(&mut self) -> Result<&[u8]>;
* consume(&mut self, n: usize);

Maybe for starters, we can just impl an "identity" decoder. To get all the boilerplate right.

---

Thoughts on code "structure", specifically how to "yield" chunks in an ergonomic way:
* let's try making the "core" code return an iterator over chunks
* and then wrap this in an adapter that impl's BufRead
I don't see why this shouldn't work, so let's give it a shot :)

*/

mod decoder;
mod output_adapter;
mod errors;

use std::{
    io::prelude::*,
    sync::mpsc::{self},
    thread,
};

use decoder::Decoder;
use output_adapter::DecoderHandle;

pub use errors::DecodeError;

pub fn decode(compressed_input: impl BufRead + Send + 'static) -> impl BufRead {
    let (tx, rx) = mpsc::sync_channel(0);

    thread::spawn(move || {
        let d = Decoder::new(compressed_input);
        d.decode(tx).unwrap(); // todo: think about error handling -- it's currently fucky
    });

    DecoderHandle::new(rx)
}
