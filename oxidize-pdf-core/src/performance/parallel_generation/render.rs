use super::{PageSpec, ParallelPageGenerator, ThreadPriority};
use crate::{Page, PdfError, Result};
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use std::time::{Duration, Instant};

struct RenderedChunk {
    pages: Vec<Page>,
    page_time: Duration,
    elapsed: Duration,
    thread_id: usize,
}

impl ParallelPageGenerator {
    /// Render owned core pages through a caller callback, preserving input order.
    /// Uses this generator's bounded Rayon pool when available, otherwise serial
    /// execution. Nonzero callback memory limits, non-normal OS priorities and
    /// disabling load balancing are unsupported and rejected before callbacks.
    /// A failed call returns no pages; callbacks already running may finish.
    /// Statistics describe successful calls only; failed attempts leave them unchanged.
    pub fn generate_pdf_pages<F>(&self, specs: Vec<PageSpec>, render: F) -> Result<Vec<Page>>
    where
        F: Fn(&PageSpec) -> Result<Page> + Send + Sync,
    {
        self.render_pages(specs, render, true)
    }

    /// Execute the same callback contract serially, even in a Rayon build.
    pub fn generate_pdf_pages_sequential<F>(
        &self,
        specs: Vec<PageSpec>,
        render: F,
    ) -> Result<Vec<Page>>
    where
        F: Fn(&PageSpec) -> Result<Page> + Send + Sync,
    {
        self.render_pages(specs, render, false)
    }

    fn render_pages<F>(&self, specs: Vec<PageSpec>, render: F, parallel: bool) -> Result<Vec<Page>>
    where
        F: Fn(&PageSpec) -> Result<Page> + Send + Sync,
    {
        if self.options.max_memory_per_thread != 0
            || self.options.thread_pool_config.thread_priority != ThreadPriority::Normal
            || !self.options.load_balancing
        {
            return Err(PdfError::InvalidOperation("Callback generation requires memory limit 0, Normal priority and load balancing; OS memory/priority controls are unsupported".into()));
        }
        for spec in &specs {
            if !spec.width.is_finite()
                || !spec.height.is_finite()
                || spec.width <= 0.0
                || spec.height <= 0.0
            {
                return Err(PdfError::InvalidOperation(
                    "Page specification dimensions must be finite and positive".into(),
                ));
            }
        }
        let started = Instant::now();
        #[cfg(feature = "rayon")]
        let use_parallel = parallel && self.thread_pool.is_some();
        #[cfg(not(feature = "rayon"))]
        let use_parallel = {
            let _ = parallel;
            false
        };
        let generate_chunk = |chunk: &[PageSpec]| -> Result<RenderedChunk> {
            let chunk_start = Instant::now();
            #[cfg(feature = "rayon")]
            let thread_id = if use_parallel {
                rayon::current_thread_index().unwrap_or(0)
            } else {
                0
            };
            #[cfg(not(feature = "rayon"))]
            let thread_id = 0;
            let mut pages = Vec::with_capacity(chunk.len());
            let mut page_time = Duration::ZERO;
            for spec in chunk {
                let page_start = Instant::now();
                let page = render(spec)?;
                crate::performance::validate_page(&page)?;
                page_time += page_start.elapsed();
                pages.push(page);
            }
            Ok(RenderedChunk {
                pages,
                page_time,
                elapsed: chunk_start.elapsed(),
                thread_id,
            })
        };
        #[cfg(feature = "rayon")]
        let result: Result<Vec<RenderedChunk>> = if use_parallel {
            self.thread_pool
                .as_ref()
                .expect("checked pool")
                .install(|| {
                    specs
                        .par_chunks(self.options.chunk_size)
                        .map(generate_chunk)
                        .collect()
                })
        } else {
            specs
                .chunks(self.options.chunk_size)
                .map(generate_chunk)
                .collect()
        };
        #[cfg(not(feature = "rayon"))]
        let result: Result<Vec<RenderedChunk>> = specs
            .chunks(self.options.chunk_size)
            .map(generate_chunk)
            .collect();
        let chunks = result?;
        let mut stats = self
            .stats
            .lock()
            .map_err(|_| PdfError::Internal("Generation statistics lock poisoned".into()))?;
        stats.total_processing_time = started.elapsed();
        if use_parallel {
            stats.parallel_executions += 1;
        } else {
            stats.sequential_executions += 1;
        }
        let mut pages = Vec::with_capacity(specs.len());
        for chunk in chunks {
            stats.chunks_processed += 1;
            stats.total_chunk_time += chunk.elapsed;
            stats.chunk_sizes.push(chunk.pages.len());
            if self.options.progress_reporting {
                stats.pages_completed += chunk.pages.len();
                stats.total_page_time += chunk.page_time;
                *stats.thread_usage.entry(chunk.thread_id).or_default() += chunk.pages.len();
            }
            pages.extend(chunk.pages);
        }
        stats.total_pages_processed = pages.len();
        Ok(pages)
    }
}
