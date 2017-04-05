//
// Errors
//
use std::io;
use std::result;
use std::error;
use std::num;
use ndarray_linalg;
use std::fmt;
use linxal::svd as lxsvd;
use ndarray as nd;

/// Type alias for Iredell errors
pub type Result<X> = result::Result<X, Error>;

/// Wrapper for many kinds of errors occuring as part of search
#[derive(Debug)]
pub enum Error {
    InvalidDimensions(String),
    SVDError(lxsvd::SVDError),
    LinalgError(ndarray_linalg::error::LinalgError),
    ShapeError(nd::ShapeError),
    IOError(io::Error),
    ParseFloatError(num::ParseFloatError),
    MissingFile(&'static str, Option<io::Error>),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidDimensions(ref info) => write!(f, "Dimension Mismatch: {}", info),
            Error::SVDError(ref err) => write!(f, "SVD Error error: {:?}", err),
            Error::LinalgError(ref err) => write!(f, "Linear Algebra error: {}", err),
            Error::ShapeError(ref err) => write!(f, "NDArray shape error: {:?}", err),
            Error::IOError(ref err) => write!(f, "IO error: {}", err),
            Error::ParseFloatError(ref err) => write!(f, "Error parsing float: {}", err),
            Error::MissingFile(ref info, ref opt_err) => {
                write!(f,
                    "The {} must already exist at this point but there was a problem opening it. \
                    Wrong directory? Maybe missed a step? The OS error was: ",
                    info)?;
                if let &Some(ref err) = opt_err { err.fmt(f) }
                else { write!(f, "Unknown") }
            },
            Error::Other(ref info) => write!(f, "{}", info),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::InvalidDimensions(_) => {
                "Can't match dimensions of two matrices for an operation"
            }
            Error::SVDError(_) => "Error computing dense LAPACK svd",
            Error::LinalgError(ref err) => err.description(),
            Error::ShapeError(ref err) => err.description(),
            Error::IOError(ref err) => err.description(),
            Error::ParseFloatError(ref err) => err.description(),
            Error::MissingFile(ref info, _) => info,
            Error::Other(ref info) => info,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::InvalidDimensions(_) => None,
            Error::SVDError(_) => None,
            Error::LinalgError(ref err) => Some(err),
            Error::ShapeError(ref err) => Some(err),
            Error::IOError(ref err) => Some(err),
            Error::ParseFloatError(ref err) => Some(err),
            Error::MissingFile(_, _) => None,
            Error::Other(_) => None,
        }
    }
}
//
// Convert everything else into Error
//
impl From<ndarray_linalg::error::LinalgError> for Error {
    fn from(err: ndarray_linalg::error::LinalgError) -> Self {
        Error::LinalgError(err)
    }
}
impl From<lxsvd::SVDError> for Error {
    fn from(err: lxsvd::SVDError) -> Self {
        Error::SVDError(err)
    }
}
impl From<nd::ShapeError> for Error {
    fn from(err: nd::ShapeError) -> Self {
        Error::ShapeError(err)
    }
}
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IOError(err)
    }
}
impl From<num::ParseFloatError> for Error {
    fn from(err: num::ParseFloatError) -> Self {
        Error::ParseFloatError(err)
    }
}

//
// Convert Error into a general io Error
//
impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        io::Error::new(io::ErrorKind::Other, err)
    }
}
