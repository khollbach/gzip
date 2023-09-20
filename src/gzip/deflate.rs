mod out_buf;

use std::io::{self, BufRead, ErrorKind};

use miniz_oxide::{
    inflate::stream::{self as mz_stream, InflateState},
    DataFormat, MZFlush, MZStatus,
};

use self::out_buf::OutBuf;

/// Size of decompressed chunks (except possibly the last chunk, which may be
/// smaller).
const OUT_CHUNK_SIZE: usize = 32 * 1024;

/// Implements iterator, producing chunks of bytes.
pub struct Decoder<R: BufRead> {
    input: R,
    state: DecoderState,
}

/// todo: write a comment explaining the "pause/resume" mechanic and why it's
/// necessary. (iirc its to do with ownership of the input stream.. TODO: what
/// about just using Rc to make that issue go away?)
pub struct DecoderState {
    mz_state: Box<InflateState>,
    out_buf: OutBuf,
}

impl DecoderState {
    pub fn new() -> Self {
        Self {
            mz_state: InflateState::new_boxed(DataFormat::Raw),
            out_buf: OutBuf::with_capacity(OUT_CHUNK_SIZE),
        }
    }
}

impl<R: BufRead> Decoder<R> {
    pub fn resume(state: DecoderState, input: R) -> Self {
        Self { state, input }
    }

    pub fn into_state(self) -> DecoderState {
        self.state
    }

    /// Helper function for `Iterator::next`.
    pub fn next_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        loop {
            // Use miniz-oxide to perform the deflate decoding.
            // todo: implement deflate by-hand yourself.
            let info = mz_stream::inflate(
                &mut self.state.mz_state,
                self.input.fill_buf()?,
                self.state.out_buf.remaining(),
                MZFlush::None,
            );
            let io_error = |e| io::Error::new(ErrorKind::Other, format!("{e:?}"));
            let status = info.status.map_err(io_error)?;

            self.input.consume(info.bytes_consumed);
            self.state.out_buf.advance(info.bytes_written);

            if self.state.out_buf.is_full() {
                let chunk = self.state.out_buf.take();
                return Ok(Some(chunk));
            }

            match status {
                MZStatus::Ok => continue,
                MZStatus::StreamEnd => {
                    if !self.state.out_buf.is_empty() {
                        let partial_chunk = self.state.out_buf.take();
                        return Ok(Some(partial_chunk));
                    } else {
                        return Ok(None);
                    }
                }
                // gzip doesn't support preset dictionaries, so this status will never be returned.
                MZStatus::NeedDict => unreachable!("miniz_oxide never returns NeedDict"),
            }
        }
    }
}

impl<R: BufRead> Iterator for Decoder<R> {
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_chunk().transpose()
    }
}
