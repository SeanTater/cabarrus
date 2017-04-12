//! Read and write NDArrays as Numpy arrays

use ndarray::prelude::*;
use ndarray as nd;
use std::fs::File;
use std::path::Path;
use std::ptr;
use std::str;
use std::io::{Read, Write};
use regex::bytes::Regex;
use byteorder::{LittleEndian, BigEndian, ReadBytesExt, WriteBytesExt};
use errors::*;
use memmap::{Mmap, Protection};

/// Write an array as a numpy array, fast but only works for native byte order
pub fn write_matrix<S, P>(path: P, arr: &ArrayBase<S, Ix2>) -> Result<()>
    where S: nd::Data<Elem=f64>, P: AsRef<Path> {
    let header_nospace = format!("{{'descr': '<f8', 'fortran_order': False, 'shape': ({},{})}}",
        arr.shape()[0], arr.shape()[1]);
    let virtual_len =
        // Calculating how many bytes we have in the header, so we can get alignment
        header_nospace.len()
        + 6 // The magic string
        + 2 // The version number
        + 2 // An unsigned 2-byte integer for header length
        + 1 ; // Because there will be a \n added
    let padding_needed = (((virtual_len + 15) / 16) * 16) - virtual_len; // to get to the next 16
    
    // Numpy version 1.0
    let mut writer = File::create(path)?;
    writer.write_all(b"\x93NUMPY\x01\x00")?;
    writer.write_u16::<LittleEndian>((header_nospace.len()
        + padding_needed
        + 1) // newline
        // Magic string, version number and 2 bytes for the header length number not included.
        as u16
        )?;
    write!(writer, "{}{}\n", header_nospace, " ".repeat(padding_needed))?;
    unsafe {
        // O(N) array copy without memory overhead.
        assert!(arr.is_standard_layout(), "To write an array it needs to be in C order (for now).");
        // Treat the incoming data as floating point
        writer.write_all(::std::slice::from_raw_parts(arr.as_ptr() as *const u8, arr.len()*8))?;
    };
    Ok(())
}

/// Read a Numpy matrix into memory. Be careful if it's large. You could run out of memory.
///
/// You need to know the number of dimensions at compile time so for convenience, we assume you
/// need a matrix. Also, this method is fast but only works for native byte order.
pub fn read_matrix<P: AsRef<Path>>(path: P) -> Result<Array2<f64>> {
    let header_match = Regex::new(r"NUMPY\x01\x00(?s:..)\{'descr': ?'<f8', ?'fortran_order': ?False, ?'shape': ?\((\d+), ?(\d+)\)\} *\n").unwrap();
    let mut reader = File::open(path.as_ref())?;
    let mut content = vec![];
    reader.read_to_end(&mut content)?;
    // The skip and the nested context here is so that we can regex parse (with a borrow)
    // and then reuse the buffer as the array. This way there are never two copies in memory.
    let res: Result<(usize, usize, usize)> = {
        let captures = header_match.captures(&content)
            .ok_or(helpful_complaint(path.as_ref(), &content))?;
        Ok((// where the full match ends
            captures.get(0).unwrap().end(),
            // The shape of the array as described in the metadata
            str::from_utf8(&captures[1]).unwrap().parse().unwrap(),
            str::from_utf8(&captures[2]).unwrap().parse().unwrap()
        ))
    };
    let (skip, h, w) = res?;
    let content_f64 = unsafe {
        // O(N) array copy without memory overhead. 
        let length = content.len() - skip;
        assert_eq!(length, h*w*8,
            "The numpy file's array is the wrong length for a {}x{} array. \
            It should be {} elements, ({} bytes), but it is actually {} bytes.",
            h, w, h*w, h*w*8, length
        );
        ptr::copy((&content[skip..]).as_ptr(),
                  (&mut content[..length]).as_mut_ptr(),
                  length);
        content.set_len(length);
        // Treat the incoming data as floating point
        let mut out : Vec<f64> = ::std::mem::transmute(content);
        out.set_len(length/8);
        out
    };
    Ok(ArrayBase::from_shape_vec([h, w], content_f64)?)
}

pub struct MatFile(usize, usize, Mmap);

/// Load a Numpy matrix as an mmap. This only consumes address space. (Part 1)
///
/// This is a two-step process because the Mmap needs to outlive the matrix.
pub fn open_matrix_mmap<P: AsRef<Path>>(path: P) -> Result<MatFile> {
    let header_match = Regex::new(r"NUMPY\x01\x00(?s:..)\{'descr': ?'<f8', ?'fortran_order': ?False, ?'shape': ?\((\d+), ?(\d+)\)\} *\n").unwrap();
    let mut reader = File::open(path.as_ref())?;
    let mut content = [0u8; 128];
    let bytes_read = reader.read(&mut content)?;
    assert!(bytes_read > 0, format!("The numpy file {} seems to be empty.", path.as_ref().display())); 
    // The skip and the nested context here is so that we can regex parse (with a borrow)
    // and then reuse the buffer as the array. This way there are never two copies in memory.
    let captures = header_match.captures(&content)
        .ok_or(helpful_complaint(path.as_ref(), &content))?;
    // where the full match ends
    let skip = captures.get(0).unwrap().end();
    // The shape of the array as described in the metadata
    let h = str::from_utf8(&captures[1]).unwrap().parse().unwrap();
    let w = str::from_utf8(&captures[2]).unwrap().parse().unwrap();
    
    Ok(MatFile(h, w, Mmap::open_with_offset(&reader, Protection::ReadWrite, skip, h*w*8)?))
}

/// Load a Numpy matrix as an mmap (Part 2)
///
/// You need to know the number of dimensions at compile time so for convenience, we assume you
/// need a matrix. Also, this method is even faster than read_matrix but only works for native byte
/// order.
///
/// This is different than you might expect. The file you read from must outlive the array,
/// because the array is based on an mmap of that file (so that it doesn't read it into memory).
pub fn read_matrix_mmap<'t>(mmap: &'t MatFile) -> Result<ArrayViewMut2<'t, f64>> {
    unsafe {
        let new_slice= ::std::slice::from_raw_parts_mut(mmap.2.ptr() as *mut f64, mmap.2.len()/8);
        Ok(ArrayViewMut2::from_shape([mmap.0, mmap.1], new_slice)?)
    }
}

/// Tell the user more info about the file
///
/// It seems verbose but you can see this error often so it save you time.
fn helpful_complaint(p: &Path, header: &[u8]) -> Error {
    let cap = ::std::cmp::min(header.len(), 100);
    let complaint = format!(
        "Expected {} to be an uncompressed numpy (.npy) file, but couldn't \
        parse the header. The first hundred bytes look like:
        
        {}
        
        
        As bytes, the header is as follows:
        
        {:?}
        
        
        It should look something like this example, where . are non-printable characters: \
        NUMPY..{{'descr': '<f8', 'fortran_order': False, 'shape': (34, 27)}}\
        Note: Cabarrus only supports 2D big-endian 64-bit float matrices in C order (for \
        simplicity). You may need to change the dtype accordingly.",
        p.display(),
        String::from_utf8_lossy(&header[..cap]),
        &header[..cap]);
    Error::Other(complaint)
}