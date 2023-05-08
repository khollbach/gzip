mod deflate;
mod flags;
mod identity_decoder;

use self::flags::Flags;
use crate::bufread::Item;
use std::io::{self, prelude::*, ErrorKind};

pub struct Decoder<R: BufRead> {
    input: R,
    state: State,
}

#[derive(Default)]
enum State {
    #[default]
    Header,
    Body,
    Footer,
    Done,
}

impl<R: BufRead> Iterator for Decoder<R> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.next_state() {
                Err(e) => {
                    self.state = State::Done;
                    return Some(Err(e));
                }
                Ok(None) => match self.state {
                    State::Done => return None,
                    _ => continue,
                },
                Ok(Some(chunk)) => return Some(Ok(chunk)),
            }
        }
    }
}

impl<R: BufRead> Decoder<R> {
    pub fn new(input: R) -> Self {
        Self {
            input,
            state: State::default(),
        }
    }

    /// On success, transition states and possibly return an output chunk.
    ///
    /// On failure, just return the error (don't transition states).
    fn next_state(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut chunk = None;

        self.state = match self.state {
            State::Header => {
                let flags = self.read_required_headers()?;
                self.discard_optional_headers(flags)?;

                State::Body
            }
            State::Body => {
                chunk = self.body_next_chunk()?;

                if chunk.is_some() {
                    State::Body
                } else {
                    State::Footer
                }
            }
            State::Footer => {
                self.validate_footer()?;
                self.validate_eof()?;

                State::Done
            }
            State::Done => State::Done,
        };

        Ok(chunk)
    }

    fn read_required_headers(&mut self) -> io::Result<Flags> {
        let mut header = [0; 10];
        self.input.read_exact(&mut header)?;

        let magic_number = [0x1f, 0x8b];
        let first_2 = &header[..2];
        if first_2 != &magic_number {
            let msg = format!("unrecognized gzip magic; got {first_2:?}");
            return Err(io::Error::new(ErrorKind::Other, msg));
        }

        let method = header[2];
        if method != 8 {
            let reserved = if method < 8 { "reserved value " } else { "" };
            let msg = format!(
                "unsupported compression method. \
                expected 8, got {reserved}{method}"
            );
            return Err(io::Error::new(ErrorKind::Other, msg));
        }

        let flags = Flags::new(header[3])?;

        // These aren't very useful (and in particular, the gzip RFC permits us to ignore them).
        let _mtime = &header[4..8]; // The modification time of the original uncompressed file.
        let _xflags = header[8]; // May be used to indicate the level of compression performed.
        let _os = header[9]; // The operating system / file system on which the compression took place.

        Ok(flags)
    }

    fn discard_optional_headers(&mut self, flags: Flags) -> io::Result<()> {
        if flags.contains(Flags::EXTRA) {
            let mut buf = vec![0; read_u16_le(&mut self.input)? as usize];
            self.input.read_exact(&mut buf)?;
        }

        // For now, we just discard the "original file name" field, if present.
        // In the future, we might want to provide an API for the user to get this value.
        if flags.contains(Flags::NAME) {
            let mut buf = vec![];
            self.input.read_until(0, &mut buf)?;
        }

        if flags.contains(Flags::COMMENT) {
            let mut buf = vec![];
            self.input.read_until(0, &mut buf)?;
        }

        if flags.contains(Flags::HCRC) {
            // Ignored, as permitted by the RFC.
            // It would be nice to implement this.
            let _header_crc = read_u16_le(&mut self.input)?;
        }

        // We ignore this flag, as permitted by the RFC.
        // We're producing a stream of bytes anyways, so it doesn't matter if
        // it's hinted that the contents is probably text.
        let _is_text = flags.contains(Flags::TEXT);

        Ok(())
    }

    fn body_next_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut inner_decoder = identity_decoder::Decoder::new(&mut self.input);
        inner_decoder.next_chunk()
    }

    /// The gzip spec permits ignoring the CRC, but it would be nice to
    /// implement this.
    fn validate_footer(&mut self) -> io::Result<()> {
        let _crc = read_u32_le(&mut self.input)?;
        let _uncompressed_size_mod_32 = read_u32_le(&mut self.input)?;
        Ok(())
    }

    fn validate_eof(&mut self) -> io::Result<()> {
        if self.input.fill_buf()?.is_empty() {
            Ok(())
        } else {
            // We could add support for gzip multi-streams at some point,
            // but they're almost never used. People prefer to simply `tar`
            // and then `gzip` if they're compressing multiple files.
            let msg = "expected eof, but got more bytes \
                (note: multiple gzip members not supported)";
            Err(io::Error::new(ErrorKind::Other, msg))
        }
    }
}

fn read_u16_le(mut input: impl Read) -> io::Result<u16> {
    let mut buf = [0; 2];
    input.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u32_le(mut input: impl Read) -> io::Result<u32> {
    let mut buf = [0; 4];
    input.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{read::GzEncoder, Compression};
    use std::io::Cursor;
    use test_case::test_case;

    #[test_case(b"Hello world!")]
    #[test_case(b"abc")]
    #[test_case(b"A")]
    #[test_case(b"")]
    fn round_trip(uncompressed: &[u8]) {
        let compressed = gzip_compress(uncompressed);
        let decoder = Decoder::new(Cursor::new(compressed));
        let decompressed: Vec<u8> = decoder.map(Result::unwrap).flatten().collect();
        assert_eq!(&decompressed, uncompressed);
    }

    fn gzip_compress(bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![];
        GzEncoder::new(bytes, Compression::default())
            .read_to_end(&mut out)
            .unwrap();
        out
    }
}