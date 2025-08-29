#!/usr/bin/env python3
"""
Convierte ISO_REQUIREMENTS_MASTER.json a ISO_COMPLIANCE_MATRIX.toml
Matriz INMUTABLE - solo definiciones, sin estado de verificación.
"""

import json
import toml
from pathlib import Path
from datetime import datetime
import sys

def load_json_requirements(json_path):
    """Carga el archivo JSON master"""
    print(f"Cargando {json_path}...")
    with open(json_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    print(f"Total requisitos: {data['metadata']['total_requirements']}")
    return data

def convert_section_to_toml(section_name, section_data, toml_data):
    """Convierte una sección del JSON a formato TOML"""
    section_key = section_name.replace('section_', 'section_')
    
    # Extraer información de la sección
    section_info = {
        'name': section_name.split('_')[2].title() if len(section_name.split('_')) > 2 else 'Unknown',
        'iso_section': section_name.split('_')[1] if len(section_name.split('_')) > 1 else 'Unknown',
        'total_requirements': len(section_data)
    }
    
    # Mapear nombres de secciones más descriptivos
    section_names = {
        'syntax': 'Document Syntax',
        'graphics': 'Graphics', 
        'text': 'Text',
        'rendering': 'Rendering',
        'transparency': 'Transparency',
        'interactive': 'Interactive Features',
        'multimedia': 'Multimedia Features',
        'interchange': 'Document Interchange'
    }
    
    section_suffix = section_name.split('_')[-1] if '_' in section_name else section_name
    section_info['name'] = section_names.get(section_suffix, section_suffix.title())
    
    toml_data[section_key] = section_info
    
    # Convertir requisitos
    requirements = []
    for req in section_data:
        requirement = {
            'id': req['id'],
            'name': f"Requirement {req['id']}",  # Nombre básico
            'description': req['text'][:200] + '...' if len(req['text']) > 200 else req['text'],
            'iso_reference': f"Page {req['page']}, Section {req['id']}",
            'requirement_type': req['requirement_type'],
            'page': req['page'],
            'original_text': req['text']  # Texto completo original
        }
        
        # Limpiar caracteres problemáticos para TOML
        requirement['description'] = requirement['description'].replace('\n', ' ').replace('\r', '')
        requirement['original_text'] = requirement['original_text'].replace('\n', ' ').replace('\r', '')
        
        requirements.append(requirement)
    
    toml_data[section_key]['requirements'] = requirements
    print(f"  Sección {section_key}: {len(requirements)} requisitos")

def create_compliance_matrix_toml(json_data):
    """Crea la estructura TOML de la matriz de cumplimiento"""
    toml_data = {
        'metadata': {
            'version': datetime.now().strftime('%Y-%m-%d'),
            'total_features': json_data['metadata']['total_requirements'],
            'specification': 'ISO 32000-1:2008',
            'methodology': 'docs/ISO_TESTING_METHODOLOGY.md',
            'source_file': 'ISO_REQUIREMENTS_MASTER.json',
            'extraction_date': json_data['metadata']['extraction_date'],
            'immutable': True,  # Marca este archivo como inmutable
            'note': 'Este archivo NO debe modificarse. El estado de verificación va en ISO_VERIFICATION_STATUS.toml'
        }
    }
    
    # Convertir secciones
    print("\nConvirtiendo secciones:")
    for section_name, section_data in json_data['sections'].items():
        convert_section_to_toml(section_name, section_data, toml_data)
    
    # Agregar resumen overall
    toml_data['overall_summary'] = {
        'total_sections': len(json_data['sections']),
        'total_requirements': json_data['metadata']['total_requirements'],
        'by_type': json_data['summary']['by_type'],
        'by_section': json_data['summary']['by_section']
    }
    
    # Herramientas de validación
    toml_data['validation_tools'] = {
        'external_validators': ['qpdf', 'verapdf', 'pdftk'],
        'internal_parser': True,
        'reference_pdfs': False,
        'automated_testing': True
    }
    
    return toml_data

def save_toml_file(toml_data, output_path):
    """Guarda el archivo TOML"""
    print(f"\nGuardando {output_path}...")
    
    # Crear directorio si no existe
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w', encoding='utf-8') as f:
        # Escribir metadata al principio
        f.write("# ISO 32000-1:2008 Compliance Matrix\n")
        f.write("# ARCHIVO INMUTABLE - NO MODIFICAR\n")
        f.write("# El estado de verificación va en ISO_VERIFICATION_STATUS.toml\n")
        f.write(f"# Generado: {datetime.now().isoformat()}\n\n")
        
        toml.dump(toml_data, f)
    
    print(f"✓ Archivo guardado: {output_path}")

def main():
    # Rutas de archivos
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    json_path = project_root / "ISO_REQUIREMENTS_MASTER.json"
    toml_path = project_root / "ISO_COMPLIANCE_MATRIX.toml"
    
    print("=== Conversión JSON a TOML para Matriz ISO ===")
    print(f"Input: {json_path}")
    print(f"Output: {toml_path}")
    
    if not json_path.exists():
        print(f"ERROR: No se encuentra {json_path}")
        sys.exit(1)
    
    # Cargar JSON
    json_data = load_json_requirements(json_path)
    
    # Convertir a TOML
    print("\nConvirtiendo estructura JSON a TOML...")
    toml_data = create_compliance_matrix_toml(json_data)
    
    # Guardar TOML
    save_toml_file(toml_data, toml_path)
    
    print(f"\n✓ Conversión completada")
    print(f"✓ Matriz inmutable creada: {toml_path}")
    print(f"✓ Total requisitos: {toml_data['metadata']['total_features']}")
    print(f"✓ Total secciones: {toml_data['overall_summary']['total_sections']}")
    
    print("\nPróximo paso: Crear ISO_VERIFICATION_STATUS.toml con todos los requisitos en nivel 0")

if __name__ == '__main__':
    main()