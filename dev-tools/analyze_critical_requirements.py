#!/usr/bin/env python3
"""
Analiza los requirements ISO más críticos basados en:
1. Tests existentes
2. Funcionalidad core de PDF
3. Frecuencia de uso en PDFs reales
"""

import toml
import re
from pathlib import Path
from collections import defaultdict, Counter

def load_verification_status():
    """Cargar estado de verificación actual"""
    try:
        with open('ISO_VERIFICATION_STATUS.toml', 'r') as f:
            data = toml.load(f)
            return data.get('status', {})
    except Exception as e:
        print(f"Error cargando status: {e}")
        return {}

def find_existing_tests():
    """Encontrar tests ISO existentes"""
    tests = []
    test_dir = Path('oxidize-pdf-core/src/verification/tests')
    
    if not test_dir.exists():
        return tests
    
    for test_file in test_dir.rglob('*.rs'):
        content = test_file.read_text()
        # Buscar macro iso_test!
        matches = re.findall(r'iso_test!\s*\(\s*[^,]+,\s*"([^"]+)"', content)
        for match in matches:
            tests.append({
                'id': match,
                'file': str(test_file.relative_to(Path('.'))),
                'section': match.split('.')[0] if '.' in match else match
            })
    
    return tests

def identify_critical_requirements():
    """Identificar los 50 requirements más críticos"""
    
    # Categorías de criticidad basadas en la especificación PDF
    critical_categories = {
        'document_structure': {
            'priority': 1,
            'requirements': [
                '7.5.2.1',  # Catalog /Type entry
                '7.5.2.2',  # Catalog /Version
                '7.5.3.1',  # Page tree root
                '7.5.4.1',  # Page objects
                '7.5.5.1',  # Name dictionaries
            ]
        },
        'file_structure': {
            'priority': 1,
            'requirements': [
                '7.2.1.1',  # PDF header
                '7.2.2.1',  # Body structure
                '7.2.3.1',  # Cross-reference table
                '7.2.4.1',  # Trailer dictionary
            ]
        },
        'content_streams': {
            'priority': 2,
            'requirements': [
                '7.8.2.1',  # Stream objects
                '7.8.3.1',  # Stream filters
                '9.2.1.1',  # Text objects
                '8.4.1.1',  # Graphics state
            ]
        },
        'fonts': {
            'priority': 2,
            'requirements': [
                '9.6.2.1',  # Font dictionaries
                '9.6.3.1',  # Simple fonts
                '9.8.1.1',  # Font embedding
            ]
        },
        'color_spaces': {
            'priority': 3,
            'requirements': [
                '8.6.3.1',  # DeviceRGB
                '8.6.3.2',  # DeviceCMYK
                '8.6.3.3',  # DeviceGray
            ]
        }
    }
    
    # Expandir con patrones basados en secciones críticas
    critical_patterns = {
        '7.2': 'File Structure (Critical)',
        '7.5': 'Document Structure (Critical)', 
        '7.8': 'Content Streams (High)',
        '8.4': 'Graphics State (High)',
        '8.6.3': 'Device Color Spaces (High)',
        '9.2': 'Text Objects (High)',
        '9.6': 'Fonts (High)',
    }
    
    return critical_categories, critical_patterns

