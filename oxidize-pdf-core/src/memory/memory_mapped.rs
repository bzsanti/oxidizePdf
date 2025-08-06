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
        let _bytes_read = reader.read(&mut buf).unwrap();
        assert_eq!(&buf, b"56");

        // Seek relative
        assert_eq!(reader.seek(SeekFrom::Current(-3)).unwrap(), 4);
        let _bytes_read = reader.read(&mut buf).unwrap();
        assert_eq!(&buf, b"45");

        // Seek from end
        assert_eq!(reader.seek(SeekFrom::End(-2)).unwrap(), 8);
        let _bytes_read = reader.read(&mut buf).unwrap();
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

    #[test]
    fn test_memory_mapped_file_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        // Don't write anything to create empty file

        let result = MemoryMappedFile::new(temp_file.path());

        // Should fail to map empty file
        assert!(result.is_err());
        match result {
            Err(PdfError::InvalidFormat(msg)) => {
                assert!(msg.contains("empty file"));
            }
            _ => panic!("Expected InvalidFormat error for empty file"),
        }
    }

    #[test]
    fn test_memory_mapped_file_nonexistent() {
        let nonexistent_path = "/path/that/does/not/exist.pdf";
        let result = MemoryMappedFile::new(nonexistent_path);

        assert!(result.is_err());
        match result {
            Err(PdfError::Io(_)) => {}
            _ => panic!("Expected IO error for nonexistent file"),
        }
    }

    #[test]
    fn test_memory_mapped_file_large() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let large_data = vec![0xAB; 10000];
        temp_file.write_all(&large_data).unwrap();
        temp_file.flush().unwrap();

        let mmap = MemoryMappedFile::new(temp_file.path()).unwrap();

        assert_eq!(mmap.len(), 10000);
        assert!(!mmap.is_empty());
        assert_eq!(mmap[0], 0xAB);
        assert_eq!(mmap[9999], 0xAB);
        assert_eq!(&mmap[..100], &large_data[..100]);
    }

    #[test]
    fn test_memory_mapped_file_deref() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Deref test data";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mmap = MemoryMappedFile::new(temp_file.path()).unwrap();

        // Test Deref implementation
        let slice: &[u8] = &mmap;
        assert_eq!(slice, test_data);

        // Test indexing (uses Deref)
        assert_eq!(mmap[0], b'D');
        assert_eq!(mmap[5], b' ');
        assert_eq!(mmap[test_data.len() - 1], b'a');

        // Test AsRef implementation
        let as_ref: &[u8] = mmap.as_ref();
        assert_eq!(as_ref, test_data);
    }

    #[test]
    fn test_memory_mapped_file_binary_data() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let binary_data = vec![0x00, 0xFF, 0x7F, 0x80, 0x01, 0xFE];
        temp_file.write_all(&binary_data).unwrap();
        temp_file.flush().unwrap();

        let mmap = MemoryMappedFile::new(temp_file.path()).unwrap();

        assert_eq!(mmap.len(), 6);
        assert_eq!(mmap[0], 0x00);
        assert_eq!(mmap[1], 0xFF);
        assert_eq!(mmap[2], 0x7F);
        assert_eq!(mmap[3], 0x80);
        assert_eq!(mmap[4], 0x01);
        assert_eq!(mmap[5], 0xFE);
    }

    #[test]
    fn test_memory_mapped_file_single_byte() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&[42]).unwrap();
        temp_file.flush().unwrap();

        let mmap = MemoryMappedFile::new(temp_file.path()).unwrap();

        assert_eq!(mmap.len(), 1);
        assert!(!mmap.is_empty());
        assert_eq!(mmap[0], 42);
    }

    #[test]
    fn test_mapped_reader_creation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Reader creation test";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let reader = MappedReader::new(temp_file.path()).unwrap();

        assert_eq!(reader.mmap.len(), test_data.len());
        assert_eq!(reader.position, 0);
    }

    #[test]
    fn test_mapped_reader_read_entire_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Complete file read test";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        let mut buf = vec![0u8; test_data.len()];
        let bytes_read = reader.read(&mut buf).unwrap();

        assert_eq!(bytes_read, test_data.len());
        assert_eq!(&buf, test_data);
        assert_eq!(reader.position, test_data.len());
    }

    #[test]
    fn test_mapped_reader_read_beyond_eof() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Short";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Read entire file
        let mut buf = vec![0u8; test_data.len()];
        assert_eq!(reader.read(&mut buf).unwrap(), test_data.len());

        // Try to read more - should return 0
        let mut buf = vec![0u8; 10];
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
        assert_eq!(reader.position, test_data.len());
    }

    #[test]
    fn test_mapped_reader_partial_reads() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"0123456789ABCDEF";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Read in chunks
        let mut buf = [0u8; 4];

        assert_eq!(reader.read(&mut buf).unwrap(), 4);
        assert_eq!(&buf, b"0123");
        assert_eq!(reader.position, 4);

        assert_eq!(reader.read(&mut buf).unwrap(), 4);
        assert_eq!(&buf, b"4567");
        assert_eq!(reader.position, 8);

        assert_eq!(reader.read(&mut buf).unwrap(), 4);
        assert_eq!(&buf, b"89AB");
        assert_eq!(reader.position, 12);

        assert_eq!(reader.read(&mut buf).unwrap(), 4);
        assert_eq!(&buf, b"CDEF");
        assert_eq!(reader.position, 16);

        // EOF
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn test_mapped_reader_seek_start() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Seek test data";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Seek to different positions from start
        assert_eq!(reader.seek(SeekFrom::Start(0)).unwrap(), 0);
        assert_eq!(reader.position, 0);

        assert_eq!(reader.seek(SeekFrom::Start(5)).unwrap(), 5);
        assert_eq!(reader.position, 5);

        assert_eq!(
            reader
                .seek(SeekFrom::Start(test_data.len() as u64))
                .unwrap(),
            test_data.len() as u64
        );
        assert_eq!(reader.position, test_data.len());
    }

    #[test]
    fn test_mapped_reader_seek_current() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Current seek test";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Move forward
        assert_eq!(reader.seek(SeekFrom::Current(5)).unwrap(), 5);
        assert_eq!(reader.position, 5);

        // Move forward more
        assert_eq!(reader.seek(SeekFrom::Current(3)).unwrap(), 8);
        assert_eq!(reader.position, 8);

        // Move backward
        assert_eq!(reader.seek(SeekFrom::Current(-2)).unwrap(), 6);
        assert_eq!(reader.position, 6);

        // Move to start
        assert_eq!(
            reader
                .seek(SeekFrom::Current(-(reader.position as i64)))
                .unwrap(),
            0
        );
        assert_eq!(reader.position, 0);
    }

    #[test]
    fn test_mapped_reader_seek_end() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"End seek test data";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Seek to end
        assert_eq!(
            reader.seek(SeekFrom::End(0)).unwrap(),
            test_data.len() as u64
        );
        assert_eq!(reader.position, test_data.len());

        // Seek backward from end
        assert_eq!(
            reader.seek(SeekFrom::End(-5)).unwrap(),
            (test_data.len() - 5) as u64
        );
        assert_eq!(reader.position, test_data.len() - 5);

        // Seek to specific position from end
        assert_eq!(
            reader
                .seek(SeekFrom::End(-(test_data.len() as i64)))
                .unwrap(),
            0
        );
        assert_eq!(reader.position, 0);
    }

    #[test]
    fn test_mapped_reader_seek_out_of_bounds() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Bounds test";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Negative position
        let result = reader.seek(SeekFrom::Start(u64::MAX));
        assert!(result.is_err());
        assert_eq!(reader.position, 0); // Position should remain unchanged

        // Beyond file end
        let result = reader.seek(SeekFrom::Start((test_data.len() + 1) as u64));
        assert!(result.is_err());
        assert_eq!(reader.position, 0);

        // Negative from current
        let result = reader.seek(SeekFrom::Current(-1));
        assert!(result.is_err());
        assert_eq!(reader.position, 0);

        // Too far from end
        let result = reader.seek(SeekFrom::End(-((test_data.len() + 1) as i64)));
        assert!(result.is_err());
        assert_eq!(reader.position, 0);
    }

    #[test]
    fn test_mapped_reader_seek_and_read_combination() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"0123456789";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Seek and read from middle
        reader.seek(SeekFrom::Start(3)).unwrap();
        let mut buf = [0u8; 3];
        let _bytes_read = reader.read(&mut buf).unwrap();
        assert_eq!(&buf, b"345");

        // Seek back and read
        reader.seek(SeekFrom::Start(1)).unwrap();
        let _bytes_read = reader.read(&mut buf).unwrap();
        assert_eq!(&buf, b"123");

        // Seek from current position
        reader.seek(SeekFrom::Current(2)).unwrap(); // Now at position 6
        let mut buf = [0u8; 2];
        let _bytes_read = reader.read(&mut buf).unwrap();
        assert_eq!(&buf, b"67");
    }

    #[test]
    fn test_get_slice_edge_cases() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Edge case testing";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let reader = MappedReader::new(temp_file.path()).unwrap();

        // Empty slice
        assert_eq!(reader.get_slice(5, 5), Some(&b""[..]));

        // Single byte
        assert_eq!(reader.get_slice(0, 1), Some(&b"E"[..]));
        assert_eq!(
            reader.get_slice(test_data.len() - 1, test_data.len()),
            Some(&b"g"[..])
        );

        // Full file
        assert_eq!(
            reader.get_slice(0, test_data.len()),
            Some(test_data.as_ref())
        );

        // At boundary
        assert_eq!(
            reader.get_slice(test_data.len(), test_data.len()),
            Some(&b""[..])
        );

        // Out of bounds scenarios
        assert_eq!(reader.get_slice(test_data.len(), test_data.len() + 1), None);
        assert_eq!(
            reader.get_slice(test_data.len() + 1, test_data.len() + 2),
            None
        );
        assert_eq!(reader.get_slice(10, 5), None); // start > end
    }

    #[test]
    fn test_mapped_reader_zero_length_read() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Zero length test";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Read zero bytes
        let mut buf = [];
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
        assert_eq!(reader.position, 0); // Position should not change
    }

    #[test]
    fn test_mapped_reader_large_buffer_read() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Small data";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        // Try to read more than available
        let mut buf = vec![0u8; 100];
        let bytes_read = reader.read(&mut buf).unwrap();

        assert_eq!(bytes_read, test_data.len());
        assert_eq!(&buf[..bytes_read], test_data);
        assert_eq!(reader.position, test_data.len());
    }

    #[test]
    fn test_memory_mapped_file_utf8_content() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = "Hello, 世界! 🦀".as_bytes();
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mmap = MemoryMappedFile::new(temp_file.path()).unwrap();

        assert_eq!(mmap.len(), test_data.len());
        assert_eq!(&mmap[..], test_data);

        // Verify UTF-8 content
        let content = String::from_utf8_lossy(&mmap);
        assert_eq!(content, "Hello, 世界! 🦀");
    }

    #[test]
    fn test_mapped_reader_error_propagation() {
        let nonexistent_path = "/definitely/does/not/exist/test.pdf";
        let result = MappedReader::new(nonexistent_path);

        assert!(result.is_err());
        match result {
            Err(PdfError::Io(_)) => {}
            _ => panic!("Expected IO error"),
        }
    }

    #[test]
    fn test_get_slice_exact_boundaries() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Boundary test";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let reader = MappedReader::new(temp_file.path()).unwrap();

        // Test exact boundaries
        let len = test_data.len();

        // First character
        assert_eq!(reader.get_slice(0, 1), Some(&b"B"[..]));

        // Last character
        assert_eq!(reader.get_slice(len - 1, len), Some(&b"t"[..]));

        // Whole string
        assert_eq!(reader.get_slice(0, len), Some(test_data.as_ref()));

        // Empty at end
        assert_eq!(reader.get_slice(len, len), Some(&b""[..]));

        // One past end should fail
        assert_eq!(reader.get_slice(len, len + 1), None);
        assert_eq!(reader.get_slice(len + 1, len + 1), None);
    }

    #[test]
    fn test_memory_mapped_file_clone_and_share() {
        use std::sync::Arc;
        use std::thread;

        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Shared mmap test data";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mmap = Arc::new(MemoryMappedFile::new(temp_file.path()).unwrap());
        let mmap_clone = mmap.clone();

        let handle = thread::spawn(move || {
            assert_eq!(mmap_clone.len(), test_data.len());
            assert_eq!(&mmap_clone[..5], b"Share");
        });

        handle.join().unwrap();

        // Original still works
        assert_eq!(&mmap[..], test_data);
    }

    #[test]
    fn test_mapped_reader_position_consistency() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Position consistency test";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = MappedReader::new(temp_file.path()).unwrap();

        assert_eq!(reader.position, 0);

        // Read some data
        let mut buf = [0u8; 8];
        let _bytes_read = reader.read(&mut buf).unwrap();
        assert_eq!(reader.position, 8);

        // Seek and verify position
        reader.seek(SeekFrom::Start(5)).unwrap();
        assert_eq!(reader.position, 5);

        // Read more and verify position updates
        let _bytes_read = reader.read(&mut buf).unwrap();
        assert_eq!(reader.position, 13);

        // Seek relative and verify
        reader.seek(SeekFrom::Current(-3)).unwrap();
        assert_eq!(reader.position, 10);
    }
}
