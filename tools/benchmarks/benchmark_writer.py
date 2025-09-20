#!/usr/bin/env python3
"""
Benchmark de rendimiento del writer de oxidize-pdf
Script único y consolidado para medir generación de PDFs
"""

import subprocess
import time
import os
import json
import sys
from pathlib import Path
import tempfile

def create_simple_benchmark():
    """Crear programa Rust para benchmark simple (solo texto básico)"""
    return '''
use oxidize_pdf::{Document, Font, Page, Result};
use std::env;
use std::time::Instant;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let page_count = if args.len() > 1 {
        args[1].parse().unwrap_or(100)
    } else {
        100
    };

    let start_time = Instant::now();
    
    let mut doc = Document::new();
    doc.set_title("Simple Benchmark");
    
    for i in 0..page_count {
        let mut page = Page::a4();
        
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 750.0)
            .write(&format!("Page {} of {}", i + 1, page_count))?;
            
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, 700.0)
            .write("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")?;
            
        doc.add_page(page);
    }
    
    let generation_time = start_time.elapsed();
    let write_start = Instant::now();
    doc.save("examples/results/simple_benchmark.pdf")?;
    let write_time = write_start.elapsed();
    let total_time = start_time.elapsed();
    
    println!("PAGES={}", page_count);
    println!("GENERATION_MS={}", generation_time.as_millis());
    println!("WRITE_MS={}", write_time.as_millis());
    println!("TOTAL_MS={}", total_time.as_millis());
    println!("PAGES_PER_SEC={:.2}", page_count as f64 / total_time.as_secs_f64());
    
    Ok(())
}
'''

def create_realistic_benchmark():
    """Crear programa Rust para benchmark realista (múltiples párrafos y fonts)"""
    return '''
use oxidize_pdf::{Document, Font, Page, Result};
use std::env;
use std::time::Instant;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let page_count = if args.len() > 1 {
        args[1].parse().unwrap_or(50)
    } else {
        50
    };

    let start_time = Instant::now();
    
    let mut doc = Document::new();
    doc.set_title("Realistic Document Benchmark");
    
    let paragraphs = [
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
        "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.",
        "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo.",
    ];
    
    for i in 0..page_count {
        let mut page = Page::a4();
        let mut y_pos = 750.0;
        
        // Título
        page.text()
            .set_font(Font::HelveticaBold, 16.0)
            .at(50.0, y_pos)
            .write(&format!("Document Section {} - Page {}", (i / 10) + 1, i + 1))?;
        y_pos -= 40.0;
        
        // Múltiples párrafos con diferentes fonts
        for (j, paragraph) in paragraphs.iter().enumerate() {
            // Subtítulo
            page.text()
                .set_font(Font::HelveticaBold, 12.0)
                .at(50.0, y_pos)
                .write(&format!("Section {}", j + 1))?;
            y_pos -= 20.0;
            
            // Dividir párrafo en líneas
            let words: Vec<&str> = paragraph.split_whitespace().collect();
            let mut line = String::new();
            let words_per_line = 12;
            
            for (word_idx, word) in words.iter().enumerate() {
                line.push_str(word);
                line.push(' ');
                
                if (word_idx + 1) % words_per_line == 0 || word_idx == words.len() - 1 {
                    page.text()
                        .set_font(Font::Helvetica, 10.0)
                        .at(70.0, y_pos)
                        .write(&line.trim())?;
                    
                    y_pos -= 15.0;
                    line.clear();
                    
                    if y_pos < 150.0 {
                        break;
                    }
                }
            }
            
            y_pos -= 10.0;
            if y_pos < 100.0 {
                break;
            }
        }
        
        // Footer
        page.text()
            .set_font(Font::Courier, 8.0)
            .at(50.0, 50.0)
            .write(&format!("Page {} of {} | Realistic Benchmark", i + 1, page_count))?;
        
        doc.add_page(page);
    }
    
    let generation_time = start_time.elapsed();
    let write_start = Instant::now();
    doc.save("examples/results/realistic_benchmark.pdf")?;
    let write_time = write_start.elapsed();
    let total_time = start_time.elapsed();
    
    println!("PAGES={}", page_count);
    println!("GENERATION_MS={}", generation_time.as_millis());
    println!("WRITE_MS={}", write_time.as_millis());
    println!("TOTAL_MS={}", total_time.as_millis());
    println!("PAGES_PER_SEC={:.2}", page_count as f64 / total_time.as_secs_f64());
    
    Ok(())
}
'''

