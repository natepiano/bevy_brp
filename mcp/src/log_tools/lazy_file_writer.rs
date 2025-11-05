use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{self};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use tracing_subscriber::fmt::MakeWriter;

/// A lazy file writer that only creates the file on first write
pub struct LazyFileWriter {
    path: PathBuf,
    file: Arc<Mutex<Option<File>>>,
}

impl LazyFileWriter {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            file: Arc::new(Mutex::new(None)),
        }
    }
}

impl Clone for LazyFileWriter {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            file: Arc::clone(&self.file),
        }
    }
}

/// Writer instance that lazily creates the file on first write
pub struct LazyWriter {
    path: PathBuf,
    file: Arc<Mutex<Option<File>>>,
}

impl Write for LazyWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut file_guard = self
            .file
            .lock()
            .map_err(|_| io::Error::other("Mutex poisoned"))?;

        // Create file on first write or if file was deleted
        if file_guard.is_none() || !self.path.exists() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            *file_guard = Some(file);
        }

        // Try to write to the file
        match file_guard.as_mut() {
            Some(file) => {
                if let Ok(bytes) = file.write(buf) {
                    drop(file_guard);
                    Ok(bytes)
                } else {
                    // Write failed, file handle might be stale, recreate
                    let mut new_file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&self.path)?;
                    let bytes = new_file.write(buf)?;
                    *file_guard = Some(new_file);
                    drop(file_guard);
                    Ok(bytes)
                }
            },
            None => {
                // This should not happen due to the check above, but handle it
                Err(io::Error::other("File handle unexpectedly None"))
            },
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut file_guard = self
            .file
            .lock()
            .map_err(|_| io::Error::other("Mutex poisoned"))?;

        // Check if file was deleted
        if file_guard.is_some() && !self.path.exists() {
            // File was deleted, clear the stale handle
            *file_guard = None;
            drop(file_guard);
            return Ok(()); // Nothing to flush
        }

        // Try to flush
        if let Some(file) = file_guard.as_mut() {
            if matches!(file.flush(), Ok(())) {
                // Flush succeeded
            } else {
                // Flush failed, might be stale handle
                if self.path.exists() {
                    // File exists but handle is stale, recreate
                    let new_file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&self.path)?;
                    *file_guard = Some(new_file);
                } else {
                    // File doesn't exist, clear handle
                    *file_guard = None;
                }
            }
        }
        drop(file_guard);
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LazyFileWriter {
    type Writer = LazyWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LazyWriter {
            path: self.path.clone(),
            file: Arc::clone(&self.file),
        }
    }
}
