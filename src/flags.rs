use std::io;

use bitflags::bitflags;

use crate::error;

bitflags! {
    /// Gzip header flags.
    ///
    /// See RFC 1952 for detailed information about each flag.
    #[derive(Debug, PartialEq, Eq)]
    pub struct Flags: u8 {
        /// An optional indication that the payload is "probably ASCII text".
        const TEXT = 0b_0000_0001;

        /// If set, a CRC16 for the gzip header is present.
        const HCRC = 0b_0000_0010;

        /// If set, optional "extra" header fields are present.
        ///
        /// _Very_ unlikely to be useful. See here for some additional context:
        /// https://stackoverflow.com/q/65188890/
        const EXTRA = 0b_0000_0100;

        /// If set, an "original file name" is present.
        const NAME = 0b_0000_1000;

        /// If set, a "file comment" is present, intended for human consumption.
        const COMMENT = 0b_0001_0000;
    }
}

impl Flags {
    /// Return an error if any reserved bit is set.
    pub fn new(flag_byte: u8) -> io::Result<Flags> {
        match Flags::from_bits(flag_byte) {
            Some(flags) => Ok(flags),
            None => error(format!(
                "reserved bit set in gzip flag byte: {flag_byte:08b}"
            )),
        }
    }
}
