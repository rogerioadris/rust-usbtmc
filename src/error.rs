use std::fmt;

#[derive(Debug)]
pub enum UsbtmcError {
    Rusb(rusb::Error),
    Utf8(std::str::Utf8Error),
    BulkIn,
    BulkOut,
    Exception,
}

impl From<rusb::Error> for UsbtmcError {
    fn from(item: rusb::Error) -> Self {
        UsbtmcError::Rusb(item)
    }
}

impl From<std::str::Utf8Error> for UsbtmcError {
    fn from(item: std::str::Utf8Error) -> Self {
        UsbtmcError::Utf8(item)
    }
}

impl fmt::Display for UsbtmcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbtmcError::Rusb(e) => write!(f, "USB error: {}", e),
            UsbtmcError::Utf8(e) => write!(f, "invalid UTF-8 in instrument response: {}", e),
            UsbtmcError::BulkIn => write!(f, "no bulk IN endpoint found on device"),
            UsbtmcError::BulkOut => write!(f, "no bulk OUT endpoint found on device"),
            UsbtmcError::Exception => write!(f, "instrument not found or could not be opened"),
        }
    }
}

impl std::error::Error for UsbtmcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UsbtmcError::Rusb(e) => Some(e),
            UsbtmcError::Utf8(e) => Some(e),
            _ => None,
        }
    }
}
