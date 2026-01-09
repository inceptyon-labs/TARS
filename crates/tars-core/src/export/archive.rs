//! Archive creation for plugin export

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

/// Errors during archive creation
#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Walk error: {0}")]
    Walk(#[from] walkdir::Error),
}

/// Create a zip archive from a directory
///
/// # Errors
/// Returns an error if archive creation fails
pub fn create_archive(source_dir: &Path, output_path: &Path) -> Result<(), ArchiveError> {
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(source_dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let relative_path = path
                .strip_prefix(source_dir)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            let name = relative_path.to_string_lossy();
            zip.start_file(name.to_string(), options)?;

            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        }
    }

    zip.finish()?;
    Ok(())
}
