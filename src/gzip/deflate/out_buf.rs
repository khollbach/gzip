use std::io::Cursor;

/// A fixed size output buffer.
///
/// Intended to be used like [`std::io::Write`], but with an interface shaped
/// more like [`std::io::BufRead`]:
/// * The internal buffer is exposed via `remaining()`.
/// * The caller declares how many bytes they've written with `advance(n)`.
///
/// You can "consume" the current contents of the buffer into an owned chunk,
/// with `take()`.
///
/// todo: write a comment explaining why we need this
pub struct OutBuf {
    buf: Cursor<Vec<u8>>,
}

impl OutBuf {
    pub fn with_capacity(capacity: usize) -> Self {
        assert_ne!(capacity, 0);

        Self {
            buf: Cursor::new(vec![0; capacity]),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buf.position() == 0
    }

    pub fn is_full(&self) -> bool {
        let pos = self.buf.position() as usize;
        let len = self.buf.get_ref().len();
        pos >= len
    }

    pub fn remaining(&mut self) -> &mut [u8] {
        let pos = self.buf.position() as usize;
        &mut self.buf.get_mut()[pos..]
    }

    pub fn advance(&mut self, amount: usize) {
        let pos = self.buf.position() as usize + amount;
        self.buf.set_position(pos as u64);
    }

    /// Clone the contents of the buffer, and reset it to be empty.
    pub fn take(&mut self) -> Vec<u8> {
        let pos = self.buf.position() as usize;
        let contents = self.buf.get_ref()[..pos].to_vec();
        self.buf.set_position(0);
        contents
    }
}
