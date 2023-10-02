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

impl MimeType {
    pub fn from_base64_filename(path: &OsStr) -> io::Result<Self> {
        use base64::Engine;

        let filename = path.to_str().ok_or(
            io::Error::new(io::ErrorKind::InvalidData, "Invalid file name")
        )?.to_string();

        let decoded_mimetype = base64::engine::general_purpose::STANDARD_NO_PAD.decode(filename)
            .map(String::from_utf8)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(MimeType(decoded_mimetype))
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

impl From<String> for MimeType {
    fn from(s: String) -> Self {
        MimeType(s)
    }
}