def run_benchmark(benchmark_name, rust_code, page_counts, iterations=3):
    """Ejecutar un benchmark específico"""
    
    print(f"📄 Ejecutando: {benchmark_name}")
    print("-" * 40)
    
    # Crear archivo temporal con el código Rust
    with tempfile.NamedTemporaryFile(mode='w', suffix='.rs', delete=False) as f:
        f.write(rust_code)
        temp_rust_file = f.name
    
    try:
        # Compilar
        temp_binary = temp_rust_file.replace('.rs', '')
        
        compile_cmd = [
            "rustc", 
            "--edition", "2021",
            "-L", "target/release/deps",
            "--extern", "oxidize_pdf=target/release/liboxidize_pdf.rlib",
            "-o", temp_binary,
            temp_rust_file
        ]
        
        print("📦 Compilando benchmark...")
        compile_result = subprocess.run(compile_cmd, capture_output=True, text=True)
        
        if compile_result.returncode != 0:
            print("❌ Error de compilación. Intentando con cargo...")
            # Fallback: usar cargo con el ejemplo existente
            if "simple" in benchmark_name.lower():
                binary_path = "target/release/examples/performance_benchmark_1000"
                compile_result = subprocess.run([
                    "cargo", "build", "--release", "--example", "performance_benchmark_1000"
                ], capture_output=True, text=True)
            else:
                print(f"❌ No se pudo compilar {benchmark_name}")
                return None
        else:
            binary_path = temp_binary
        
        if compile_result.returncode != 0:
            print(f"❌ Error de compilación final:")
            print(compile_result.stderr)
            return None
        
        results = []
        
        for page_count in page_counts:
            print(f"\n📋 {page_count} páginas:")
            iteration_results = []
            
            for i in range(iterations):
                print(f"  Iteración {i+1}/{iterations}...", end=" ")
                
                # Medir tiempo total real
                start_time = time.perf_counter()
                
                result = subprocess.run([
                    binary_path, str(page_count)
                ], capture_output=True, text=True)
                
                end_time = time.perf_counter()
                wall_time = end_time - start_time
                
                if result.returncode == 0:
                    # Parsear output
                    data = {}
                    for line in result.stdout.strip().split('\n'):
                        if '=' in line:
                            key, value = line.split('=', 1)
                            if key in ['PAGES', 'GENERATION_MS', 'WRITE_MS', 'TOTAL_MS']:
                                data[key] = int(value) if key != 'PAGES_PER_SEC' else float(value)
                            elif key == 'PAGES_PER_SEC':
                                data[key] = float(value)
                    
                    wall_pages_per_sec = page_count / wall_time
                    internal_pages_per_sec = data.get('PAGES_PER_SEC', 0)
                    
                    iteration_data = {
                        'wall_time_ms': wall_time * 1000,
                        'wall_pages_per_sec': wall_pages_per_sec,
                        'internal_pages_per_sec': internal_pages_per_sec,
                        'generation_ms': data.get('GENERATION_MS', 0),
                        'write_ms': data.get('WRITE_MS', 0),
                        'total_internal_ms': data.get('TOTAL_MS', 0)
                    }
                    
                    iteration_results.append(iteration_data)
                    print(f"{wall_pages_per_sec:.1f} pág/seg")
                else:
                    print("❌ Error")
            
            if iteration_results:
                # Calcular promedios
                avg_wall_pages_per_sec = sum(r['wall_pages_per_sec'] for r in iteration_results) / len(iteration_results)
                avg_generation_ms = sum(r['generation_ms'] for r in iteration_results) / len(iteration_results)
                avg_write_ms = sum(r['write_ms'] for r in iteration_results) / len(iteration_results)
                
                write_percentage = (avg_write_ms / (avg_generation_ms + avg_write_ms)) * 100 if (avg_generation_ms + avg_write_ms) > 0 else 0
                
                result_summary = {
                    'page_count': page_count,
                    'avg_pages_per_sec': avg_wall_pages_per_sec,
                    'avg_generation_ms': avg_generation_ms,
                    'avg_write_ms': avg_write_ms,
                    'write_percentage': write_percentage,
                    'iterations': iteration_results
                }
                
                results.append(result_summary)
                
                print(f"  📊 Promedio: {avg_wall_pages_per_sec:.1f} pág/seg")
                print(f"      Gen: {avg_generation_ms:.0f}ms, Write: {avg_write_ms:.0f}ms ({write_percentage:.0f}% I/O)")
        
        return results
        
    finally:
        # Cleanup
        if os.path.exists(temp_rust_file):
            os.unlink(temp_rust_file)
        temp_binary = temp_rust_file.replace('.rs', '')
        if os.path.exists(temp_binary):
            os.unlink(temp_binary)

