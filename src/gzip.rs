#[allow(dead_code)]
mod deflate_miniz;

mod deflate;
pub mod flags;

use std::io::{self, prelude::*};

use crate::{error, read_u32_le};

// #[propane::generator]
// pub fn decode(mut input: impl BufRead) -> io::Result<Vec<u8>> {
//     let flags = read_required_headers(&mut input)?;
//     discard_optional_headers(&mut input, flags)?;

//     // (TODO: is this safe?)
//     // This tricks `propane` into thinking we're not holding a reference across
//     // a yield-point... but idk if the code is actually 'correct'.
//     let raw_input: *mut _ = &mut input;
//     for chunk in deflate_miniz::decode(unsafe { raw_input.as_mut().unwrap() }) {
//         yield Ok(chunk?);
//     }

//     validate_footer(&mut input)?;
//     validate_eof(&mut input)?;
// }

#[propane::generator]
pub fn decode_body(mut input: impl BufRead) -> io::Result<Vec<u8>> {
    // (TODO: is this safe?)
    // This tricks `propane` into thinking we're not holding a reference across
    // a yield-point... but idk if the code is actually 'correct'.
    let raw_input: *mut _ = &mut input;
    for chunk in deflate_miniz::decode(unsafe { raw_input.as_mut().unwrap() }) {
        yield Ok(chunk?);
    }

    validate_footer(&mut input)?;
    validate_eof(&mut input)?;
}

// fn read_required_headers(mut input: impl BufRead) -> io::Result<Flags> {
//     let mut header = [0; 10];
//     input.read_exact(&mut header)?;

//     let magic_number = [0x1f, 0x8b];
//     let first_2 = &header[..2];
//     if first_2 != magic_number {
//         error(format!(
//             "unrecognized gzip magic. \
//             expected {magic_number:?}, got {first_2:?}"
//         ))?;
//     }

//     let method = header[2];
//     if method != 8 {
//         let reserved = if method < 8 { "reserved value " } else { "" };
//         error(format!(
//             "unsupported compression method. \
//             expected 8, got {reserved}{method}"
//         ))?;
//     }

//     let flags = Flags::new(header[3])?;

//     // These aren't very useful (and in particular, the gzip RFC permits us to ignore them).
//     let _mtime = &header[4..8]; // The modification time of the original uncompressed file.
//     let _xflags = header[8]; // May be used to indicate the level of compression performed.
//     let _os = header[9]; // The operating system / file system on which the compression took place.

//     Ok(flags)
// }

// fn discard_optional_headers(mut input: impl BufRead, flags: Flags) -> io::Result<()> {
//     if flags.contains(Flags::EXTRA) {
//         let mut buf = vec![0; read_u16_le(&mut input)? as usize];
//         input.read_exact(&mut buf)?;
//     }

//     // For now, we just discard the "original file name" field, if present.
//     // In the future, we might want to provide an API for the user to get this value.
//     if flags.contains(Flags::NAME) {
//         let mut buf = vec![];
//         input.read_until(0, &mut buf)?;
//     }

//     if flags.contains(Flags::COMMENT) {
//         let mut buf = vec![];
//         input.read_until(0, &mut buf)?;
//     }

//     if flags.contains(Flags::HCRC) {
//         // Ignored, as permitted by the RFC.
//         // It would be nice to implement this. (todo)
//         let _header_crc = read_u16_le(&mut input)?;
//     }

//     // We ignore this flag, as permitted by the RFC.
//     // We're producing a stream of bytes anyways, so it doesn't matter if
//     // it's hinted that the contents is probably text.
//     let _is_text = flags.contains(Flags::TEXT);

//     Ok(())
// }

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
        let chunks = decode(Cursor::new(compressed));
        let decompressed: Vec<u8> = chunks.map(Result::unwrap).flatten().collect();
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
