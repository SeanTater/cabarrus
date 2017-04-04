//! Helper functions for example dist. sem. programs
//!
//! This code is intended to reduce the boilerplate in the included binaries and probably does not
//! serve much use elsewhere. But if you do use it, please consider citing it!


#[macro_use] extern crate log;
#[macro_use] extern crate nom;
extern crate ndarray;
extern crate farmhash;
extern crate hash_hasher;
extern crate linxal;
extern crate ndarray_linalg;
extern crate byteorder;
extern crate regex;
pub mod warc;
pub mod farm;
pub mod numpy;
pub mod errors;