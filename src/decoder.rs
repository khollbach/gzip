mod flags;

use std::{
    io::{self, prelude::*},
    sync::mpsc::SyncSender,
};

use crate::DecodeError;

use self::flags::Flags;

/// Size of decompressed chunks (except possibly the last chunk, which may be
/// smaller).
const OUT_CHUNK_SIZE: usize = 32 * 1024;

pub struct Decoder<R: BufRead> {
    input: R,
}

impl<R: BufRead> Decoder<R> {
    pub fn new(input: R) -> Self {
        Self { input }
    }

    /// Decode the input stream, yielding output chunks to `out`.
    pub fn decode(mut self, out: SyncSender<io::Result<Vec<u8>>>) -> io::Result<()> {
        let flags = self.read_required_headers()?;
        self.discard_optional_headers(flags)?;

        self.decode_deflate_body(out)?;

        self.read_and_validate_footer()?;

        if self.input.fill_buf()?.is_empty() {
            Ok(())
        } else {
            // We could add support for gzip multi-streams at some point,
            // but they're almost never used. People prefer to simply `tar`
            // and then `gzip` if they're compressing multiple files.
            let msg = "multiple gzip members present but not supported";
            Err(DecodeError::Footer(msg.into()).into())
        }
    }

    fn read_required_headers(&mut self) -> io::Result<Flags> {
        let mut header = [0; 10];
        self.input.read_exact(&mut header)?;

        let magic_number = [0x1f, 0x8b];
        let first_2 = &header[..2];
        if first_2 != &magic_number {
            let msg = format!("unrecognized gzip magic; got {first_2:?}");
            return Err(DecodeError::Header(msg).into());
        }

        let method = header[2];
        if method != 8 {
            let reserved = if method < 8 { "reserved value " } else { "" };
            let msg =
                format!("unsupported compression method. expected 8 found {reserved}{method}");
            return Err(DecodeError::Header(msg).into());
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
            let mut buf = vec![0; self.read_u16_le()? as usize];
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
            let _header_crc = self.read_u16_le()?;
        }

        // We ignore this flag, as permitted by the RFC.
        // We're producing a stream of bytes anyways, so it doesn't matter if
        // it's hinted that the contents is probably text.
        let _is_text = flags.contains(Flags::TEXT);

        Ok(())
    }

    // TODO: The gzip spec permits ignoring the CRC,
    // but we may like to implement it as an optional check in a future CL.
    // (Same goes for the optional header CRC.)
    fn read_and_validate_footer(&mut self) -> io::Result<()> {
        let _crc = self.read_u32_le()?;
        let _uncompressed_size_mod_32 = self.read_u32_le()?;
        Ok(())
    }

    /// Yield output chunks to `out`.
    //
    // TODO: This implementation blocks on output
    // until it has consumed enough input to produce a full output buffer. Some API
    // consumers may like to have the output buffer flushed whenever reading from input
    // would block.
    fn decode_deflate_body(&mut self, _out: SyncSender<io::Result<Vec<u8>>>) -> io::Result<()> {
        todo!()

        // let mut mz_state = InflateState::new_boxed(DataFormat::Raw);

        // let mut output_buf = vec![0; *this.output_chunk_size];
        // let mut output_len = 0; // How much of the output buffer is currently filled.

        // loop {
        //     // Ensure more input is available. Note the deflate body is followed by a gzip
        //     // footer, so the stream should never dry up at this stage.
        //     let input_buf = this.input.fill_buf().await?;

        //     let info = mz_stream::inflate(
        //         &mut mz_state,
        //         &input_buf,
        //         &mut output_buf[output_len..],
        //         MZFlush::None,
        //     );

        //     let status = info.status.map_err(DecodeError::from)?;
        //     this.input.consume_unpin(info.bytes_consumed);
        //     output_len += info.bytes_written;

        //     // If we have a full output chunk, yield it.
        //     if output_len == output_buf.len() {
        //         let output_chunk = Bytes::copy_from_slice(&output_buf);
        //         out.yield_(output_chunk).await;
        //         output_len = 0;
        //     } else if output_len > output_buf.len() {
        //         panic!("logic error: over-full buffer");
        //     }

        //     match status {
        //         MZStatus::Ok => (),
        //         MZStatus::StreamEnd => {
        //             // Return a partial chunk with the rest of the output data.
        //             if output_len != 0 {
        //                 let output_chunk = Bytes::copy_from_slice(&output_buf[..output_len]);
        //                 out.yield_(output_chunk).await;
        //             }

        //             return Ok(());
        //         }
        //         // gzip doesn't support preset dictionaries, so this status will never be returned.
        //         MZStatus::NeedDict => unreachable!("miniz_oxide never returns NeedDict"),
        //     }
        // }
    }

    fn read_u16_le(&mut self) -> io::Result<u16> {
        let mut buf = [0; 2];
        self.input.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u32_le(&mut self) -> io::Result<u32> {
        let mut buf = [0; 4];
        self.input.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }
}
