//! Run the input embedding matrix (and associated wordlist) against the Mikolav 2013 analogy
//! dataset
//!

// argument parsing
#[macro_use] extern crate clap;
// logging
#[macro_use] extern crate log;
extern crate env_logger;
// numpy-like arrays
extern crate ndarray;
// better segmentation
extern crate unicode_segmentation;
// lastly, this library
extern crate cabarrus;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;
use ndarray::prelude::*;

use cabarrus::errors::*;
use cabarrus::numpy;

pub fn main() {
    // Main can't return a Result, and the ? operator needs the enclosing function to return Result
    inner_main().expect("Could not recover. Exiting.");
}
pub fn inner_main() -> Result<()> {
    env_logger::init().unwrap();
    let args = app_from_crate!()
        .arg_from_usage("<wordlist> 'one word per line corresponding to rows 1.. of the embedding (0=unknown)'")
        .arg_from_usage("<embedding> 'linear word embeddings as a Numpy file'")
        .get_matches();

    // Embed the analogy table in the executable
    let mut analogies : Vec<(&str, &str, &str, &str)> = vec![];
    for line in include_str!("metrics-mikolav2013-word-analogies.txt").lines() {
        let l : Vec<&str>= line.split_whitespace().collect();
        if l.len() == 4 {
            analogies.push((l[0], l[1], l[2], l[3]));
        }
    }
    
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

    // This will be a table with rows of center words and columns of context words
    // It could be usize instead of f64 but this is easier for interop
    // and its range is still enough not to be a problem.
    let mut embedding: Array2<f64> = numpy::read_matrix(args.value_of("embedding").unwrap())?;
    
    // L2 normalize the embedding.
    // Cosine similarity equal to just a dot product if the magnitudes are all one.
    // If they are 0, then they can't be normalized. They stay 0 and their similarities will all
    // be 0, when in fact they should be undefined.
    for mut row in embedding.outer_iter_mut() {
        let sum_sq = row.fold(0.0, |acc, x| acc + (x*x));
        if sum_sq != 0.0 { row.mapv_inplace(|x| x/sum_sq); }
    }
    
    // Get the vector encoding from the embedding corresponding to a word
    let get_vector_idx = |mention: &str| *word_ids.get(mention).unwrap_or(&0);
    let get_vector = |mention| embedding.row(get_vector_idx(mention));
    
    // This allows sorting (and taking max) of floating points, which is forbidden because of NaN
    // It's not a good idea in production but we assume if there is a NaN in your embedding you
    // would want to know anyway so early failure (read "crash") is easy and not so bad
    #[derive(PartialOrd, PartialEq)]
    struct Fp(f64);
    impl Eq for Fp {}
    impl Ord for Fp {
        fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
            self.partial_cmp(other).expect("NaN or infinity in embedding; cannot create metrics.")
        }
    }
    
    // Grade the embedding
    let mut correct = 0.0f64;
    let total = analogies.len() as f64;
    for (ix, (from_a, to_a, from_b, to_b)) in analogies.into_iter().enumerate() {
        let analog = &get_vector(to_a) - &get_vector(from_a) + &get_vector(from_b);
        let argmax = embedding.dot(&analog).indexed_iter()
            .max_by_key(|&(_idx, score)| Fp(*score))
            .expect("Empty embedding. You need at least one vector.")
            .0;
        if argmax == get_vector_idx(to_b) {
            correct += 1.0;
        }
        println!("Completed analogy {}", ix);
    }
    
    println!("{} of {} are correct, {:.3}%", correct, total, 100.0 * (correct / total));
    Ok(())
}
