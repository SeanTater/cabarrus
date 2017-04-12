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
    let outname = args.value_of("output").unwrap();
    let mut mats : Vec<&str> = args.values_of("addends").unwrap().collect();
    mats.sort();
    
    let size = value_t!(args, "size", usize).unwrap_or_else(|e| e.exit());
    let rank = value_t!(args, "rank", usize).unwrap_or_else(|e| e.exit());
    let chunksize = (mats.len() + size - 1) / size;
    mats = mats.into_iter().skip(rank*chunksize).take(chunksize).collect();
    info!("{} files, {} workers (I'm #{}), chunks of {} (I get {}), output goes to {}",
        mats.len(), size, rank, chunksize, mats.len(), outname);
    
    if ! mats.is_empty() {
        {
            let matname = mats[0];
            let ref matfile = cabarrus::numpy::open_matrix_mmap(matname)
                .expect(&format!("Failed to open first matrix, {}", matname));
            let ref mat = cabarrus::numpy::read_matrix_mmap(matfile)
                .unwrap();
            cabarrus::numpy::write_matrix(outname, mat)
                .expect("Failed to create accumulator matrix file");
        };
        let ref accumfile = cabarrus::numpy::open_matrix_mmap(outname)
            .expect("Failed to reopen accumulator matrix");
        let mut accum = cabarrus::numpy::read_matrix_mmap(accumfile)?;

        for matname in mats {
            let ref matfile = cabarrus::numpy::open_matrix_mmap(matname)
                .expect(&format!("Failed to open matrix {}", matname));
            let ref mat = cabarrus::numpy::read_matrix_mmap(matfile)?;

            info!("Reading {} ({} GB) ..", matname, mat.len() >> 27);
            accum += mat;
        }
    } else {
            info!("No matrices processed so nothing saved.");
        }
    Ok(())
}
