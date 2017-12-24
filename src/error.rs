//! Representations of errors returned by this crate.

#[cfg(feature = "typed")]
use ssmarshal;

#[cfg(feature = "use_std")]
use std::io;

#[cfg(feature = "use_std")]
use std::result;
#[cfg(not(feature = "use_std"))]
use core::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// COBS decode failed
    CobsDecodeFailed,

    /// End of data while reading a frame
    EofDuringFrame,

    /// The supplied value was too short to be an encoded frame
    EncodedFrameTooShort,

    /// Forwarded io::Error.
    #[cfg(feature = "use_std")]
    Io(io::Error),

    /// Forwarded ssmarshal::Error.
    #[cfg(feature = "typed")]
    Ssmarshal(ssmarshal::Error),
}

#[cfg(feature = "use_std")]
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

#[cfg(feature = "typed")]
impl From<ssmarshal::Error> for Error {
    fn from(e: ssmarshal::Error) -> Error {
        Error::Ssmarshal(e)
    }
}
