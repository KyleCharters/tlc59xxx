use std::error::Error as STDError;
use std::fmt;
use std::result::Result as STDResult;

pub type Result<T> = STDResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    Lat,
    Spi,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Lat => write!(f, "Latch Write Error"),
            Error::Spi => write!(f, "SPI Write Error"),
        }
    }
}

impl STDError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Lat => "Error writing to latch.",
            Error::Spi => "Error writing to SPI.",
        }
    }

    fn cause(&self) -> Option<&dyn STDError> {
        None
    }
}
