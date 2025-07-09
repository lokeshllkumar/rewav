use infer::Type;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

use crate::errors::TranscoderError;

/// reads beginning of file to determine type
pub fn infer_file_type(path: &Path) -> Result<Option<Type>, TranscoderError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();

    reader.take(4096).read_to_end(&mut buffer)?;

    Ok(infer::get(&buffer))
}

/// extracts the file extension from a path as a lowercase string
pub fn get_file_extension(path: &Path) -> Result<String, TranscoderError> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_ascii_lowercase())
        .ok_or_else(|| TranscoderError::Path(format!("File path has no extension: {:?}", path))) 
}