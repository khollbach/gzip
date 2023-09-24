//! An implementation of deflate decoding, that just calls out to an existing
//! library. Useful for testing, probably.

mod out_buf;

use std::io::{self, BufRead, ErrorKind};

use miniz_oxide::{
    inflate::stream::{self as mz_stream, InflateState},
    DataFormat, MZFlush, MZStatus,
};
use out_buf::OutBuf;

/// Size of decompressed chunks (except possibly the last chunk, which may be
/// smaller).
const OUT_CHUNK_SIZE: usize = 32 * 1024;

#[propane::generator]
pub fn decode(mut input: impl BufRead) -> io::Result<Vec<u8>> {
    let mut mz_state = InflateState::new_boxed(DataFormat::Raw);
    let mut out_buf = OutBuf::with_capacity(OUT_CHUNK_SIZE);

    loop {
        // Use miniz-oxide to perform the deflate decoding.
        // todo: implement deflate by-hand yourself.
        let info = mz_stream::inflate(
            &mut mz_state,
            input.fill_buf()?,
            out_buf.remaining(),
            MZFlush::None,
        );
        let io_error = |e| io::Error::new(ErrorKind::Other, format!("{e:?}"));
        let status = info.status.map_err(io_error)?;

        input.consume(info.bytes_consumed);
        out_buf.advance(info.bytes_written);

        if out_buf.is_full() {
            let chunk = out_buf.take();
            yield Ok(chunk);
        }

        match status {
            MZStatus::Ok => continue,
            MZStatus::StreamEnd => {
                if !out_buf.is_empty() {
                    let partial_chunk = out_buf.take();
                    yield Ok(partial_chunk);
                }
                return;
            }
            // gzip doesn't support preset dictionaries, so this status will never be returned.
            MZStatus::NeedDict => unreachable!("miniz_oxide never returns NeedDict"),
        }
    }
}
