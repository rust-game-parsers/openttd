#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "null error: {}", reason)]
    NullError { reason: String },
    #[fail(display = "data parse error: {}", reason)]
    DataParseError { reason: String },
    #[fail(display = "network error: {}", reason)]
    NetworkError { reason: String },
    #[fail(display = "invalid packet: {}", reason)]
    InvalidPacketError { reason: String },
    #[fail(display = "IO error: {}", reason)]
    IOError { reason: String },
    #[fail(display = "pipe error: {}", reason)]
    PipeError { reason: String },
    #[fail(display = "operation timed out: {}", reason)]
    TimeoutError { reason: String },
}
