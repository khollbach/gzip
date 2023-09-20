mod deflate;
mod flags;

// todo: does the gzip spec allow identity encoding?
#[allow(unused)]
mod identity_decoder;

use std::{
    io::{self, prelude::*, ErrorKind},
    mem,
};

use self::flags::Flags;
use crate::bufread::Item;

pub struct Decoder<R: BufRead> {
    input: R,
    state: State,
}

enum State {
    Header,
    Body(deflate::DecoderState),
    Footer,
    Done,
}

impl<R: BufRead> Iterator for Decoder<R> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        // Call `next_state` until it:
        // * produces a chunk or an error, or
        // * transitions to the Done state.
        //
        // If it returns None and state != Done, keep polling.

        loop {
            let item = self.next_state().transpose();

            if item.is_some() || matches!(self.state, State::Done) {
                return item;
            }
        }
    }
}

impl<R: BufRead> Decoder<R> {
    pub fn new(input: R) -> Self {
        Self {
            input,
            state: State::Header,
        }
    }

    /// On success, transition states and possibly return an output chunk.
    ///
    /// On failure, transition to the `Done` state and return an error.
    ///
    /// Note that Ok(None) doesn't necessarily mean EOF -- you have to check for
    /// self.state == Done as well.
    fn next_state(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut chunk = None;

        // Replace self.state with Done, in case we bail via the `?` operator.
        let old_state = mem::replace(&mut self.state, State::Done);

        let new_state = match old_state {
            State::Header => {
                let flags = self.read_required_headers()?;
                self.discard_optional_headers(flags)?;

                State::Body(deflate::DecoderState::new())
            }
            State::Body(state) => {
                let mut inner_decoder = deflate::Decoder::resume(state, &mut self.input);
                chunk = inner_decoder.next_chunk()?;

                if chunk.is_some() {
                    State::Body(inner_decoder.into_state())
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

        self.state = new_state;

        Ok(chunk)
    }

    fn read_required_headers(&mut self) -> io::Result<Flags> {
        let mut header = [0; 10];
        self.input.read_exact(&mut header)?;

        let magic_number = [0x1f, 0x8b];
        let first_2 = &header[..2];
        if first_2 != magic_number {
            let msg = format!(
                "unrecognized gzip magic. \
                expected {magic_number:?}, got {first_2:?}"
            );
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
            // It would be nice to implement this. (todo)
            let _header_crc = read_u16_le(&mut self.input)?;
        }

        // We ignore this flag, as permitted by the RFC.
        // We're producing a stream of bytes anyways, so it doesn't matter if
        // it's hinted that the contents is probably text.
        let _is_text = flags.contains(Flags::TEXT);

        Ok(())
    }

    /// The gzip spec permits ignoring the CRC, but it would be nice to
    /// implement this. (todo)
    fn validate_footer(&mut self) -> io::Result<()> {
        let _crc = read_u32_le(&mut self.input)?;
        let _uncompressed_size_mod_32 = read_u32_le(&mut self.input)?;
        Ok(())
    }

    fn validate_eof(&mut self) -> io::Result<()> {
        if self.input.fill_buf()?.is_empty() {
            Ok(())
        } else {
            // todo: does our impl still conform to the spec?
            // (maybe we should impl multi-streams...)

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
    use std::io::Cursor;

    use flate2::{read::GzEncoder, Compression};
    use test_case::test_case;

    use super::*;

    #[test_case(b"Hello world!")]
    #[test_case(b"abc")]
    #[test_case(b"A")]
    #[test_case(b"")]
    fn round_trip(input: &[u8]) {
        let compressed = gzip_compress(input);
        let decoder = Decoder::new(Cursor::new(compressed));
        let decompressed: Vec<u8> = decoder.map(Result::unwrap).flatten().collect();
        assert_eq!(&decompressed, input);
    }

    fn gzip_compress(bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![];
        GzEncoder::new(bytes, Compression::default())
            .read_to_end(&mut out)
            .unwrap();
        out
    }
}
