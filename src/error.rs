use std::fmt;

#[derive(Debug)]
pub enum JavixError {
    UnexpectedEof,
    InvalidMagic(u32),
    InvalidUtf8,
    UnknownConstantTag(u8),
    Io(std::io::Error),
}

impl fmt::Display for JavixError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            JavixError::UnexpectedEof => {
                write!(f, "unexpected end of file while reading class data")
            }
            JavixError::InvalidMagic(magic) => write!(
                f,
                "not a class file: expected magic 0xCAFEBABE, found {:#010X}",
                magic
            ),
            JavixError::InvalidUtf8 => write!(f, "invalid modified UTF-8 in constant pool entry"),
            JavixError::UnknownConstantTag(tag) => {
                write!(f, "unknown constant pool tag: {}", tag)
            }
            JavixError::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for JavixError {}

impl From<std::io::Error> for JavixError {
    fn from(err: std::io::Error) -> Self {
        JavixError::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, JavixError>;
