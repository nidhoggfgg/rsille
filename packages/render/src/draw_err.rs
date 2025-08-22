#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DrawErr;

impl From<DrawErr> for std::io::Error {
    fn from(value: DrawErr) -> Self {
        std::io::Error::other(value)
    }
}

impl std::fmt::Display for DrawErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("draw error")
    }
}

impl core::error::Error for DrawErr {}
