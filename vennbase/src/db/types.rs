use std::ffi::OsStr;
use std::io;

#[derive(Debug)]
pub struct VennTimestamp(pub i64);

impl VennTimestamp {
    pub fn now() -> Self {
        VennTimestamp(chrono::Utc::now().timestamp_millis())
    }
}

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct MimeType(String);

#[derive(Debug)]
pub struct InvalidMimeType;

pub const MIN_MIMETYPE_LENGTH: usize = 3;
pub const MAX_MIMETYPE_LENGTH: usize = 255;
pub const VALID_MIMETYPE_CHARS: &str = "abcdefghijklmnopqrstuvwxyz0123456789-+/";

impl MimeType {
    pub fn from_base64_filename(path: &OsStr) -> io::Result<Self> {
        use base64::Engine;

        let filename = path.to_str().ok_or(
            io::Error::new(io::ErrorKind::InvalidData, "Invalid file name")
        )?.to_string();

        let decoded_mimetype = base64::engine::general_purpose::STANDARD_NO_PAD
            .decode(filename)
            .map(String::from_utf8)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(MimeType(decoded_mimetype))
    }

    pub fn from(mimetype: &str) -> Result<Self, InvalidMimeType> {
        let mimetype = mimetype.to_ascii_lowercase();

        if mimetype.len() < MIN_MIMETYPE_LENGTH || mimetype.len() > MAX_MIMETYPE_LENGTH {
            return Err(InvalidMimeType);
        }
        if mimetype.find('/').is_none() || mimetype.find('/') == mimetype.rfind('/') {
            return Err(InvalidMimeType);
        }
        if mimetype.chars().all(|c| VALID_MIMETYPE_CHARS.contains(c)) {
            Ok(MimeType(mimetype))
        }
        else {
            Err(InvalidMimeType)
        }
    }

    pub fn to_base64_pathname(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(&self.0)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

// We implemented the Debug trait ourselves so that it doesn't print an unnecessary line break
impl std::fmt::Debug for MimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("MimeType({})", self.0).as_str())
    }
}

impl std::fmt::Display for MimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl From<String> for MimeType {
    fn from(s: String) -> Self {
        MimeType(s)
    }
}

impl From<&str> for MimeType {
    fn from(value: &str) -> Self {
        MimeType(value.to_string())
    }
}
