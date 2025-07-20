//! Memory-mapped file support for efficient large PDF handling
//!
//! Uses OS-level memory mapping to access PDF files without loading
//! them entirely into memory.

use crate::error::{PdfError, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

/// Platform-specific memory mapping implementation
#[cfg(unix)]
mod unix_mmap {
    use super::*;
    use std::os::unix::io::AsRawFd;
    use std::ptr;

    pub struct MmapInner {
        ptr: *mut u8,
        len: usize,
    }

    // SAFETY: MmapInner is used in a read-only context
    unsafe impl Send for MmapInner {}
    unsafe impl Sync for MmapInner {}

    impl MmapInner {
        pub fn new(file: &File, len: usize) -> Result<Self> {
            if len == 0 {
                return Err(PdfError::InvalidFormat(
                    "Cannot mmap empty file".to_string(),
                ));
            }

            unsafe {
                let ptr = libc::mmap(
                    ptr::null_mut(),
                    len,
                    libc::PROT_READ,
                    libc::MAP_PRIVATE,
                    file.as_raw_fd(),
                    0,
                );

                if ptr == libc::MAP_FAILED {
                    return Err(PdfError::Io(std::io::Error::last_os_error()));
                }

                Ok(Self {
                    ptr: ptr as *mut u8,
                    len,
                })
            }
        }

        pub fn as_slice(&self) -> &[u8] {
            unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
        }
    }

    impl Drop for MmapInner {
        fn drop(&mut self) {
            unsafe {
                libc::munmap(self.ptr as *mut libc::c_void, self.len);
            }
        }
    }
}

#[cfg(windows)]
mod windows_mmap {
    use super::*;
    use std::os::windows::io::AsRawHandle;
    use std::ptr;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::memoryapi::{
        CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ,
    };
    use winapi::um::winnt::PAGE_READONLY;

    pub struct MmapInner {
        ptr: *mut u8,
        len: usize,
        mapping_handle: *mut winapi::ctypes::c_void,
    }

    unsafe impl Send for MmapInner {}
    unsafe impl Sync for MmapInner {}

    impl MmapInner {
        pub fn new(file: &File, len: usize) -> Result<Self> {
            if len == 0 {
                return Err(PdfError::InvalidFormat(
                    "Cannot mmap empty file".to_string(),
                ));
            }

            unsafe {
                let mapping_handle = CreateFileMappingW(
                    file.as_raw_handle() as *mut _,
                    ptr::null_mut(),
                    PAGE_READONLY,
                    0,
                    0,
                    ptr::null(),
                );

                if mapping_handle.is_null() {
                    return Err(PdfError::Io(std::io::Error::last_os_error()));
                }

                let ptr = MapViewOfFile(mapping_handle, FILE_MAP_READ, 0, 0, len);

                if ptr.is_null() {
                    CloseHandle(mapping_handle);
                    return Err(PdfError::Io(std::io::Error::last_os_error()));
                }

                Ok(Self {
                    ptr: ptr as *mut u8,
                    len,
                    mapping_handle,
                })
            }
        }

        pub fn as_slice(&self) -> &[u8] {
            unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
        }
    }

    impl Drop for MmapInner {
        fn drop(&mut self) {
            unsafe {
                UnmapViewOfFile(self.ptr as *mut _);
                CloseHandle(self.mapping_handle);
            }
        }
    }
}

// Fallback implementation for unsupported platforms
#[cfg(not(any(unix, windows)))]
mod fallback_mmap {
    use super::*;

    pub struct MmapInner {
        data: Vec<u8>,
    }

    impl MmapInner {
        pub fn new(file: &File, len: usize) -> Result<Self> {
            let mut data = vec![0u8; len];
            let mut file_clone = file.try_clone()?;
            file_clone.seek(SeekFrom::Start(0))?;
            file_clone.read_exact(&mut data)?;
            Ok(Self { data })
        }

        pub fn as_slice(&self) -> &[u8] {
            &self.data
        }
    }
}

#[cfg(not(any(unix, windows)))]
use fallback_mmap::MmapInner;
#[cfg(unix)]
use unix_mmap::MmapInner;
#[cfg(windows)]
use windows_mmap::MmapInner;

/// Memory-mapped file for efficient access
pub struct MemoryMappedFile {
    inner: Arc<MmapInner>,
}

impl MemoryMappedFile {
    /// Create a new memory-mapped file
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let len = metadata.len() as usize;

        let inner = MmapInner::new(&file, len)?;

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Get the length of the mapped region
    pub fn len(&self) -> usize {
        self.inner.as_slice().len()
    }

    /// Check if the mapped region is empty
    pub fn is_empty(&self) -> bool {
        self.inner.as_slice().is_empty()
    }
}

impl Deref for MemoryMappedFile {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.inner.as_slice()
    }
}