def analyze_current_state():
    """Analizar el estado actual de implementación"""
    print("📊 Analizando estado actual del sistema ISO...")
    
    # Cargar estado de verificación
    status = load_verification_status()
    print(f"✓ Estado cargado: {len(status)} requirements")
    
    # Encontrar tests existentes
    existing_tests = find_existing_tests()
    print(f"✓ Tests encontrados: {len(existing_tests)} implementados")
    
    # Analizar distribución por sección
    section_counts = Counter()
    level_counts = Counter()
    
    for req_id, req_status in status.items():
        if '.' in req_id:
            section = req_id.split('.')[0]
            section_counts[section] += 1
            level_counts[req_status.get('level', 0)] += 1
    
    print("\n📈 Distribución por sección ISO:")
    for section, count in section_counts.most_common(10):
        print(f"  - Sección {section}: {count} requirements")
    
    print(f"\n🎯 Distribución por nivel de implementación:")
    for level in range(5):
        count = level_counts[level]
        percentage = (count / len(status) * 100) if status else 0
        print(f"  - Level {level}: {count} ({percentage:.1f}%)")
    
    # Identificar requirements críticos
    critical_categories, critical_patterns = identify_critical_requirements()
    
    print("\n🚀 Requirements Críticos Identificados:")
    
    critical_requirements = []
    
    # Procesar categorías críticas
    for category, data in critical_categories.items():
        print(f"\n  📂 {category.replace('_', ' ').title()} (Prioridad {data['priority']}):")
        for req_id in data['requirements']:
            req_status = status.get(req_id, {})
            level = req_status.get('level', 0)
            has_test = any(t['id'] == req_id for t in existing_tests)
            
            status_str = f"Level {level}"
            if has_test:
                status_str += " ✓"
            
            critical_requirements.append({
                'id': req_id,
                'category': category,
                'priority': data['priority'],
                'level': level,
                'has_test': has_test,
                'section': req_id.split('.')[0]
            })
            
            print(f"    - {req_id}: {status_str}")
    
    # Buscar requirements adicionales por patrones
    print(f"\n  🔍 Requirements por patrones críticos:")
    pattern_reqs = []
    
    for req_id in status.keys():
        for pattern, description in critical_patterns.items():
            if req_id.startswith(pattern) and len(pattern_reqs) < 30:
                req_status = status.get(req_id, {})
                level = req_status.get('level', 0)
                has_test = any(t['id'] == req_id for t in existing_tests)
                
                pattern_reqs.append({
                    'id': req_id,
                    'category': pattern,
                    'priority': 2,
                    'level': level,
                    'has_test': has_test,
                    'section': req_id.split('.')[0],
                    'description': description
                })
    
    # Mostrar top 20 por patrones
    pattern_reqs_sorted = sorted(pattern_reqs, key=lambda x: (x['priority'], -x['level'], x['has_test']))[:20]
    for req in pattern_reqs_sorted[:10]:
        status_str = f"Level {req['level']}"
        if req['has_test']:
            status_str += " ✓"
        print(f"    - {req['id']}: {status_str} ({req['description']})")
    
    # Combinar y seleccionar top 50
    all_critical = critical_requirements + pattern_reqs_sorted
    
    # Remover duplicados y ordenar por prioridad
    seen = set()
    unique_critical = []
    for req in all_critical:
        if req['id'] not in seen:
            seen.add(req['id'])
            unique_critical.append(req)
    
    # Ordenar por: prioridad, nivel actual (los de mayor nivel primero), si tiene test
    top_50 = sorted(unique_critical, key=lambda x: (x['priority'], -x['level'], not x['has_test']))[:50]
    
    print(f"\n🎯 TOP 50 Requirements Críticos para Implementar:")
    print("=" * 60)
    
    for i, req in enumerate(top_50, 1):
        status_str = f"Level {req['level']}"
        if req['has_test']:
            status_str += " (✓ Test)"
        else:
            status_str += " (No test)"
        
        priority_str = {1: "🔴 CRÍTICO", 2: "🟡 ALTO", 3: "🟢 MEDIO"}.get(req['priority'], "⚫ BAJO")
        
        print(f"{i:2d}. {req['id']:<12} | {priority_str} | {status_str}")
    
    # Resumen por sección
    section_summary = defaultdict(int)
    for req in top_50:
        section_summary[req['section']] += 1
    
    print(f"\n📊 Resumen por sección (Top 50):")
    for section, count in sorted(section_summary.items()):
        print(f"  - Sección {section}: {count} requirements")
    
    # Recomendaciones
    print(f"\n💡 Recomendaciones de Implementación:")
    print(f"  1. Comenzar con Sección 7.2 (File Structure) - Base fundamental")
    print(f"  2. Continuar con Sección 7.5 (Document Structure) - Catálogo y páginas")
    print(f"  3. Implementar Sección 9.2 (Text Objects) - Contenido básico")
    print(f"  4. Agregar Sección 8.6.3 (Color Spaces) - Funcionalidad visual")
    print(f"  5. Desarrollar Sección 9.6 (Fonts) - Tipografía")
    
    implemented_with_tests = sum(1 for req in top_50 if req['level'] > 2 and req['has_test'])
    ready_for_improvement = sum(1 for req in top_50 if req['level'] in [1, 2] and req['has_test'])
    need_implementation = sum(1 for req in top_50 if req['level'] == 0)
    
    print(f"\n📈 Estado Top 50:")
    print(f"  - ✅ Implementados con tests (Level 3+): {implemented_with_tests}")
    print(f"  - 🔧 Listos para mejorar (Level 1-2 + test): {ready_for_improvement}")
    print(f"  - 🚀 Necesitan implementación (Level 0): {need_implementation}")
    
    return top_50

if __name__ == "__main__":
    analyze_current_state()