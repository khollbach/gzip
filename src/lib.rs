#![feature(generators, generator_trait, try_trait_v2)]

mod bufread;
mod gzip;

use std::io::{self, prelude::*, ErrorKind};

use bufread::BufReadAdapter;
use gzip::flags::Flags;

pub fn decode(compressed_input: impl BufRead) -> impl BufRead {
    // BufReadAdapter::new(gzip::decode(compressed_input))

    let decoder = start_decoding(compressed_input).unwrap(); // TODO: propagate somehow
    decoder.decode_body()
}

pub fn start_decoding<R: BufRead>(compressed_input: R) -> io::Result<Decoder<R>> {
    let mut input = compressed_input;

    let mut header = read_required_headers(&mut input)?;
    read_optional_headers(&mut input, &mut header)?;
    Ok(Decoder { header, input })
}

use std::error::Error as StdError;

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
        BufReadAdapter::new(gzip::decode_body(self.input))
    }
}

#[derive(Debug)]
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

// fn crc(input) -> u16 {
//     let mut check = 0;
//     for byte in input {
//         check ^= byte;
//     }
//     check
// }

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
        header.name = Some(buf);
    }

    if header.flags.contains(Flags::COMMENT) {
        let mut buf = vec![];
        input.read_until(0, &mut buf)?;
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

// -> Decoder
// Decoder::bufreader() -> BufRead
// Decoder::headers ... () -> Option<&str>
// struct Headers { ... }
