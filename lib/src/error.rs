use std::{num::ParseIntError, fmt, result, error};

#[derive(Debug)]
pub enum ParseRGBError {
    WrongLength(usize),
    ParseHex(ParseIntError),
}

impl fmt::Display for ParseRGBError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseRGBError::WrongLength(len) =>
                write!(f, "excluding pre/suffixes, \
                    string can only be of length 3 or 6 ({} given)", len),
            ParseRGBError::ParseHex(ref pie) =>
                write!(f, "{}", pie),
        }
    }
}

impl error::Error for ParseRGBError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            ParseRGBError::WrongLength(_) => None,
            ParseRGBError::ParseHex(ref pie) => Some(pie),
        }
    }
}

impl From<ParseIntError> for ParseRGBError {
    fn from(err: ParseIntError) -> ParseRGBError {
        ParseRGBError::ParseHex(err)
    }
}

/// A result of a function that may return a `Error`.
pub type USBResult<T> = result::Result<T, USBError>;

#[derive(Debug)]
pub enum USBError {
    NonCompatibleDevice,
    DeviceNotFound,
    /// (total, written) An incomplete write
    IncompleteWrite(usize, usize),
    /// (total, read) An incomplete read
    IncompleteRead(usize, usize),
    ResponseMismatch,
    DeviceBusy,
    CommandFailed,
    CommandNotSupported,
    CommandTimeout,
    ResponseUnknownStatus(u8),
    ResponseUnknownValue(u8),
    /// Wrapper for rusb::Error
    RUSBError(rusb::Error),
}

impl fmt::Display for USBError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            USBError::NonCompatibleDevice => write!(f, "device is incompatible"),
            USBError::DeviceNotFound => write!(f, "device not found"),
            USBError::IncompleteWrite(total, written) =>
                write!(f, "failed to write full control message \
                    (written {} out of {} bytes)", written, total),
            USBError::IncompleteRead(total, read) =>
                write!(f, "failed to read full control message \
                    (read {} out of {} bytes)", read, total),
            USBError::ResponseMismatch => write!(f, "wrong response type"),
            USBError::DeviceBusy => write!(f, "device is busy"),
            USBError::CommandFailed => write!(f, "command failed"),
            USBError::CommandNotSupported => write!(f, "command not supported"),
            USBError::CommandTimeout => write!(f, "command timed out"),
            USBError::ResponseUnknownStatus(status) => 
                write!(f, "unrecognized status in response: {:#02X}", status),
            USBError::ResponseUnknownValue(value) => 
                write!(f, "unrecognized value in response: {:#02X}", value),
            USBError::RUSBError(ref e) => write!(f, "{}", e),
        }
    }
}

impl error::Error for USBError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            USBError::RUSBError(ref e) => Some(e),
            _ => None
        }
    }
}

impl From<rusb::Error> for USBError {
    fn from(err: rusb::Error) -> USBError {
        USBError::RUSBError(err)
    }
}