use log::{error, info, warn};
use std::collections::HashMap;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::sync::{Mutex, OnceLock};
use crate::error::{Error, LogError};

/// A global (static) map that holds our file handles for different log categories.
/// We use `OnceLock` to ensure it's initialized only once.
/// `Mutex` ensures thread-safe access if multiple threads log concurrently.
static LOG_MAP: OnceLock<Mutex<HashMap<String, std::fs::File>>> = OnceLock::new();

/// Initializes logging for a given list of categories.
/// A file named `category.log` will be created (or appended to) in the `log/` directory.
pub fn init_logging(log_dir: &str, categories: &[&str]) -> Result<(), Error> {
    std::fs::create_dir_all(log_dir).map_err(|e| LogError::DirectoryCreationFailed(e.to_string()))?;

    let mut new_map = HashMap::new();
    for &category in categories {
        let path = format!("{}/{}.log", log_dir, category);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| LogError::FileOpenFailed(e.to_string()))?;

        // If file is non-empty, write a separator:
        let metadata = file.metadata().map_err(|e| LogError::MetadataFailed(e.to_string()))?;
        write_separator(path, &mut file, metadata)?;

        new_map.insert(category.to_string(), file);
    }

    match LOG_MAP.set(Mutex::new(new_map)) {
        // First time initialization succeeded:
        Ok(_) => {
            info!("init_logging: first-time initialization complete.");
            Ok(())
        }
        // LOG_MAP was already initialized, so let's merge in any new categories:
        Err(_) => {
            info!("init_logging: LOG_MAP was already initialized; merging categories.");
            if let Some(mutex_map) = LOG_MAP.get() {
                let mut global_map = mutex_map.lock().map_err(|e| LogError::MutexPoisoned(e.to_string()))?;

                for &category in categories {
                    if !global_map.contains_key(category) {
                        // If this category is truly new, open again just in case
                        let path = format!("{}/{}.log", log_dir, category);
                        let mut file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&path)
                            .map_err(|e| LogError::FileOpenFailed(e.to_string()))?;

                        let metadata = file.metadata().map_err(|e| LogError::MetadataFailed(e.to_string()))?;
                        write_separator(path, &mut file, metadata)?;

                        global_map.insert(category.to_string(), file);
                        info!(
                            "Added new category '{}' during re-initialization.",
                            category
                        );
                    }
                }
            }
            Ok(())
        }
    }
}

fn write_separator(_path: String, file: &mut File, metadata: Metadata) -> Result<(), Error> {
    if metadata.len() > 0 {
        file.seek(SeekFrom::End(0)).map_err(|e| LogError::WriteFailed(e.to_string()))?;
        let separator = "\n\n\n########## NEW SESSION ##########\n";
        file.write_all(separator.as_bytes()).map_err(|e| LogError::WriteFailed(e.to_string()))?;
    }
    Ok(())
}

