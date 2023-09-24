use std::{
    cmp::min,
    io::{self, BufRead},
};

use crate::gzip::{error, read_u16_le};

/// Size of decompressed chunks (except possibly the last chunk, which may be
/// smaller).
const OUT_CHUNK_SIZE: usize = 32 * 1024;

#[propane::generator]
pub fn decode(mut input: impl BufRead) -> io::Result<Vec<u8>> {
    // One iteration per block.
    loop {
        // Assume block header bits are at the start of a byte boundary, for now.
        // TODO: remove this assumption, when we figure out how to do bit-wise reading.
        let mut buf = [0u8; 1];
        input.read_exact(&mut buf)?;
        let header_bits = buf[0];
        let last_block = header_bits & 1 << 7 != 0;
        let btype = header_bits >> 5 & 0b_11;
        match btype {
            0b_00 => (),
            0b_01 => todo!(),
            0b_10 => todo!(),
            0b_11 => error("reserved block header bit pattern: 11")?,
            _ => unreachable!(),
        }

        // (throw away the rest of that byte, after those 3 bits)

        let len = read_u16_le(&mut input)?;
        let nlen = read_u16_le(&mut input)?;
        if len != !nlen {
            error(format!(
                "len and ~nlen don't match:\n{len:016b}\n{:016b}",
                !nlen
            ))?;
        }

        let mut remaining = len as usize;
        while remaining != 0 {
            let n = min(remaining, OUT_CHUNK_SIZE);
            remaining -= n;

            let mut chunk = vec![0; n];
            input.read_exact(&mut chunk)?;
            yield Ok(chunk);
        }

        if last_block {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use rand_chacha::{
        rand_core::{RngCore, SeedableRng},
        ChaCha8Rng,
    };

    use super::*;

    /// Generate 100KB of random garbage; call this the 'payload'.
    ///
    /// Create a deflate stream of 2 non-compressed blocks, containing the
    /// payload.
    ///
    /// Check that the deflate decoder can extract the original payload.
    #[test]
    fn non_compressed_blocks() -> anyhow::Result<()> {
        let mut payload = vec![0; 100_000];
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        rng.fill_bytes(&mut payload);

        // Generate input.
        let mut deflate_stream = Vec::with_capacity(payload.len() + 100);
        for chunk in payload.chunks(u16::MAX as usize) {
            let last_chunk = chunk.len() != u16::MAX as usize;
            let header_byte = if last_chunk {
                0b_1000_0000
            } else {
                0b_0000_0000
            };
            deflate_stream.write(&[header_byte])?;
            let len = chunk.len() as u16;
            deflate_stream.write(&len.to_le_bytes())?;
            deflate_stream.write(&(!len).to_le_bytes())?;
            deflate_stream.write(&chunk)?;
        }

        // Decode it.
        let chunks: Vec<_> = decode(Cursor::new(deflate_stream)).collect::<Result<_, _>>()?;
        assert!(chunks.len() >= 2);
        let decoded = chunks.concat();
        assert_eq!(decoded, payload);

        Ok(())
    }
}
