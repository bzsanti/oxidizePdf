#!/usr/bin/env python3
"""
Cross-language PDF Library Benchmark
Compares oxidize-pdf against major Python, Java, and C++ PDF libraries
"""

import subprocess
import time
import os
import json
import tempfile
import statistics
from pathlib import Path
from typing import Dict, List, Any, Optional
from datetime import datetime
import sys

# Install dependencies if needed
def ensure_dependencies():
    """Ensure Python PDF libraries are installed"""
    dependencies = [
        'reportlab',
        'pypdf', 
        'PyMuPDF',
        'weasyprint',
        'matplotlib',
        'tabulate'
    ]
    
    for dep in dependencies:
        try:
            __import__(dep.replace('-', '_'))
        except ImportError:
            print(f"Installing {dep}...")
            subprocess.run([sys.executable, '-m', 'pip', 'install', dep], check=True)

class BenchmarkResult:
    def __init__(self, library_name: str, test_case: str, pages: int,
                 generation_time_ms: float, file_size_bytes: int,
                 success: bool, error: Optional[str] = None):
        self.library_name = library_name
        self.test_case = test_case
        self.pages = pages
        self.generation_time_ms = generation_time_ms
        self.file_size_bytes = file_size_bytes
        self.success = success
        self.error = error
        self.timestamp = datetime.now().isoformat()
        
    def to_dict(self) -> Dict[str, Any]:
        return {
            'library_name': self.library_name,
            'test_case': self.test_case,
            'pages': self.pages,
            'generation_time_ms': self.generation_time_ms,
            'file_size_bytes': self.file_size_bytes,
            'file_size_kb': self.file_size_bytes / 1024,
            'pages_per_second': self.pages / (self.generation_time_ms / 1000) if self.generation_time_ms > 0 else 0,
            'success': self.success,
            'error': self.error,
            'timestamp': self.timestamp
        }

def measure_time_and_size(func, output_path: str) -> tuple[float, int, Optional[str]]:
    """Measure execution time and resulting file size"""
    try:
        start_time = time.time()
        func(output_path)
        end_time = time.time()
        
        generation_time = (end_time - start_time) * 1000  # Convert to milliseconds
        file_size = os.path.getsize(output_path) if os.path.exists(output_path) else 0
        
        return generation_time, file_size, None
    except Exception as e:
        return 0.0, 0, str(e)

# Python PDF Library Implementations

def create_pdf_reportlab(pages: int, output_path: str):
    """Create PDF using ReportLab"""
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import A4
    
    c = canvas.Canvas(output_path, pagesize=A4)
    
    for i in range(1, pages + 1):
        c.drawString(50, 750, f"Page {i} - ReportLab performance test")
        c.drawString(50, 730, "Lorem ipsum dolor sit amet, consectetur adipiscing elit.")
        
        # Add some more content
        c.drawString(50, 710, "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.")
        c.rect(50, 600, 500, 100, stroke=1, fill=0)
        
        if i < pages:
            c.showPage()
    
    c.save()

def create_pdf_pymupdf(pages: int, output_path: str):
    """Create PDF using PyMuPDF"""
    import fitz  # PyMuPDF
    
    doc = fitz.open()
    
    for i in range(1, pages + 1):
        page = doc.new_page()
        
        # Add text
        page.insert_text((50, 750), f"Page {i} - PyMuPDF performance test", fontsize=12)
        page.insert_text((50, 730), "Lorem ipsum dolor sit amet, consectetur adipiscing elit.", fontsize=10)
        page.insert_text((50, 710), "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.", fontsize=10)
        
        # Add rectangle
        rect = fitz.Rect(50, 600, 550, 700)
        page.draw_rect(rect, color=(0, 0, 1), fill=(0.9, 0.9, 1.0))
    
    doc.save(output_path)
    doc.close()

