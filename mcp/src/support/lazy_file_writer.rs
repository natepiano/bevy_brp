use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

        // Write to the file, recreate if write fails (file might be deleted)
        if let Some(Ok(bytes)) = file_guard.as_mut().map(|file| file.write(buf)) {
            drop(file_guard);
            Ok(bytes)
        } else {
            // Write failed, try recreating the file
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            let bytes = file.write(buf)?;
            *file_guard = Some(file);
            drop(file_guard);
            Ok(bytes)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut file_guard = self
            .file
            .lock()
            .map_err(|_| io::Error::other("Mutex poisoned"))?;
        
        // If file doesn't exist or flush fails, recreate the file handle
        match file_guard.as_mut().map(std::io::Write::flush) {
            Some(Err(_)) | None if self.path.exists() => {
                // File exists but flush failed, recreate handle
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.path)?;
                *file_guard = Some(file);
                drop(file_guard);
                Ok(())
            }
            _ => {
                drop(file_guard);
                Ok(()) // File doesn't exist or flush succeeded, nothing more to do
            }
        }
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
