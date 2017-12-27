#![deny(warnings)]

extern crate serde;
#[macro_use]
extern crate serde_derive;

#[derive(Debug, Deserialize, Serialize)]
pub struct Test {
    pub a: u32,
    pub b: u16,
}
