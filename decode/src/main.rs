#![deny(warnings)]

#[macro_use]
extern crate clap;
#[macro_use]
extern crate derive_error;
extern crate framed;

include!(env!("FRAMED_DECODE_DYNAMIC_RS"));

mod error;
use error::{Error, Result};

// use clap::{App, Arg};
use framed::typed::Receiver;
use std::io::stdin;

fn main() {
    match try() {
        Ok(()) => (),
        Err(e) => eprintln!("Error: {}\n\
                             Detail: {:#?}", e, e),
    };
}

fn try() -> Result<()> {
    let app = app_from_crate!();
    let _matches = app.get_matches();
    let mut r = Receiver::<_, UserType>::new(stdin());

    loop {
        let v = r.recv();
        match v {
            Ok(v) => {
                println!("decode/main.rs: received value of type {}: {:#?}",
                         USER_TYPE_NAME, v);
            },
            Err(framed::Error::EofBeforeFrame) => return Ok(()),
            Err(e) => return Err(Error::from(e)),
        };
    }

    // Not reached.
}
