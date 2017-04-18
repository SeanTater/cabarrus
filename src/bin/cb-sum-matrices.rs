/// MPI-based method for summing many arrays

// Argument parsing
// logging
#[macro_use] extern crate log;
#[macro_use] extern crate clap;
#[macro_use] extern crate ndarray;
extern crate env_logger;
extern crate rayon;
use rayon::prelude::*;
// lastly, this library
extern crate cabarrus;
use cabarrus::errors::*;
use ndarray::prelude::*;

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
        let accumfile = {
            let matname = mats[0];
            let ref matfile = cabarrus::numpy::open_matrix_mmap(matname)
                .expect(&format!("Failed to open first matrix, {}", matname));
            let ref mat = cabarrus::numpy::read_matrix_mmap(matfile)
                .unwrap();
            cabarrus::numpy::create_empty_mmap(outname, mat.shape())
                .expect("Failed to create accumulator matrix file")
        };
        let mut accum = cabarrus::numpy::read_matrix_mmap(&accumfile)?;

        // This is awkward because the matrices are too large for memory..
        // But if accum is an mmap, you waste a *ton* of IO otherwise.
        // Get chunks of about 8 MB.
        let width = accum.len_of(Axis(1));
        let height = accum.len_of(Axis(0));
        let capacity = std::cmp::max(1, (1 << 20) / width);
        let new_bufchunk = || Array2::zeros([capacity, width]);
        let mut bufchunk = new_bufchunk();
        // 1024 files at a time
        for matnames in mats.chunks(1024) {
            info!("Working on {:?}", matnames);
            let matfiles : Vec<cabarrus::numpy::MatFile> =
                matnames.iter()
                .map(|name| cabarrus::numpy::open_matrix_mmap(name)
                    .expect(&format!("Failed to open matrix {}", name)))
                .collect();
            let mats : Vec<ArrayViewMut2<f64>> = matfiles.iter()
                .map(|matfile| cabarrus::numpy::read_matrix_mmap(matfile)
                    .expect("Failed to read matrix"))
                .collect();
            
            // The overhead just doesn't matter when the IO is the limit
            let mut row_i = 0 as isize;
            while row_i < accum.len_of(Axis(0)) as isize {
                let fill = std::cmp::min(capacity as isize, accum.len_of(Axis(0)) as isize - row_i);
                info!("Starting chunk at row {} / {} ({}%) [POW: {}]",
                    row_i,
                    height,
                    100.0 * row_i as f64 / height as f64,
                    
                
                //bufchunk *= 0.0;
                
//                let mut buf = bufchunk.slice_mut(s![..fill, ..]);
//                for mat in mats.iter() {
//                    buf += &mat.slice(s![row_i..row_i+fill, ..]);
//                }
//                let mut accum_chunk = accum.slice_mut(s![row_i..row_i+fill, ..]);
//                accum_chunk += &buf;
                    
                    
                let buf = mats.par_iter()
                    .map(|mat| mat.slice(s![row_i..row_i+fill, ..]).to_owned())
                    .reduce(|| Array2::zeros([capacity, fill]), |lmat, rmat| {lmat += &rmat; lmat});
                let mut accum_chunk = accum.slice_mut(s![row_i..row_i+fill, ..]);
                accum_chunk += &buf;
                
                row_i += fill;
            }
        }
    } else {
        info!("No matrices processed so nothing saved.");
    }
    Ok(())
}