impl AsRef<[u8]> for MemoryMappedFile {
    fn as_ref(&self) -> &[u8] {
        self.inner.as_slice()
    }
}

/// A reader that uses memory-mapped files
pub struct MappedReader {
    mmap: MemoryMappedFile,
    position: usize,
}

impl MappedReader {
    /// Create a new mapped reader
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mmap = MemoryMappedFile::new(path)?;
        Ok(Self { mmap, position: 0 })
    }

    /// Get a slice of the file at the given range
    pub fn get_slice(&self, start: usize, end: usize) -> Option<&[u8]> {
        let data = self.mmap.as_ref();
        if start <= end && end <= data.len() {
            Some(&data[start..end])
        } else {
            None
        }
    }
}

impl Read for MappedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let data = self.mmap.as_ref();
        let remaining = data.len().saturating_sub(self.position);
        let to_read = buf.len().min(remaining);

        if to_read > 0 {
            buf[..to_read].copy_from_slice(&data[self.position..self.position + to_read]);
            self.position += to_read;
        }

        Ok(to_read)
    }
}

impl Seek for MappedReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::End(n) => self.mmap.len() as i64 + n,
            SeekFrom::Current(n) => self.position as i64 + n,
        };

        if new_pos < 0 || new_pos > self.mmap.len() as i64 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Seek position out of bounds",
            ));
        }

        self.position = new_pos as usize;
        Ok(self.position as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_memory_mapped_file() {
        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, memory mapped world!";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        // Memory map it
        let mmap = MemoryMappedFile::new(temp_file.path()).unwrap();

        assert_eq!(mmap.len(), test_data.len());
        assert!(!mmap.is_empty());
        assert_eq!(&mmap[..], test_data);
    }

    #[test]
    fn test_mapped_reader_read() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Test data for mapped reader";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Read partial data
        let mut buf = [0u8; 4];
        assert_eq!(reader.read(&mut buf).unwrap(), 4);
        assert_eq!(&buf, b"Test");

        // Read more data
        let mut buf = [0u8; 5];
        assert_eq!(reader.read(&mut buf).unwrap(), 5);
        assert_eq!(&buf, b" data");
    }

    #[test]
    fn test_mapped_reader_seek() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"0123456789";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Seek to position 5
        assert_eq!(reader.seek(SeekFrom::Start(5)).unwrap(), 5);
        let mut buf = [0u8; 2];
        reader.read(&mut buf).unwrap();
        assert_eq!(&buf, b"56");

        // Seek relative
        assert_eq!(reader.seek(SeekFrom::Current(-3)).unwrap(), 4);
        reader.read(&mut buf).unwrap();
        assert_eq!(&buf, b"45");

        // Seek from end
        assert_eq!(reader.seek(SeekFrom::End(-2)).unwrap(), 8);
        reader.read(&mut buf).unwrap();
        assert_eq!(&buf, b"89");
    }

    #[test]
    fn test_get_slice() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World!";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let reader = MappedReader::new(temp_file.path()).unwrap();

        assert_eq!(reader.get_slice(0, 5), Some(&b"Hello"[..]));
        assert_eq!(reader.get_slice(7, 12), Some(&b"World"[..]));
        assert_eq!(
            reader.get_slice(0, test_data.len()),
            Some(test_data.as_ref())
        );

        // Out of bounds
        assert_eq!(reader.get_slice(10, 20), None);
        assert_eq!(reader.get_slice(5, 3), None); // start > end
    }
}