def create_pdf_pypdf_writer(pages: int, output_path: str):
    """Create PDF using pypdf (formerly PyPDF2)"""
    from pypdf import PdfWriter
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import A4
    import io
    
    # pypdf is mainly for manipulation, so we create with reportlab first
    # then demonstrate pypdf's manipulation capabilities
    writer = PdfWriter()
    
    for i in range(1, pages + 1):
        # Create page in memory with reportlab
        packet = io.BytesIO()
        c = canvas.Canvas(packet, pagesize=A4)
        c.drawString(50, 750, f"Page {i} - pypdf performance test")
        c.drawString(50, 730, "Lorem ipsum dolor sit amet, consectetur adipiscing elit.")
        c.drawString(50, 710, f"This page was created and processed by pypdf - {i}/{pages}")
        c.rect(50, 600, 500, 100, stroke=1, fill=0)
        c.save()
        
        # Add to writer
        packet.seek(0)
        from pypdf import PdfReader
        reader = PdfReader(packet)
        writer.add_page(reader.pages[0])
    
    with open(output_path, "wb") as output_file:
        writer.write(output_file)

# Rust library benchmarking (external process)

def create_pdf_oxidize_rust(pages: int, output_path: str):
    """Create PDF using oxidize-pdf via Rust example"""
    # Create a temporary Rust program that uses oxidize-pdf
    rust_code = f"""
use oxidize_pdf::{{Document, Page, PageFormat, text::Font, graphics::Color}};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let mut doc = Document::new(PageFormat::A4);
    
    for i in 1..={pages} {{
        let mut page = Page::new(PageFormat::A4);
        page.add_text_simple(
            &format!("Page {{}} - oxidize-pdf performance test", i),
            50.0,
            750.0,
            Font::Helvetica,
            12.0,
            Color::black(),
        )?;
        
        page.add_text_simple(
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
            50.0,
            730.0,
            Font::Helvetica,
            10.0,
            Color::black(),
        )?;
        
        doc.add_page(page);
    }}
    
    doc.save("{output_path}")?;
    Ok(())
}}
"""
    
    # This would require setting up a temporary Rust project
    # For now, we'll call our existing example if available
    try:
        result = subprocess.run([
            "cargo", "run", "--example", "font_spacing_test", 
            "--", "--pages", str(pages), "--output", output_path
        ], capture_output=True, text=True, cwd="../../")
        
        if result.returncode != 0:
            raise Exception(f"Rust execution failed: {result.stderr}")
            
    except Exception as e:
        # Fallback: create a simple file as placeholder
        with open(output_path, 'w') as f:
            f.write(f"PDF placeholder for {pages} pages")
        raise e

# Main benchmarking functions

def run_single_benchmark(library_name: str, create_func, pages: int, iterations: int = 3) -> List[BenchmarkResult]:
    """Run benchmark for a single library with multiple iterations"""
    results = []
    
    for i in range(iterations):
        with tempfile.NamedTemporaryFile(suffix='.pdf', delete=False) as tmp_file:
            tmp_path = tmp_file.name
            
        try:
            generation_time, file_size, error = measure_time_and_size(
                lambda path: create_func(pages, path),
                tmp_path
            )
            
            success = error is None and os.path.exists(tmp_path)
            
            result = BenchmarkResult(
                library_name=library_name,
                test_case=f"simple_pdf_{pages}_pages",
                pages=pages,
                generation_time_ms=generation_time,
                file_size_bytes=file_size,
                success=success,
                error=error
            )
            
            results.append(result)
            
        finally:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)
    
    return results

def run_comprehensive_benchmark() -> Dict[str, List[BenchmarkResult]]:
    """Run comprehensive benchmark across all libraries and page counts"""
    
    libraries = {
        'reportlab': create_pdf_reportlab,
        'pymupdf': create_pdf_pymupdf, 
        'pypdf': create_pdf_pypdf_writer,
        # 'oxidize-pdf': create_pdf_oxidize_rust,  # Enable if Rust setup works
    }
    
    page_counts = [1, 10, 50, 100]
    all_results = {}
    
    print("Running comprehensive PDF library benchmark...")
    print(f"Libraries: {list(libraries.keys())}")
    print(f"Page counts: {page_counts}")
    print("-" * 60)
    
    for library_name, create_func in libraries.items():
        print(f"Benchmarking {library_name}...")
        all_results[library_name] = []
        
        for pages in page_counts:
            print(f"  Testing {pages} pages...")
            
            try:
                results = run_single_benchmark(library_name, create_func, pages)
                all_results[library_name].extend(results)
                
                # Show quick stats
                times = [r.generation_time_ms for r in results if r.success]
                sizes = [r.file_size_bytes for r in results if r.success]
                
                if times and sizes:
                    avg_time = statistics.mean(times)
                    avg_size = statistics.mean(sizes)
                    print(f"    Avg: {avg_time:.1f}ms, {avg_size/1024:.1f}KB")
                else:
                    print(f"    Failed: {results[0].error if results else 'Unknown error'}")
                    
            except Exception as e:
                print(f"    Error: {e}")
                error_result = BenchmarkResult(
                    library_name=library_name,
                    test_case=f"simple_pdf_{pages}_pages",
                    pages=pages,
                    generation_time_ms=0,
                    file_size_bytes=0,
                    success=False,
                    error=str(e)
                )
                all_results[library_name].append(error_result)
    
    return all_results

