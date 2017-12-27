use framed;
use std;

#[derive(Debug, Error)]
pub enum Error {
    /// Error in library `framed`.
    #[error(non_std)]
    Framed(framed::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