def main():
    print("🚀 oxidize-pdf Writer Benchmark")
    print("=" * 40)
    
    # Asegurar directorio de resultados
    os.makedirs("examples/results", exist_ok=True)
    
    # Verificar que tenemos el target compilado
    if not os.path.exists("target/release/liboxidize_pdf.rlib"):
        print("📦 Compilando oxidize-pdf...")
        build_result = subprocess.run(["cargo", "build", "--release"], 
                                    capture_output=True, text=True)
        if build_result.returncode != 0:
            print("❌ Error compilando oxidize-pdf:")
            print(build_result.stderr)
            return 1
    
    all_results = {}
    
    # Benchmark 1: Simple (texto básico)
    print("\n🎯 TEST 1: DOCUMENTO SIMPLE")
    print("=" * 40)
    simple_results = run_benchmark(
        "simple_document", 
        create_simple_benchmark(),
        [100, 500, 1000]
    )
    
    if simple_results:
        all_results['simple'] = {
            'description': 'Documento simple (2 líneas de texto por página)',
            'results': simple_results
        }
    
    # Benchmark 2: Realistic (párrafos completos)
    print("\n🎯 TEST 2: DOCUMENTO REALISTA")
    print("=" * 40)
    realistic_results = run_benchmark(
        "realistic_document", 
        create_realistic_benchmark(),
        [25, 50, 100]
    )
    
    if realistic_results:
        all_results['realistic'] = {
            'description': 'Documento realista (párrafos completos, múltiples fonts)',
            'results': realistic_results
        }
    
    if not all_results:
        print("❌ No se pudo ejecutar ningún benchmark")
        return 1
    
    # Guardar resultados
    output = {
        'timestamp': time.time(),
        'benchmarks': all_results
    }
    
    with open("tools/benchmarks/writer_results.json", 'w') as f:
        json.dump(output, f, indent=2)
    
    # Mostrar resumen
    print("\n" + "=" * 50)
    print("📈 RESUMEN DE RENDIMIENTO")
    print("=" * 50)
    
    for test_name, test_data in all_results.items():
        print(f"\n🎯 {test_data['description'].upper()}:")
        
        for result in test_data['results']:
            pages = result['page_count']
            perf = result['avg_pages_per_sec']
            write_pct = result['write_percentage']
            
            print(f"   {pages:3d} páginas: {perf:7.1f} pág/seg ({write_pct:.0f}% I/O)")
    
    # Comparación con claims
    print(f"\n🔍 ANÁLISIS:")
    
    all_performances = []
    for test_data in all_results.values():
        for result in test_data['results']:
            all_performances.append(result['avg_pages_per_sec'])
    
    if all_performances:
        best_perf = max(all_performances)
        avg_perf = sum(all_performances) / len(all_performances)
        
        print(f"   Mejor rendimiento:     {best_perf:7.1f} pág/seg")
        print(f"   Rendimiento promedio:  {avg_perf:7.1f} pág/seg")
        print(f"   vs Claim anterior:     21,379 pág/seg")
        print(f"   Factor real:           {avg_perf / 21379:.4f}x")
        
        if avg_perf < 1000:
            print(f"   ✅ Métricas realistas y creíbles")
        else:
            print(f"   ⚠️  Rendimiento aún sospechosamente alto")
    
    print()
    print("💾 Resultados guardados en: tools/benchmarks/writer_results.json")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())