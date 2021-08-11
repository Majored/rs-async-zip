#[derive(Debug)]
pub enum ZIPError {
    ReadNumInvariantFailed,
    ReadFailed,
    LocalFileHeaderError,
}
