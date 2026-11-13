use super::{
    parallel_generation::{PageSpec, ParallelGenerationOptions, ParallelPageGenerator},
    HighPerformanceDocument, Operation,
};
use crate::{
    writer::{PdfWriter, WriterConfig},
    Document, Page, PdfError, Result,
};
use std::{
    fs,
    io::{BufWriter, Write},
    path::Path,
};

impl HighPerformanceDocument {
    /// Register an owned embedded font for pages using Font::Custom(name).
    /// Invalid font data is rejected before changing the registration.
    pub fn add_font_from_bytes(&mut self, name: impl Into<String>, data: Vec<u8>) -> Result<()> {
        let name = name.into();
        let mut validation = Document::new();
        validation.add_font_from_bytes(name.clone(), data.clone())?;
        self.font_data.insert(name, data);
        Ok(())
    }

    /// Append an owned core page, preserving its content and resources.
    pub fn add_pdf_page(&mut self, page: Page) -> Result<()> {
        super::validate_page(&page)?;
        self.pages.push(page);
        Ok(())
    }

    /// Build and append pages in input order. On any callback failure no pages
    /// are appended; already running callbacks may finish and their side effects
    /// are not rolled back. Callbacks must be thread-safe and must not reenter this document.
    pub fn generate_pages<F>(&mut self, specs: Vec<PageSpec>, render: F) -> Result<()>
    where
        F: Fn(&PageSpec) -> Result<Page> + Send + Sync,
    {
        let options = ParallelGenerationOptions {
            max_threads: self.options.max_threads,
            max_memory_per_thread: 0,
            ..Default::default()
        };
        let generator = if self.options.parallel_generation {
            ParallelPageGenerator::new(options)?
        } else {
            ParallelPageGenerator::new_sequential(options)?
        };
        let pages = generator.generate_pdf_pages(specs, render)?;
        self.pages.extend(pages);
        Ok(())
    }

    /// Publish a complete PDF. Generation/write/flush failures preserve an
    /// existing destination. An existing symlink targets its resolved file;
    /// dangling links and non-regular destinations are rejected.
    /// Source pages are retained, allowing repeated saves.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let destination = match fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() => fs::canonicalize(path)?,
            Ok(_) => path.to_path_buf(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => path.to_path_buf(),
            Err(error) => return Err(error.into()),
        };
        let permissions = match fs::metadata(&destination) {
            Ok(meta) if meta.is_file() => Some(meta.permissions()),
            Ok(_) => {
                return Err(PdfError::InvalidOperation(
                    "PDF destination must be a regular file".into(),
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        let token = self.metrics.start_operation(Operation::PdfGeneration);
        let result = self.write_pdf(temporary.as_file_mut());
        self.metrics.end_operation(token);
        result?;
        if let Some(permissions) = permissions {
            temporary.as_file().set_permissions(permissions)?;
        }
        temporary.as_file().sync_all()?;
        temporary
            .persist(&destination)
            .map_err(|error| PdfError::Io(error.error))?;
        Ok(())
    }

    fn write_pdf<W: Write>(&self, output: W) -> Result<()> {
        let mut document = Document::new();
        for (name, data) in &self.font_data {
            document.add_font_from_bytes(name.clone(), data.clone())?;
        }
        for page in &self.pages {
            document.add_page(page.clone());
        }
        let config = WriterConfig {
            compress_streams: self.options.intelligent_compression,
            ..Default::default()
        };
        let mut output = BufWriter::with_capacity(self.options.stream_buffer_size, output);
        if self.options.streaming_writer {
            PdfWriter::with_config(&mut output, config).write_document(&mut document)?;
        } else {
            let mut bytes = self.memory_pool.get_buffer(0)?;
            PdfWriter::with_config(&mut *bytes, config).write_document(&mut document)?;
            output.write_all(&bytes)?;
        }
        // The shared writer currently ignores its final flush error. Check the
        // borrowed buffer explicitly before publication, including drop-time I/O.
        output.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FlushFailure;
    impl Write for FlushFailure {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("flush failure"))
        }
    }
    #[test]
    fn final_flush_failure_is_not_reported_as_success() {
        for streaming_writer in [false, true] {
            let doc = HighPerformanceDocument::new(super::super::PerformanceOptions {
                streaming_writer,
                ..Default::default()
            })
            .unwrap();
            assert!(doc
                .write_pdf(FlushFailure)
                .unwrap_err()
                .to_string()
                .contains("flush failure"));
        }
    }
}
