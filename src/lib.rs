#![feature(generators, generator_trait, try_trait_v2)]

mod bufread_adapter;
mod deflate;
mod flags;

// // #[allow(dead_code)]
// mod deflate_miniz;

use std::{
    error::Error as StdError,
    io::{self, prelude::*, ErrorKind},
};

use bufread_adapter::BufReadAdapter;
pub use flags::Flags;

pub fn decode(compressed_input: impl BufRead) -> impl BufRead {
    BufReadAdapter::new(decode_all(compressed_input))
}

#[propane::generator]
fn decode_all(input: impl BufRead) -> io::Result<Vec<u8>> {
    let decoder = start_decoding(input)?;
    for chunk in decoder.decode_body_iter() {
        yield Ok(chunk?);
    }
}

pub fn start_decoding<R: BufRead>(compressed_input: R) -> io::Result<Decoder<R>> {
    let mut input = compressed_input;
    let mut header = read_required_headers(&mut input)?;
    read_optional_headers(&mut input, &mut header)?;
    Ok(Decoder { header, input })
}

#[propane::generator]
fn decode_body(mut input: impl BufRead) -> io::Result<Vec<u8>> {
    // (TODO: is this safe?)
    // This tricks `propane` into thinking we're not holding a reference across
    // a yield-point... but idk if the code is actually 'correct'.
    let raw_input: *mut _ = &mut input;
    for chunk in deflate::decode(unsafe { raw_input.as_mut().unwrap() }) {
        yield Ok(chunk?);
    }

    validate_footer(&mut input)?;
    validate_eof(&mut input)?;
}

pub struct Decoder<R: BufRead> {
    header: Header,
    /// Remaining input stream, after the headers have been consumed.
    input: R,
}

impl<R: BufRead> Decoder<R> {
    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn decode_body(self) -> impl BufRead {
        BufReadAdapter::new(self.decode_body_iter())
    }

