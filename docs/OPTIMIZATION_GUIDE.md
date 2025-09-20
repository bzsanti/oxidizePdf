# oxidize-pdf Optimization Guide

Guía práctica para optimizar el rendimiento de oxidize-pdf v1.2.x

## 📊 Baseline Actual (Agosto 2025)

### Parser Performance
- **Current**: 42.6 PDFs/segundo promedio
- **Target**: 100+ PDFs/segundo  
- **Improvement needed**: 2.4x

### Writer Performance  
- **Current**: ~12,000 páginas/segundo (contenido simple)
- **Target**: 10,000+ páginas/segundo (contenido real)
- **Challenge**: Mantener rendimiento con contenido complejo

## 🔍 Parser Optimizations

### 1. Profiling y Análisis

#### Setup de profiling:
```bash
# Instalar herramientas
cargo install flamegraph
sudo dtrace -n 'profile-997 /execname == "oxidizepdf"/ { @[ustack()] = count(); }'

# Generar flamegraph
cargo flamegraph --bin oxidizepdf -- info sample.pdf
```

#### Áreas críticas identificadas:
- **Cross-reference table parsing**: ~30% del tiempo total
- **Stream decompression**: ~25% del tiempo  
- **Object resolution**: ~20% del tiempo
- **String/name decoding**: ~15% del tiempo
- **Memory allocation**: ~10% del tiempo

### 2. Optimizaciones Específicas

#### 2.1 Cache de objetos frecuentes
```rust
// Implementar en parser/document.rs
use std::collections::HashMap;
use std::sync::Arc;

pub struct CachedParser {
    object_cache: HashMap<ObjectId, Arc<PdfObject>>,
    xref_cache: HashMap<u64, XrefEntry>,
}

impl CachedParser {
    pub fn get_object_cached(&mut self, id: ObjectId) -> Result<Arc<PdfObject>> {
        if let Some(cached) = self.object_cache.get(&id) {
            return Ok(cached.clone());
        }
        
        let obj = self.parse_object(id)?;
        let shared = Arc::new(obj);
        self.object_cache.insert(id, shared.clone());
        Ok(shared)
    }
}
```

#### 2.2 Parallel parsing de páginas independientes
```rust
use rayon::prelude::*;

pub fn parse_pages_parallel(&self, page_refs: &[ObjectRef]) -> Result<Vec<Page>> {
    page_refs.par_iter()
        .map(|page_ref| self.parse_page(*page_ref))
        .collect()
}
```

#### 2.3 Optimizar decodificación de streams
```rust
// Reutilizar buffers de decompresión
use std::cell::RefCell;

thread_local! {
    static DECOMPRESS_BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(64 * 1024));
}

pub fn decompress_stream_optimized(compressed: &[u8]) -> Result<Vec<u8>> {
    DECOMPRESS_BUFFER.with(|buf| {
        let mut buffer = buf.borrow_mut();
        buffer.clear();
        
        // Reuse existing capacity
        flate2::read::ZlibDecoder::new(compressed)
            .read_to_end(&mut buffer)?;
            
        Ok(buffer.clone())
    })
}
```

### 3. Métricas de Progreso

Ejecutar benchmarks después de cada optimización:
```bash
python3 tools/benchmarks/benchmark_parser.py
```

Targets incrementales:
- **Phase 1**: 60 PDFs/seg (+40%)
- **Phase 2**: 80 PDFs/seg (+30%) 
- **Phase 3**: 100+ PDFs/seg (+25%)

## ⚡ Writer Optimizations

### 1. Memory Pool Implementation

#### 1.1 Buffer reuse para contenido repetitivo
```rust
pub struct BufferPool {
    small_buffers: Vec<Vec<u8>>,   // < 4KB
    medium_buffers: Vec<Vec<u8>>,  // 4KB - 64KB  
    large_buffers: Vec<Vec<u8>>,   // > 64KB
}

impl BufferPool {
    pub fn get_buffer(&mut self, size: usize) -> Vec<u8> {
        let pool = match size {
            s if s < 4096 => &mut self.small_buffers,
            s if s < 65536 => &mut self.medium_buffers, 
            _ => &mut self.large_buffers,
        };
        
        pool.pop()
            .map(|mut buf| { buf.clear(); buf.reserve(size); buf })
            .unwrap_or_else(|| Vec::with_capacity(size))
    }
}
```

#### 1.2 Batch I/O operations
```rust
use std::io::{BufWriter, Write};

pub struct BatchWriter<W: Write> {
    writer: BufWriter<W>,
    batch_size: usize,
    current_batch: Vec<u8>,
}

impl<W: Write> BatchWriter<W> {
    pub fn write_pdf_object(&mut self, obj: &PdfObject) -> Result<()> {
        obj.serialize_into(&mut self.current_batch)?;
        
        if self.current_batch.len() >= self.batch_size {
            self.flush_batch()?;
        }
        
        Ok(())
    }
    
    fn flush_batch(&mut self) -> Result<()> {
        self.writer.write_all(&self.current_batch)?;
        self.current_batch.clear();
        Ok(())
    }
}
```

### 2. Intelligent Compression

#### 2.1 Content-aware compression
```rust
pub enum ContentType {
    Text,      // Use Flate compression
    Image,     // Skip compression (already compressed)  
    Vector,    // Use aggressive Flate
    Mixed,     // Analyze and choose
}

pub fn compress_content_optimized(content: &[u8], content_type: ContentType) -> Vec<u8> {
    match content_type {
        ContentType::Text | ContentType::Vector => {
            // High compression ratio for text/vector
            flate2_compress(content, flate2::Compression::best())
        },
        ContentType::Image => {
            // Don't double-compress images
            content.to_vec()
        },
        ContentType::Mixed => {
            // Analyze content and choose strategy
            analyze_and_compress(content)
        }
    }
}
```

