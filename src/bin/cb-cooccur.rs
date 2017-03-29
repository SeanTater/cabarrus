extern crate cabarrus;
#[macro_use] extern crate clap;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate ndarray;
extern crate farmhash;
extern crate unicode_segmentation;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io;
use std::collections::HashMap;
use cabarrus::warc::WarcStreamer;
use ndarray::prelude::*;
use std::cmp::min;
use unicode_segmentation::UnicodeSegmentation;

const WINDOW_RADIUS: usize = 10;
const WINDOW_WIDTH: usize = 2 * WINDOW_RADIUS + 1;

fn main() {
    // Main can't return a Result.
    inner_main().expect("Could not recover. Exiting.");
}
fn inner_main() -> io::Result<()> {
    env_logger::init().unwrap();
    let args = app_from_crate!()
        .args_from_usage("<wordlist> 'file containing words to look for, one per line'")
        .get_matches();

    let words = BufReader::new(File::open(args.value_of("wordlist").unwrap())?).lines()
        .collect::<io::Result<Vec<String>>>()?;

    // Note that word 0 is the unknown word.
    //let word_ids = HashMap::<&str, usize, FarmHasher>::default();
    // We are accepting the tiny probability that two strings hashes match.
    let word_ids: HashMap<&str, usize> = words.iter()
        .enumerate()
        .map(|(id, word)| (word.as_ref(), id + 1))
        .collect();

    let mut cooccurrences: Array2<usize> = Array2::zeros((words.len() + 1, words.len() + 1));

    info!("Collecting cooccurrences (with one another) of: {:?}", words);
    for (rec_i, rec) in WarcStreamer::new()?.enumerate() {
        if rec_i % 250000 == 0 {
            info!("Finished {}, this one is: {:?}",
                rec_i,
                &rec.chars().take(100).collect::<String>());
        }
        let mention_ids = tokenize(&rec, &word_ids);
        for mention_i in 0..mention_ids.len() {
            for context_i in mention_i..min(mention_ids.len(), WINDOW_WIDTH) {
                cooccurrences[[
                    mention_ids[mention_i], // row: center word
                    mention_ids[context_i] // column: context word
                ]] += 1; // uniform window weight
            }
        }
    }

    println!("Cooccurs look like {}", cooccurrences);
    Ok(())
}

/// Vanilla tokenization: split at everything not alphanumeric
pub fn tokenize(content: &str, ids: &HashMap<&str, usize>) -> Vec<usize> {
    /// Notice that we return indices (avoiding allocation)
    content
        //.split(|c: char| ! c.is_alphanumeric())
        .split_word_bounds()
        .map(|mention| *ids.get(mention).unwrap_or(&0))
        //.filter(|i| *i > 0)
        .collect()
}
