#!/usr/bin/env python3
"""
Benchmark de rendimiento del parser de oxidize-pdf
Script único y consolidado para medir parsing de PDFs reales
"""

import subprocess
import time
import os
import json
import sys
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
from collections import defaultdict
import statistics

def get_pdf_info(pdf_path):
    """Obtener información básica del PDF"""
    try:
        size = os.path.getsize(pdf_path)
        
        # Categorías por tamaño
        if size < 100_000:  # < 100KB
            category = "small"
        elif size < 1_000_000:  # < 1MB
            category = "medium"
        else:
            category = "large"
            
        return {
            'size': size,
            'category': category,
            'pages': 1  # Asumimos 1 página por defecto
        }
    except:
        return None

def parse_pdf(pdf_path, binary_path):
    """Parsear un PDF y medir tiempo"""
    start_time = time.perf_counter()
    
    try:
        result = subprocess.run(
            [binary_path, "info", str(pdf_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        end_time = time.perf_counter()
        parse_time = end_time - start_time
        
        return {
            'file': os.path.basename(pdf_path),
            'success': result.returncode == 0,
            'parse_time': parse_time,
            'error': result.stderr.strip() if result.returncode != 0 else None
        }
        
    except subprocess.TimeoutExpired:
        return {
            'file': os.path.basename(pdf_path),
            'success': False,
            'parse_time': 30.0,
            'error': "Timeout after 30 seconds"
        }
    except Exception as e:
        return {
            'file': os.path.basename(pdf_path),
            'success': False,
            'parse_time': 0.0,
            'error': str(e)
        }

def main():
    print("🔍 oxidize-pdf Parser Benchmark")
    print("=" * 40)
    
    # Verificar binary
    binary_path = "./target/release/oxidizepdf"
    if not os.path.exists(binary_path):
        print(f"❌ Error: Binary no encontrado en {binary_path}")
        print("   Ejecuta: cargo build --release")
        return 1
    
    # Encontrar PDFs
    fixtures_path = Path("tests/fixtures")
    if not fixtures_path.exists():
        fixtures_path = Path("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf-render/tests/fixtures")
    
    if not fixtures_path.exists():
        print("❌ Error: No se encontró el directorio de fixtures")
        return 1
    
    pdf_files = list(fixtures_path.glob("*.pdf"))
    if not pdf_files:
        print("❌ Error: No se encontraron PDFs en fixtures")
        return 1
    
    print(f"📁 Encontrados {len(pdf_files)} PDFs para analizar")
    print()
    
    # Obtener información de PDFs
    print("📊 Analizando PDFs...")
    pdf_info = {}
    categories = defaultdict(list)
    total_size = 0
    
    for pdf_path in pdf_files:
        info = get_pdf_info(pdf_path)
        if info:
            pdf_info[pdf_path] = info
            categories[info['category']].append(pdf_path)
            total_size += info['size']
    
    print(f"   Total: {len(pdf_info)} PDFs válidos")
    print(f"   Tamaño: {total_size / (1024*1024):.1f} MB")
    for category, pdfs in categories.items():
        avg_size = sum(pdf_info[p]['size'] for p in pdfs) / len(pdfs) / 1024
        print(f"   {category}: {len(pdfs)} PDFs (avg: {avg_size:.1f} KB)")
    print()
    
    # Ejecutar parsing
    print("⚡ Ejecutando benchmark...")
    results = []
    
    start_total = time.perf_counter()
    
    for i, pdf_path in enumerate(pdf_info.keys(), 1):
        if i % 100 == 0:
            print(f"   {i}/{len(pdf_info)} ({i/len(pdf_info)*100:.0f}%)")
        
        result = parse_pdf(pdf_path, binary_path)
        result.update(pdf_info[pdf_path])
        results.append(result)
    
    end_total = time.perf_counter()
    total_time = end_total - start_total
    
    print(f"   ✅ Completado en {total_time:.2f} segundos")
    print()
    
    # Análisis de resultados
    successful = [r for r in results if r['success']]
    failed = [r for r in results if not r['success']]
    
    print("📈 RESULTADOS")
    print("=" * 30)
    print(f"✅ Exitosos: {len(successful)}/{len(results)} ({len(successful)/len(results)*100:.1f}%)")
    
    if successful:
        parse_times = [r['parse_time'] for r in successful]
        
        total_parse_time = sum(parse_times)
        avg_time = statistics.mean(parse_times)
        median_time = statistics.median(parse_times)
        p95_time = statistics.quantiles(parse_times, n=20)[18] if len(parse_times) >= 20 else max(parse_times)
        p99_time = statistics.quantiles(parse_times, n=100)[98] if len(parse_times) >= 100 else max(parse_times)
        
        pdfs_per_second = len(successful) / total_parse_time
        
        print()
        print("🎯 RENDIMIENTO:")
        print(f"   PDFs/segundo: {pdfs_per_second:.1f}")
        print(f"   Tiempo avg:   {avg_time*1000:.1f}ms")
        print(f"   Tiempo P50:   {median_time*1000:.1f}ms")
        print(f"   Tiempo P95:   {p95_time*1000:.1f}ms")
        print(f"   Tiempo P99:   {p99_time*1000:.1f}ms")
        print()
        
        # Por categoría
        print("📊 POR TAMAÑO:")
        for category in ['small', 'medium', 'large']:
            cat_results = [r for r in successful if r['category'] == category]
            if cat_results:
                cat_times = [r['parse_time'] for r in cat_results]
                cat_total_time = sum(cat_times)
                cat_pdfs_per_sec = len(cat_results) / cat_total_time
                cat_avg_time = statistics.mean(cat_times)
                
                print(f"   {category.upper()}: {cat_pdfs_per_sec:.1f} PDFs/seg ({cat_avg_time*1000:.1f}ms avg)")
    
    if failed:
        print()
        print("❌ ERRORES:")
        error_types = defaultdict(int)
        for result in failed:
            error = result.get('error', 'Unknown')
            if 'encrypted' in error.lower():
                error_types['Encrypted'] += 1
            elif 'timeout' in error.lower():
                error_types['Timeout'] += 1
            elif 'invalid' in error.lower():
                error_types['Invalid'] += 1
            else:
                error_types['Other'] += 1
        
        for error_type, count in error_types.items():
            print(f"   {error_type}: {count}")
    
    # Guardar resultados
    output = {
        'timestamp': time.time(),
        'summary': {
            'total_pdfs': len(results),
            'successful': len(successful),
            'success_rate': len(successful) / len(results) * 100,
            'pdfs_per_second': pdfs_per_second if successful else 0,
            'avg_time_ms': avg_time * 1000 if successful else 0
        },
        'results': results
    }
    
    with open("tools/benchmarks/parser_results.json", 'w') as f:
        json.dump(output, f, indent=2)
    
    print()
    print("💾 Resultados guardados en: tools/benchmarks/parser_results.json")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())