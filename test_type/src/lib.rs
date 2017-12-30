#![deny(warnings)]

extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::fmt::{self, Debug, Formatter};

#[derive(Debug, Deserialize, Serialize)]
pub struct Test {
    pub a: u32,
    pub b: u16,
}

#[derive(Deserialize, Serialize)]
pub struct CustomDebug(Test);

impl Debug for CustomDebug {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "CustomDebug a=0x{:08x}, b=0x{:X}", self.0.a, self.0.b)
    }
}
