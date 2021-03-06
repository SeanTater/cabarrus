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

// argument parsing
#[macro_use] extern crate clap;
// logging
#[macro_use] extern crate log;
extern crate env_logger;
// numpy-like arrays
extern crate ndarray;
extern crate ndarray_rand;
extern crate rand;
// better segmentation
extern crate unicode_segmentation;
// lastly, this library
extern crate cabarrus;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::cmp::min;
use std::collections::HashMap;
use ndarray::prelude::*;
use ndarray_rand::RandomExt;
use rand::SeedableRng;
use rand::distributions::Normal;
use unicode_segmentation::UnicodeSegmentation;

use cabarrus::warc::WarcStreamer;
use cabarrus::errors::*;
use cabarrus::numpy;

const RANK: usize = 1024;
const WINDOW_RADIUS: usize = 10;
const WINDOW_WIDTH: usize = 2 * WINDOW_RADIUS + 1;

pub fn main() {
    // Main can't return a Result, and the ? operator needs the enclosing function to return Result
    inner_main().expect("Could not recover. Exiting.");
}
pub fn inner_main() -> Result<()> {
    env_logger::init().unwrap();
    let args = app_from_crate!()
        .arg_from_usage("--context 'get the random (but consistent) context vectors instead of counting")
        .arg_from_usage("<wordlist> 'file containing words to look for, one per line'")
        .arg_from_usage("<output> 'file in which to store the resulting cooccurrence matrix'")
        .get_matches();

    // Read the word list from a file.
    let mut words = vec![];
    for line in BufReader::new(File::open(args.value_of("wordlist").unwrap())?).lines() {
        words.push(line?);
    }
    
    // Note that word 0 is the unknown word.
    let word_ids: HashMap<&str, usize> = words.iter()
        .enumerate()
        .map(|(id, word)| (word.as_ref(), id + 1))
        .collect();

    // Contexts is a random of uniformly distributed but deterministic floats which are almost
    // orthogonal vectors representing each word
    // Cooccurrences is an accumulator of those context vectors
    let mut rng = rand::StdRng::from_seed(&[3141592653589793]);
    let contexts: Array2<f64> = Array::random_using([words.len()+ 1, RANK],
        Normal::new(0., 1.),
        &mut rng);
    let mut cooccurrences: Array2<f64> = Array2::zeros([words.len() + 1, RANK]);
    
    if args.is_present("context") {
        // Just dump the contexts (not the usual way you'd use this program)
        numpy::write_matrix(args.value_of("output").unwrap(), &contexts)?;
    } else {
        // The usual case: count the words' cooccurrences
        if words.len() < 25 {
            info!("Collecting cooccurrences (with one another) of: {:?}", words);
        } else {
            info!("Collecting cooccurrences (with one another) of {} words.", words.len());
        }

        for rec in WarcStreamer::new()? {
            let mention_ids = tokenize(&rec, &word_ids);
            for mention_i in 0..mention_ids.len() {
                for context_i in mention_i..min(mention_ids.len(), WINDOW_WIDTH) {
                    cooccurrences
                    .row_mut(mention_ids[mention_i]) // row: center word
                    .scaled_add(1.0, // uniform window weight
                        &contexts.row(mention_ids[context_i])); // column: context word;
                }
            }
        }
        if words.len() <= 10 {
            println!("Cooccurrences look like {}", cooccurrences);
        }

        numpy::write_matrix(args.value_of("output").unwrap(), &cooccurrences)?;
    }

    Ok(())
}

/// Tokenize a string according to a dictionary. Unknowns will be 0.
pub fn tokenize(content: &str, ids: &HashMap<&str, usize>) -> Vec<usize> {
    /// Notice that we return indices (avoiding allocation)
    content
        // You can split on nonalphanumerics for a big speedup but the tokens are dubious.
        //.split(|c: char| ! c.is_alphanumeric())
        .split_word_bounds()
        .map(|mention| *ids.get(mention).unwrap_or(&0))
        // This will remove unknown words
        // but this is troublesome because it makes the context windows too wide if you have few words
        // because most words will be unknown
        //.filter(|i| *i > 0)
        .collect()
}
