//! Defines function to manage files (write, delete old)

// Imports
use actix_multipart::Field;
use std::path::PathBuf;

use futures::StreamExt;
use std::fs;

// Enum
#[derive(Debug)]
pub enum WriteError {
    CannotCreateFile(String),
    CannotWriteFile(String),
    FileTooLarge(String),
}

/// Tries to write the given file after checking the max file size allowed
pub async fn write_file(filepath: PathBuf, field: Field) -> Result<(), WriteError> {
    // Create file
    let mut file = match std::fs::File::create(&filepath) {
        Ok(f) => f,
        Err(_) => return Err(WriteError::CannotCreateFile("Failed to create file".to_string())),
    };

    // Prepare to write
    let mut size: u64 = 0;
    let mut field_data = field;

    // Get max size from .env, and convert to bytes
    let max_file_size_gb: u64 = std::env::var("max_file_size_GB")
        .expect("'max_file_size_GB' env var not set")
        .parse()
        .expect("'max_file_size_GB' was not an u64");
    let max_file_size: u64 = max_file_size_gb * (1024 * 1024 * 1024);

    // Write
    while let Some(Ok(chunk)) = field_data.next().await {
        size += chunk.len() as u64;

        if size > max_file_size {
            let _ = fs::remove_file(&filepath);
            return Err(WriteError::FileTooLarge("File too large".to_string()));
        }

        use std::io::Write;
        if let Err(_) = file.write_all(&chunk) {
            let _ = fs::remove_file(&filepath);
            return Err(WriteError::CannotWriteFile("Failed to write file".to_string()));
        }
    }

    Ok(())
}

/// Removes files so that there is no more than `nb_to_keep` files in `folder_name`.
/// It removes first the oldest files (based on metadata modified date)
pub fn cleanup_old_files(nb_to_keep: usize, folder_name: &str) -> std::io::Result<()> {
    // Create a vector of (path, modif_time)
    let mut files: Vec<_> = fs::read_dir(folder_name)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() {
                let metadata = entry.metadata().ok()?;
                Some((path, metadata.modified().ok()?))
            } else {
                None
            }
        })
        .collect();

    // Sort by modification time, newest first
    files.sort_by(|a, b| b.1.cmp(&a.1));

    // Delete files beyond `nb_to_keep`
    for (path, _) in files.iter().skip(nb_to_keep) {
        fs::remove_file(path).ok();
        println!("Deleted old file: {:?}", path.file_name());
    }

    Ok(())
}


