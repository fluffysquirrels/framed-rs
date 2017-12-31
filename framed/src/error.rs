//! Representations of errors returned by this crate.

use ssmarshal;

#[cfg(feature = "use_std")]
use std::io;

#[cfg(feature = "use_std")]
use std::result;
#[cfg(not(feature = "use_std"))]
use core::result;

/// Type alias for results from this crate.
pub type Result<T> = result::Result<T, Error>;

/// Errors from this crate.
#[derive(Debug)]
pub enum Error {
    /// COBS decode failed
    CobsDecodeFailed,

    /// Checksum error: the received frame was corrupted.
    ChecksumError,

    /// End of data while reading a frame; we received some of a frame
    /// but it was incomplete.
    EofDuringFrame,

    /// End of data before a frame started; we received none of a frame.
    EofBeforeFrame,

    /// The supplied value was too short to be an encoded frame
    EncodedFrameTooShort,

    /// Forwarded io::Error.
    #[cfg(feature = "use_std")]
    Io(io::Error),

    /// Forwarded ssmarshal::Error.
    Ssmarshal(ssmarshal::Error),
}

impl Error {
    /// Returns true if the error represents a corrupted frame. Data
    /// may have been lost but the decoder should decode the next
    /// frame correctly.
    pub fn is_corrupt_frame(&self) -> bool {
        match *self {
            Error::ChecksumError |
            Error::CobsDecodeFailed |
            Error::EofDuringFrame
                => true,
            _ => false,
        }
    }
}

#[cfg(feature = "use_std")]
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<ssmarshal::Error> for Error {
    fn from(e: ssmarshal::Error) -> Error {
        Error::Ssmarshal(e)
    }
}
