use std::{
    io::{self, prelude::*, Cursor},
    sync::mpsc::{Receiver, RecvError},
};

/// Receives decoded output chunks from the decoder thread.
///
/// Implements `BufRead`.
pub struct DecoderHandle {
    /// The (perhaps partially-consumed) current chunk.
    curr_chunk: Cursor<Vec<u8>>,

    /// The source of all future chunks.
    chunks: Receiver<io::Result<Vec<u8>>>,
}

impl DecoderHandle {
    pub fn new(chunks: Receiver<io::Result<Vec<u8>>>) -> Self {
        Self {
            chunks,
            curr_chunk: Cursor::default(),
        }
    }
}

impl BufRead for DecoderHandle {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.curr_chunk.fill_buf()?.is_empty() {
            // Wait for a new chunk.
            self.curr_chunk = match self.chunks.recv() {
                Ok(new_chunk) => Cursor::new(new_chunk?),
                // No more chunks are coming; we're all done.
                Err(RecvError) => Cursor::default(),
            };
        }

        self.curr_chunk.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.curr_chunk.consume(amt)
    }
}

impl Read for DecoderHandle {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.fill_buf()?.read(buf)
    }
}
