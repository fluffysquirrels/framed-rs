#![deny(warnings)]

extern crate framed;
extern crate framed_test_type as lib;

use lib::Test;
use std::io::stdout;

fn main() {
    let t = Test {
        a: 1,
        b: 2,
    };

    eprintln!("test_type/main.rs: Sending sample value: {:#?}", t);
    let mut s = framed::bytes::Config::default()
                                      .typed::<Test>()
                                      .into_sender(stdout());
    s.send(&t).unwrap();
}
