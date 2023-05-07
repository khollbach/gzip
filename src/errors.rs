use std::fmt::Debug;
use std::io;

/*

TODO: think more about error types

*/

// /// An error encountered during [`decode`].
// ///
// /// [`decode`]: crate::decode
// #[derive(Debug, thiserror::Error)]
// pub enum Error<E>
// where
//     E: std::error::Error + 'static,
// {
//     /// An error yielded by the underlying input stream
//     #[error(transparent)]
//     Input(E),

//     #[error("gzip decode error: {0}")]
//     Decode(#[from] DecodeError),
// }

// impl From<io::Error> for Error<io::Error> {
//     fn from(e: io::Error) -> Self {
//         Self::Input(e)
//     }
// }

/// A decoding error, due to malformed gzip data.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("malformed header: {0}")]
    Header(String),

    #[error("malformed footer: {0}")]
    Footer(String),

    // todo: more specific error message
    #[error("malformed DEFLATE body")]
    Deflate,

    /// Misc. catch-all, for things like unexpected eof.
    #[error(transparent)]
    Other(io::Error),
}

impl From<DecodeError> for io::Error {
    fn from(e: DecodeError) -> Self {
        io::Error::new(io::ErrorKind::Other, e)
    }
}
