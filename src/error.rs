//! Representations of errors returned by this crate.

#[cfg(feature = "use_std")]
use std::io;

#[cfg(feature = "use_std")]
use std::result;
#[cfg(not(feature = "use_std"))]
use core::result;

pub type Result<T> = result::Result<T, Error>;

#[cfg_attr(feature = "use_std", derive(Debug))]
pub enum Error {
    /// COBS decode failed
    CobsDecodeFailed,

    /// End of file while reading a frame
    EofDuringFrame,

    /// Forwarded io::Error.
    #[cfg(feature = "use_std")]
    Io(io::Error),

    /// Forwarded Io error.
    ///
    /// TODO: Store some extra value here.
    #[cfg(not(feature = "use_std"))]
    Io,
}

#[cfg(feature = "use_std")]
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}