### 3. Parallel Processing

#### 3.1 Pipeline de generación
```rust
use crossbeam::channel;

pub struct ParallelPdfWriter {
    page_queue: channel::Receiver<Page>,
    processed_queue: channel::Sender<ProcessedPage>,
    num_workers: usize,
}

impl ParallelPdfWriter {
    pub fn start_workers(&self) {
        for _ in 0..self.num_workers {
            let receiver = self.page_queue.clone();
            let sender = self.processed_queue.clone();
            
            std::thread::spawn(move || {
                while let Ok(page) = receiver.recv() {
                    let processed = process_page_content(page);
                    sender.send(processed).unwrap();
                }
            });
        }
    }
}
```

## 📈 Benchmarking Strategy

### 1. Continuous Benchmarking

#### 1.1 Automated regression testing
```bash
#!/bin/bash
# benchmark_regression.sh

echo "Running regression benchmarks..."

# Current performance
python3 tools/benchmarks/benchmark_parser.py > current_results.json
python3 tools/benchmarks/benchmark_writer.py >> current_results.json

# Compare with baseline
python3 compare_with_baseline.py current_results.json baseline_results.json

# Alert if performance degraded > 5%
if [ $? -eq 1 ]; then
    echo "❌ Performance regression detected!"
    exit 1
fi
```

#### 1.2 Performance targets per release
```toml
# benchmark_targets.toml
[parser]
v1_2_0 = "42.6 PDFs/sec"
v1_3_0 = "60.0 PDFs/sec"  # Target
v1_4_0 = "80.0 PDFs/sec"  # Stretch goal

[writer_simple]
v1_2_0 = "12000 pages/sec"
v1_3_0 = "15000 pages/sec"

[writer_complex] 
v1_2_0 = "TBD"  # Not yet measured
v1_3_0 = "500 pages/sec"   # Target for real content
```

### 2. Real-world Test Cases

#### 2.1 Complex document benchmarks
```python
# tools/benchmarks/complex_documents.py
COMPLEX_TESTS = [
    {
        "name": "financial_report",
        "pages": 50,
        "content": {
            "tables": 20,
            "charts": 10, 
            "images": 5,
            "complex_text": True
        },
        "target_pages_per_sec": 100
    },
    {
        "name": "technical_manual", 
        "pages": 200,
        "content": {
            "diagrams": 50,
            "code_blocks": 100,
            "cross_references": 500
        },
        "target_pages_per_sec": 50
    }
]
```

## 🛠️ Implementation Strategy

### Phase 1: Low-hanging Fruit (2-4 weeks)
1. **Fix parser object caching** - Expected: +30% performance
2. **Implement buffer pooling** - Expected: +20% memory efficiency
3. **Optimize decompression** - Expected: +15% performance

### Phase 2: Architectural Changes (4-6 weeks)  
1. **Parallel page parsing** - Expected: +40% on multi-page docs
2. **Streaming writer implementation** - Expected: +25% memory efficiency
3. **Intelligent compression** - Expected: +10% file size, +5% speed

### Phase 3: Advanced Optimizations (6-8 weeks)
1. **Custom PDF parser with SIMD** - Expected: +50% parsing speed
2. **Zero-copy string operations** - Expected: +20% memory efficiency  
3. **Parallel compression pipeline** - Expected: +30% large document speed

## 📊 Success Metrics

### Parser Success Criteria
- [x] Baseline measurement: 42.6 PDFs/sec
- [ ] Phase 1 target: 60 PDFs/sec (+40%)
- [ ] Phase 2 target: 80 PDFs/sec (+88%)  
- [ ] Phase 3 target: 100+ PDFs/sec (+135%)

### Writer Success Criteria
- [x] Simple content baseline: 12,000 pages/sec
- [ ] Complex content baseline: TBD (measure first)
- [ ] Complex content target: 500+ pages/sec
- [ ] Memory usage: <50MB for 1000-page document

### Quality Metrics
- [x] Success rate: 98.8%
- [ ] Target success rate: 99.5%+
- [ ] Memory leaks: 0 detected
- [ ] Crash rate: 0%

## 🔧 Tools and Resources

### Profiling Tools
- **flamegraph**: Visualizar hotspots de CPU
- **valgrind**: Memory usage analysis  
- **perf**: Linux system-level profiling
- **heaptrack**: Heap memory tracking

### Development Commands
```bash
# Profile parsing performance
cargo flamegraph --bin oxidizepdf -- info large_file.pdf

# Memory usage analysis  
valgrind --tool=massif cargo run --bin oxidizepdf -- info large_file.pdf

# Benchmark specific operations
cargo bench --bench parser_benchmarks

# Test memory leaks
cargo test --features=leak-detection
```

## 📞 Getting Help

1. **Performance Issues**: File issue with benchmark results
2. **Memory Problems**: Include heaptrack/valgrind output
3. **Optimization Ideas**: Propose in GitHub Discussions
4. **Benchmark Questions**: Reference this guide

## 🎯 Summary

oxidize-pdf v1.2.x tiene rendimiento sólido pero hay margen significativo para mejoras. 
Las optimizaciones deben ser:
- **Medibles**: Usar benchmarks antes/después
- **Incrementales**: Mejoras verificables paso a paso  
- **Sostenibles**: No sacrificar calidad por velocidad

Con las estrategias de este documento, se puede lograr **2-3x mejora** en rendimiento manteniendo la confiabilidad y calidad actual.