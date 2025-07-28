#!/usr/bin/env python3
"""
Análisis completo y optimizado de todos los PDFs para oxidize-pdf.
Diseñado para ser usado como comando slash personalizado.
"""

import subprocess
import os
import json
import time
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from collections import defaultdict
import signal

def signal_handler(sig, frame):
    print('\n\nAnálisis interrumpido por el usuario.')
    sys.exit(0)

signal.signal(signal.SIGINT, signal_handler)

def analyze_pdf(pdf_path, binary_path):
    """Analiza un PDF individual."""
    result = {
        'file': os.path.basename(pdf_path),
        'path': pdf_path,
        'size': os.path.getsize(pdf_path),
        'status': 'unknown',
        'error_type': None,
        'error_message': None,
        'pdf_version': None
    }
    
    try:
        cmd_result = subprocess.run(
            [binary_path, "info", pdf_path],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        if cmd_result.returncode == 0:
            result['status'] = 'success'
            # Extraer versión PDF del output
            for line in cmd_result.stdout.split('\n'):
                if 'PDF Version:' in line:
                    result['pdf_version'] = line.split(':', 1)[1].strip()
        else:
            result['status'] = 'error'
            stderr = cmd_result.stderr.strip()
            result['error_message'] = stderr
            
            # Categorizar errores - ORDEN IMPORTANTE: encriptación primero
            if 'PDF is encrypted' in stderr or 'Encryption not supported' in stderr:
                result['error_type'] = 'EncryptionNotSupported'
            elif 'Invalid header' in stderr:
                result['error_type'] = 'InvalidHeader'
            elif 'Invalid cross-reference' in stderr or 'xref' in stderr.lower():
                result['error_type'] = 'InvalidXRef'
            elif 'Character encoding' in stderr or 'encoding' in stderr.lower():
                result['error_type'] = 'CharacterEncoding'
            elif 'Circular reference' in stderr:
                result['error_type'] = 'CircularReference'
            elif 'Invalid object reference' in stderr:
                result['error_type'] = 'InvalidObjectReference'
            elif 'Stream error' in stderr:
                result['error_type'] = 'StreamError'
            elif 'Syntax error' in stderr:
                result['error_type'] = 'SyntaxError'
            elif 'File is empty' in stderr:
                result['error_type'] = 'EmptyFile'
            else:
                result['error_type'] = 'Other'
                
    except subprocess.TimeoutExpired:
        result['status'] = 'timeout'
        result['error_type'] = 'Timeout'
        result['error_message'] = 'Timeout after 5 seconds'
    except Exception as e:
        result['status'] = 'exception'
        result['error_type'] = 'Exception'
        result['error_message'] = str(e)
    
    return result

def main():
    print("Análisis Completo de PDFs - oxidize-pdf")
    print("=" * 40)
    
    # Verificar que existe el binary
    binary_path = "./target/release/oxidizepdf"
    if not os.path.exists(binary_path):
        print(f"Error: Binary no encontrado en {binary_path}")
        print("Ejecuta: cargo build --release")
        return 1
    
    # Obtener todos los PDFs
    fixtures_dir = "tests/fixtures"
    if not os.path.exists(fixtures_dir):
        print(f"Error: Directorio {fixtures_dir} no encontrado")
        return 1
    
    pdf_files = []
    for f in os.listdir(fixtures_dir):
        if f.endswith('.pdf'):
            pdf_files.append(os.path.join(fixtures_dir, f))
    
    if not pdf_files:
        print(f"Error: No se encontraron PDFs en {fixtures_dir}")
        return 1
    
    pdf_files.sort()  # Para consistencia
    total_pdfs = len(pdf_files)
    
    print(f"Encontrados {total_pdfs} PDFs para analizar")
    print(f"Procesando en grupos de 50 con 8 workers paralelos...")
    print("Presiona Ctrl+C para interrumpir\n")
    
    # Análisis paralelo
    results = []
    start_time = time.time()
    completed = 0
    
    with ThreadPoolExecutor(max_workers=8) as executor:
        # Enviar todas las tareas
        future_to_pdf = {
            executor.submit(analyze_pdf, pdf, binary_path): pdf 
            for pdf in pdf_files
        }
        
        # Procesar resultados conforme se completan
        for future in as_completed(future_to_pdf):
            completed += 1
            pdf = future_to_pdf[future]
            
            try:
                result = future.result()
                results.append(result)
                
                # Mostrar progreso cada 50 PDFs o en importantes
                if completed % 50 == 0 or completed in [1, 10, 25] or completed == total_pdfs:
                    progress_pct = (completed / total_pdfs) * 100
                    elapsed = time.time() - start_time
                    rate = completed / elapsed if elapsed > 0 else 0
                    eta = (total_pdfs - completed) / rate if rate > 0 else 0
                    print(f"Progreso: {completed}/{total_pdfs} ({progress_pct:.1f}%) - "
                          f"Velocidad: {rate:.1f} PDFs/s - ETA: {eta:.1f}s")
                    
            except Exception as exc:
                print(f"Error procesando {pdf}: {exc}")
    
    total_time = time.time() - start_time
    
    # Analizar resultados
    success_count = sum(1 for r in results if r['status'] == 'success')
    error_count = sum(1 for r in results if r['status'] == 'error')
    timeout_count = sum(1 for r in results if r['status'] == 'timeout')
    exception_count = sum(1 for r in results if r['status'] == 'exception')
    
    # Agrupar errores por tipo
    error_types = defaultdict(int)
    for r in results:
        if r['error_type']:
            error_types[r['error_type']] += 1
    
    # Generar reporte
    print(f"\n\nAnálisis Completo de PDFs - oxidize-pdf")
    print("=" * 40)
    print(f"Total PDFs analizados: {total_pdfs}")
    print(f"Exitosos: {success_count} ({success_count/total_pdfs*100:.1f}%)")
    print(f"Errores: {error_count} ({error_count/total_pdfs*100:.1f}%)")
    print(f"Timeouts: {timeout_count}")
    print(f"Excepciones: {exception_count}")
    print(f"Tiempo total: {total_time:.1f}s")
    print(f"Velocidad promedio: {total_pdfs/total_time:.1f} PDFs/s")
    
    if error_types:
        print(f"\nDesglose de Errores:")
        for error_type, count in sorted(error_types.items(), key=lambda x: x[1], reverse=True):
            pct = (count / total_pdfs) * 100
            print(f"  {error_type}: {count} ({pct:.1f}%)")
    
    # Comparación con baseline
    baseline_total = 743
    baseline_success = 550
    baseline_pct = (baseline_success / baseline_total) * 100
    current_pct = (success_count / total_pdfs) * 100
    improvement = current_pct - baseline_pct
    
    print(f"\nMejoras desde baseline:")
    print(f"  Baseline: {baseline_success}/{baseline_total} ({baseline_pct:.1f}%)")
    print(f"  Actual: {success_count}/{total_pdfs} ({current_pct:.1f}%)")
    print(f"  Mejora: {improvement:+.1f}%")
    
    # Próximas prioridades
    print(f"\nPróximas prioridades:")
    priority_errors = sorted(error_types.items(), key=lambda x: x[1], reverse=True)[:3]
    for i, (error_type, count) in enumerate(priority_errors, 1):
        print(f"{i}. {error_type} ({count} casos)")
    
    # Guardar resultados detallados
    timestamp = time.strftime("%Y%m%d_%H%M%S")
    json_filename = f"pdf_analysis_complete_{timestamp}.json"
    
    summary = {
        'timestamp': timestamp,
        'total_pdfs': total_pdfs,
        'successful': success_count,
        'errors': error_count,
        'timeouts': timeout_count,
        'exceptions': exception_count,
        'success_rate': current_pct,
        'total_time': total_time,
        'error_types': dict(error_types),
        'baseline_comparison': {
            'baseline_success_rate': baseline_pct,
            'current_success_rate': current_pct,
            'improvement': improvement
        },
        'results': results
    }
    
    with open(json_filename, 'w') as f:
        json.dump(summary, f, indent=2)
    
    print(f"\nResultados detallados guardados en: {json_filename}")
    
    # Mostrar algunos casos específicos si hay mejoras notables
    if improvement > 5:  # Si la mejora es significativa
        print(f"\n✅ ¡Mejora significativa! (+{improvement:.1f}%)")
        
        # Mostrar PDFs que ahora funcionan (solo algunos ejemplos)
        newly_working = []
        known_problematic = [
            "Course_Glossary_SUPPLY_LIST.pdf",
            "liarsandoutliers_enablingthetrustthatsocietyneedstothrive.pdf",
            "cryptography_engineering_design_principles_and_practical_applications.pdf"
        ]
        
        for result in results:
            if result['status'] == 'success' and result['file'] in known_problematic:
                newly_working.append(result['file'])
        
        if newly_working:
            print(f"\nPDFs problemáticos ahora funcionando:")
            for pdf in newly_working:
                print(f"  ✓ {pdf}")
    
    return 0

if __name__ == "__main__":
    exit_code = main()
    sys.exit(exit_code)