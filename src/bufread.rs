use std::io::{self, prelude::*, Cursor};

/// A wrapper around an iterator of chunks of bytes.
///
/// Implements `BufRead`.
pub struct BufReadAdapter<I: Iterator<Item = io::Result<Vec<u8>>>> {
    /// The (perhaps partially-consumed) current chunk.
    curr_chunk: Cursor<Vec<u8>>,

    /// The source of all future chunks.
    chunks: I,
}

impl<I: Iterator<Item = io::Result<Vec<u8>>>> BufReadAdapter<I> {
    pub fn new(chunks: I) -> Self {
        Self {
            curr_chunk: Cursor::default(),
            chunks,
        }
    }
}

impl<I: Iterator<Item = io::Result<Vec<u8>>>> BufRead for BufReadAdapter<I> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        // Re-fill the current chunk, if necessary.
        if self.curr_chunk.fill_buf()?.is_empty() {
            self.curr_chunk = match self.chunks.next().transpose()? {
                Some(new_chunk) => Cursor::new(new_chunk),
                None => Cursor::default(),
            };
        }

        self.curr_chunk.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.curr_chunk.consume(amount);
    }
}

impl<I: Iterator<Item = io::Result<Vec<u8>>>> Read for BufReadAdapter<I> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.fill_buf()?.read(buf)?;
        self.consume(n);
        Ok(n)
    }
}
