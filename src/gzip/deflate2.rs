mod out_buf;

use bitvec::{vec::BitVec, view::BitView};
use miniz_oxide::MZStatus;

use self::out_buf::OutBuf;
use super::{
    huffmanencoding::{HuffmanEncoding, HuffmanMap},
    read_u16_le,
};
use crate::bufread::Item;
use bitvec::prelude::Lsb0;
use std::io::{self, BufRead, ErrorKind};

/// Size of decompressed chunks (except possibly the last chunk, which may be
/// smaller).
const OUT_CHUNK_SIZE: usize = 32 * 1024;

/// Implements iterator, producing chunks of bytes.
pub struct Decoder<R: BufRead> {
    input: R,
    state: DecoderState,
}

pub struct DecoderState {
    tree: Box<dyn HuffmanEncoding>,
    out_buf: OutBuf,
}

enum EncodingMethod {
    None,
    FixedHuffman,
    DynamicHuffman,
    Reserved,
}

impl EncodingMethod {
    /* BTYPE specifies how the data are compressed, as follows:
        00 - no compression
        01 - compressed with fixed Huffman codes
        10 - compressed with dynamic Huffman codes
        11 - reserved (error)
    */
    fn translate(bits: (bool, bool)) -> Self {
        match bits {
            (false, false) => EncodingMethod::None,
            (false, true) => EncodingMethod::FixedHuffman,
            (true, false) => EncodingMethod::DynamicHuffman,
            (true, true) => EncodingMethod::Reserved,
        }
    }
}

struct BlockHeader {
    last: bool,
    encoding_method: EncodingMethod,
}

impl BlockHeader {
    fn translate(bits: (bool, bool, bool)) -> Self {
        Self {
            last: bits.0,
            encoding_method: EncodingMethod::translate((bits.0, bits.1)),
        }
    }
}

impl DecoderState {
    pub fn new() -> Self {
        Self {
            tree: Box::new(HuffmanMap::new()),
            out_buf: OutBuf::with_capacity(OUT_CHUNK_SIZE),
        }
    }
}

impl<R: BufRead> Decoder<R> {
    pub fn resume(state: DecoderState, input: R) -> Self {
        Self { state, input }
    }

    pub fn into_state(self) -> DecoderState {
        self.state
    }

    pub fn next_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        loop {
            let mut header_byte = vec![0; 1];
            self.input.read_exact(&mut header_byte)?;
            let bits = header_byte.view_bits::<Lsb0>();

            let block_header = BlockHeader::translate((bits[0], bits[1], bits[2]));

            match block_header.encoding_method {
                // Non-compressed blocks (BTYPE=00)
                // Any bits of input up to the next byte boundary are ignored.
                // The rest of the block consists of the following information:
                //     0   1   2   3   4...
                //     +---+---+---+---+================================+
                //     |  LEN  | NLEN  |... LEN bytes of literal data...|
                //     +---+---+---+---+================================+
                // LEN is the number of data bytes in the block. NLEN is the one's complement of LEN.
                EncodingMethod::None => {
                    let body_len = read_u16_le(&mut self.input);
                    let neg_body_len = read_u16_le(&mut self.input);
                }
                EncodingMethod::FixedHuffman => todo!(),
                EncodingMethod::DynamicHuffman => todo!(),
                EncodingMethod::Reserved => todo!(),
            }

            let out_buf = self.state.out_buf.remaining();
        }
    }
}

impl<R: BufRead> Iterator for Decoder<R> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_chunk().transpose()
    }
}
