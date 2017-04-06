//! Example 0: Word cooccurrence counter
//!
//! This simple script takes an input WET corpus piped to STDIN and counts cooccurrences of input
//! tokens, where tokens are defined by unicode, and cooccurrence is a window 21 words in
//! diameter, and only words from a newline-separated list are considered.
//!
//! The output is a numpy file with a cooccurrence matrix. The rows are the center word, starting
//! with the unknown word as word 0, and proceeding in the order they were specified in the input
//! word list. The columns are the context words, and cooccurrence is always counted as 1 or 0.
//!

// logging
#[macro_use] extern crate log;
extern crate env_logger;
// better segmentation
extern crate unicode_segmentation;
// lastly, this library
extern crate cabarrus;

use unicode_segmentation::UnicodeSegmentation;

use cabarrus::warc::WarcStreamer;
use cabarrus::errors::*;

pub fn main() {
    // Main can't return a Result, and the ? operator needs the enclosing function to return Result
    inner_main().expect("Could not recover. Exiting.");
}
pub fn inner_main() -> Result<()> {
    env_logger::init().unwrap();
    
    let term_count = WarcStreamer::new()?
        .map(|rec| rec.split_word_bounds().count())
        .sum();
    
    println!("{}", term_count);
    Ok(())
}