def generate_report(results: Dict[str, List[BenchmarkResult]], output_path: str = "benchmark_report.json"):
    """Generate comprehensive benchmark report"""
    
    # Convert to JSON-serializable format
    json_results = {}
    for library, result_list in results.items():
        json_results[library] = [r.to_dict() for r in result_list]
    
    # Calculate summary statistics
    summary = {
        'timestamp': datetime.now().isoformat(),
        'libraries_tested': list(results.keys()),
        'total_tests': sum(len(result_list) for result_list in results.values()),
        'library_performance': {}
    }
    
    for library, result_list in results.items():
        successful_results = [r for r in result_list if r.success]
        
        if successful_results:
            times = [r.generation_time_ms for r in successful_results]
            sizes = [r.file_size_bytes for r in successful_results]
            pages_per_sec = [r.pages / (r.generation_time_ms / 1000) for r in successful_results if r.generation_time_ms > 0]
            
            summary['library_performance'][library] = {
                'avg_time_ms': statistics.mean(times),
                'median_time_ms': statistics.median(times),
                'avg_file_size_kb': statistics.mean(sizes) / 1024,
                'avg_pages_per_second': statistics.mean(pages_per_sec) if pages_per_sec else 0,
                'success_rate': len(successful_results) / len(result_list),
                'total_tests': len(result_list)
            }
        else:
            summary['library_performance'][library] = {
                'avg_time_ms': 0,
                'median_time_ms': 0,
                'avg_file_size_kb': 0,
                'avg_pages_per_second': 0,
                'success_rate': 0,
                'total_tests': len(result_list)
            }
    
    # Write full report
    full_report = {
        'summary': summary,
        'detailed_results': json_results
    }
    
    with open(output_path, 'w') as f:
        json.dump(full_report, f, indent=2)
    
    print(f"\nBenchmark report saved to: {output_path}")
    return summary

def print_summary_table(summary: Dict[str, Any]):
    """Print a nice summary table"""
    try:
        from tabulate import tabulate
        
        headers = ["Library", "Avg Time (ms)", "Pages/sec", "Avg Size (KB)", "Success Rate"]
        rows = []
        
        for library, stats in summary['library_performance'].items():
            rows.append([
                library,
                f"{stats['avg_time_ms']:.1f}",
                f"{stats['avg_pages_per_second']:.1f}",
                f"{stats['avg_file_size_kb']:.1f}",
                f"{stats['success_rate']:.1%}"
            ])
        
        print("\n" + "="*80)
        print("PDF LIBRARY BENCHMARK SUMMARY")
        print("="*80)
        print(tabulate(rows, headers=headers, tablefmt="grid"))
        print("="*80)
        
    except ImportError:
        print("\n" + "="*60)
        print("PDF LIBRARY BENCHMARK SUMMARY")
        print("="*60)
        for library, stats in summary['library_performance'].items():
            print(f"{library:15s}: {stats['avg_time_ms']:6.1f}ms avg, {stats['avg_pages_per_second']:6.1f} pages/sec")
        print("="*60)

if __name__ == "__main__":
    print("PDF Library Cross-Language Benchmark")
    print("====================================")
    
    # Ensure dependencies
    ensure_dependencies()
    
    # Run benchmarks
    results = run_comprehensive_benchmark()
    
    # Generate report
    summary = generate_report(results, "python_pdf_benchmark_results.json")
    
    # Print summary
    print_summary_table(summary)
    
    print(f"\nBenchmark completed! Tested {summary['total_tests']} configurations across {len(summary['libraries_tested'])} libraries.")