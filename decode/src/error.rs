use clap;
use csv;
use framed;
use serde_json;
use std;

#[derive(Debug, Error)]
pub enum Error {
    /// Error in library `clap`.
    Clap(clap::Error),

    /// Error in library `csv`.
    Csv(csv::Error),

    /// Error in library `framed`.
    #[error(non_std)]
    Framed(framed::Error),

    /// Error in library `serde_json`.
    SerdeJson(serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
