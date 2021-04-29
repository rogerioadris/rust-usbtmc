#[derive(Debug)]
pub enum UsbtmcError {
    Rusb(rusb::Error),
    BulkIn,
    BulkOut,
    Exception,
}

impl From<rusb::Error> for UsbtmcError {
    fn from(item: rusb::Error) -> Self {
        UsbtmcError::Rusb(item)
    }
}
