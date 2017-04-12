/// MPI-based method for summing many arrays

// Argument parsing
// logging
#[macro_use] extern crate log;
#[macro_use] extern crate clap;
extern crate ndarray;
extern crate env_logger;
// lastly, this library
extern crate cabarrus;
use cabarrus::errors::*;

pub fn main() {
    // Main can't return a Result, and the ? operator needs the enclosing function to return Result
    inner_main().expect("Could not recover. Exiting.");
}
pub fn inner_main() -> Result<()> {
    env_logger::init().unwrap();
    let args = app_from_crate!()
        .arg_from_usage("<size> 'how many workers there are'")
        .arg_from_usage("<rank> 'which worker am I, starting from 0'")
        .arg_from_usage("<addends>.. 'files containing matrices to add'")
        .arg_from_usage("<output> 'file in which to store the resulting matrix'")
        .get_matches();
    
    let mut mats : Vec<&str> = args.values_of("addends").unwrap().collect();
    mats.sort();
    
    let size = value_t!(args, "size", usize).unwrap_or_else(|e| e.exit());
    let rank = value_t!(args, "rank", usize).unwrap_or_else(|e| e.exit());
    let chunksize = (mats.len() + size - 1) / size;
    info!("{} files, {} workers (I'm #{}), chunks of {}", mats.len(), size, rank, chunksize);
    
    let mut accum = None;
    for matname in mats.into_iter().skip(rank*chunksize).take(chunksize) {
        let ref matfile = cabarrus::numpy::open_matrix_mmap(matname)?;
        let ref mat = cabarrus::numpy::read_matrix_mmap(matfile)?;
        
        info!("Reading {} ({} GB) ..", matname, matname.len() >> 27);
        accum = Some(match accum {
            Some(mut acc) => {acc += mat; acc}
            None => mat.to_owned()
        });
    }
    if let Some(ref acc) = accum {
        cabarrus::numpy::write_matrix(args.value_of("output").unwrap(), acc)?;
    } else {
        info!("No matrices processed so nothing saved.");
    }
    Ok(())
}
