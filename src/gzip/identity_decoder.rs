use std::io::{self, BufRead};

/// A wrapper around a buffered reader.
///
/// Implements iterator, producing chunks of bytes.
///
/// Note that this copies the chunks into owned buffers. There might be a way to
/// borrow them from the original reader instead. (todo)
pub struct Decoder<R: BufRead> {
    input: R,
}

impl<R: BufRead> Decoder<R> {
    pub fn new(input: R) -> Self {
        Self { input }
    }
}

impl<R: BufRead> Iterator for Decoder<R> {
    type Item = io::Result<Vec<u8>>;

    /// Get the next chunk.
    ///
    /// Return `None` on EOF.
    fn next(&mut self) -> Option<Self::Item> {
        // Massage types in the case of "succesfully produced nothing" (eof).
        self.next_chunk().transpose()
    }
}

impl<R: BufRead> Decoder<R> {
    /// Helper function for `Iterator::next`. The same logic, but slightly
    /// different types.
    ///
    /// Return `Ok(None)` on EOF.
    pub fn next_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        let chunk = self.input.fill_buf()?;

        if chunk.is_empty() {
            Ok(None)
        } else {
            let chunk = chunk.to_vec();
            self.input.consume(chunk.len());
            Ok(Some(chunk))
        }
    }
}