/// Writes a single line (with a trailing newline) to the specified log category.
///
/// # Arguments
///x
/// * `category` - The name of the log category (e.g. "hint", "challenge").
/// * `message` - The content to be written to the log file.
pub fn log_message(category: &str, message: &str) -> Result<(), Error> {
    // Check if our global logging map is set up:
    if let Some(mutex_map) = LOG_MAP.get() {
        let mut map = mutex_map.lock().unwrap_or_else(|poisoned| {
            warn!("LOG_MAP mutex was poisoned. Logging might be compromised.");
            poisoned.into_inner()
        });

        // Fetch the file handle for the requested category:
        if let Some(file) = map.get_mut(category) {
            // Try writing to the file; log an error if something goes wrong.
            writeln!(file, "{}", message).map_err(|e| LogError::WriteFailed(e.to_string()))?;
            info!("{}: {}", category, message);
            Ok(())
        } else {
            // We have no file for this category (wasn't initialized).
            warn!(
                "No log file found for category '{}'. Did you call `init_logging` first?",
                category
            );
            Err(LogError::FileOpenFailed(format!("No log file found for category '{}'", category)).into())
        }
    } else {
        // LOG_MAP was never initialized (or we tried reading it too early).
        warn!("LOG_MAP not initialized. Call `init_logging` first.");
        Err(LogError::FileOpenFailed("LOG_MAP not initialized".to_string()).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;
    use std::path::Path;
    use tempfile::tempdir;

    /// Reads the entire contents of the file at `path` into a String.
    fn read_file_to_string<P: AsRef<Path>>(path: P) -> String {
        let mut file = File::open(path).unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_init_logging_creates_directory() -> Result<(), Error> {
        // Create a temporary directory.
        let temp_dir = tempdir().map_err(|e| LogError::DirectoryCreationFailed(e.to_string()))?;
        // Choose a subdirectory that does not yet exist.
        let log_dir = temp_dir.path().join("test");
        let log_dir_str = log_dir.to_str().unwrap();

        // The log directory should not exist before initialization.
        assert!(
            !log_dir.exists(),
            "Log directory already exists before init_logging"
        );

        // Calling init_logging should create the directory.
        init_logging(log_dir_str, &["test_cat"])?;

        // Now the directory should exist.
        assert!(
            log_dir.exists(),
            "Log directory was not created by init_logging"
        );
        Ok(())
    }

    #[test]
    fn test_init_logging_creates_files_for_categories() -> Result<(), Error> {
        let temp_dir = tempdir().map_err(|e| LogError::DirectoryCreationFailed(e.to_string()))?;
        let log_dir = temp_dir.path().join("test");
        let log_dir_str = log_dir.to_str().unwrap();

        // Initialize with multiple categories.
        let categories = ["hint", "challenge", "movement"];
        init_logging(log_dir_str, &categories)?;

        // Check that each corresponding file exists.
        for cat in &categories {
            let file_path = log_dir.join(format!("{}.log", cat));
            assert!(
                file_path.exists(),
                "File not created for category {}",
                cat
            );
        }
        Ok(())
    }

    #[test]
    fn test_init_logging_appends_separator_for_existing_file() -> Result<(), Error> {
        let temp_dir = tempdir().map_err(|e| LogError::DirectoryCreationFailed(e.to_string()))?;
        let log_dir = temp_dir.path().join("test");
        let log_dir_str = log_dir.to_str().unwrap();

        // Manually create the log directory and an "existing.log" file with content.
        fs::create_dir_all(&log_dir).map_err(|e| LogError::DirectoryCreationFailed(e.to_string()))?;
        let file_path = log_dir.join("existing.log");
        fs::write(&file_path, b"Existing content...").map_err(|e| LogError::WriteFailed(e.to_string()))?;

        // Initialize logging with the "existing" category.
        init_logging(log_dir_str, &["existing"])?;

        // Read the file contents.
        let contents = read_file_to_string(&file_path);
        // The original content should be present...
        assert!(
            contents.contains("Existing content..."),
            "Original content not found"
        );
        // ...and the separator should have been appended.
        assert!(
            contents.contains("########## NEW SESSION ##########"),
            "Separator not appended"
        );
        Ok(())
    }

    #[test]
    fn test_init_logging_doesnt_append_separator_for_new_file() -> Result<(), Error> {
        let temp_dir = tempdir().map_err(|e| LogError::DirectoryCreationFailed(e.to_string()))?;
        let log_dir = temp_dir.path().join("test");
        let log_dir_str = log_dir.to_str().unwrap();
        let file_path = log_dir.join("brand_new.log");

        // Since the file doesn't exist yet, init_logging will create it.
        init_logging(log_dir_str, &["brand_new"])?;

        // Read the contents of the new file.
        let contents = read_file_to_string(&file_path);
        // A brand-new file should not contain the separator.
        assert!(
            !contents.contains("NEW SESSION"),
            "Separator was unexpectedly added to a brand-new file"
        );
        Ok(())
    }

    #[test]
    fn test_init_logging_merges_new_categories() -> Result<(), Error> {
        let temp_dir = tempdir().map_err(|e| LogError::DirectoryCreationFailed(e.to_string()))?;
        let log_dir = temp_dir.path().join("test");
        let log_dir_str = log_dir.to_str().unwrap();

        // First initialization with categories "one" and "two".
        init_logging(log_dir_str, &["one", "two"])?;
        assert!(log_dir.join("one.log").exists());
        assert!(log_dir.join("two.log").exists());
        assert!(!log_dir.join("three.log").exists());

        // Second initialization with overlapping ("two") plus a new category ("three").
        init_logging(log_dir_str, &["two", "three"])?;
        // "three" should now be created.
        assert!(log_dir.join("three.log").exists());
        Ok(())
    }

    #[test]
    fn test_log_message_appends_text() -> Result<(), Error> {
        let temp_dir = tempdir().map_err(|e| LogError::DirectoryCreationFailed(e.to_string()))?;
        let log_dir = temp_dir.path().join("test");
        let log_dir_str = log_dir.to_str().unwrap();

        init_logging(log_dir_str, &["testcat"])?;
        let file_path = log_dir.join("testcat.log");

        // Write a log message.
        log_message("testcat", "Hello from testcat")?;
        let contents = read_file_to_string(&file_path);
        assert!(
            contents.contains("Hello from testcat"),
            "Expected text not found in log"
        );

        // Append another log message.
        log_message("testcat", "Second line")?;
        let contents2 = read_file_to_string(&file_path);
        assert!(
            contents2.contains("Second line"),
            "Second log did not append"
        );
        // Verify that the first message is still present.
        assert!(
            contents2.contains("Hello from testcat"),
            "First message missing"
        );
        Ok(())
    }
}