    fn decode_body_iter(self) -> impl Iterator<Item = io::Result<Vec<u8>>> {
        decode_body(self.input)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Header {
    pub flags: Flags,
    pub mtime: u32,
    pub xflags: u8,
    pub os: u8,

    pub extra: Option<Vec<u8>>,
    pub name: Option<Vec<u8>>,
    pub comment: Option<Vec<u8>>,

    // todo: actually validate this somehow (if it exists)
    pub header_crc: Option<u16>,
}

fn read_required_headers(mut input: impl BufRead) -> io::Result<Header> {
    let mut header = [0; 10];
    input.read_exact(&mut header)?;

    let magic_number = [0x1f, 0x8b];
    let first_2 = &header[..2];
    if first_2 != magic_number {
        error(format!(
            "unrecognized gzip magic. \
            expected {magic_number:?}, got {first_2:?}"
        ))?;
    }

    let method = header[2];
    if method != 8 {
        let reserved = if method < 8 { "reserved value " } else { "" };
        error(format!(
            "unsupported compression method. \
            expected 8, got {reserved}{method}"
        ))?;
    }

    let flags = Flags::new(header[3])?;

    let mtime = &header[4..8]; // The modification time of the original uncompressed file.
    let xflags = header[8]; // May be used to indicate the level of compression performed.
    let os = header[9]; // The operating system / file system on which the compression took place.

    Ok(Header {
        flags,
        mtime: u32::from_le_bytes(mtime.try_into().unwrap()),
        xflags,
        os,
        extra: None,
        name: None,
        comment: None,
        header_crc: None,
    })
}

fn read_optional_headers(mut input: impl BufRead, required_headers: &mut Header) -> io::Result<()> {
    let header = required_headers;

    if header.flags.contains(Flags::EXTRA) {
        let mut buf = vec![0; read_u16_le(&mut input)? as usize];
        input.read_exact(&mut buf)?;
        header.extra = Some(buf);
    }

    // For now, we just discard the "original file name" field, if present.
    // In the future, we might want to provide an API for the user to get this value.
    if header.flags.contains(Flags::NAME) {
        let mut buf = vec![];
        input.read_until(0, &mut buf)?;
        if buf.ends_with(&[0]) {
            buf.pop();
        }
        header.name = Some(buf);
    }

    if header.flags.contains(Flags::COMMENT) {
        let mut buf = vec![];
        input.read_until(0, &mut buf)?;
        if buf.ends_with(&[0]) {
            buf.pop();
        }
        header.comment = Some(buf);
    }

    if header.flags.contains(Flags::HCRC) {
        // Ignored, as permitted by the RFC.
        // It would be nice to implement this. (todo)
        let header_crc = read_u16_le(&mut input)?;
        header.header_crc = Some(header_crc);
    }

    // We ignore this flag, as permitted by the RFC.
    // We're producing a stream of bytes anyways, so it doesn't matter if
    // it's hinted that the contents is probably text.
    let _is_text = header.flags.contains(Flags::TEXT);

    Ok(())
}

/// The gzip spec permits ignoring the CRC, but it would be nice to
/// implement this. (todo)
fn validate_footer(mut input: impl BufRead) -> io::Result<()> {
    let _crc = read_u32_le(&mut input)?;
    let _uncompressed_size_mod_32 = read_u32_le(&mut input)?;
    Ok(())
}

fn validate_eof(mut input: impl BufRead) -> io::Result<()> {
    if !input.fill_buf()?.is_empty() {
        // We could add support for gzip multi-streams at some point,
        // but they're almost never used. People prefer to simply `tar`
        // and then `gzip` if they're compressing multiple files.
        error(
            "expected eof, but got more bytes \
            (note: multiple gzip members not supported)",
        )?;

        // todo: does our impl still conform to the spec?
        // (maybe we should impl multi-streams...)
    }

    Ok(())
}

pub(crate) fn error<T, E>(error: E) -> io::Result<T>
where
    E: Into<Box<dyn StdError + Send + Sync>>,
{
    Err(io::Error::new(ErrorKind::Other, error))
}

pub(crate) fn read_u16_le(mut input: impl BufRead) -> io::Result<u16> {
    let mut buf = [0; 2];
    input.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

pub(crate) fn read_u32_le(mut input: impl BufRead) -> io::Result<u32> {
    let mut buf = [0; 4];
    input.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use flate2::{read::GzEncoder, Compression};
    use test_case::test_case;

    use super::*;

    /// Take as input a pre-gzipped text file of just "hello".
    ///
    /// Start decoding it and inspect the headers.
    ///
    /// Make sure they look like what we expect.
    #[test]
    fn hello_header() -> anyhow::Result<()> {
        let hello_gzipped: &str =
            "1f8b0808962a1365000368656c6c6f2e74787400cb48cdc9c9e7020020303a3606000000";
        let expected_header: Header = Header {
            flags: Flags::NAME,
            mtime: 1695754902,
            xflags: 0,
            os: 3,
            extra: None,
            name: Some(vec![104, 101, 108, 108, 111, 46, 116, 120, 116]),
            comment: None,
            header_crc: None,
        };

        let bytes = hex::decode(hello_gzipped)?;
        let decoder = start_decoding(bytes.as_slice())?;

        assert_eq!(decoder.header(), &expected_header);
        Ok(())
    }

    #[test_case(b"Hello world!")]
    #[test_case(b"abc")]
    #[test_case(b"A")]
    #[test_case(b"")]
    fn round_trip(input: &[u8]) -> anyhow::Result<()> {
        let compressed = gzip_compress(input);

        let mut decompressed = decode(compressed.as_slice());
        let mut bytes = vec![];
        decompressed.read_to_end(&mut bytes)?;

        assert_eq!(&bytes, input);
        Ok(())
    }

    fn gzip_compress(bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![];
        GzEncoder::new(bytes, Compression::default())
            .read_to_end(&mut out)
            .unwrap();
        out
    }
}
