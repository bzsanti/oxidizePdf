#!/usr/bin/env python3
"""
Crea ISO_VERIFICATION_STATUS.toml inicial con todos los requisitos en nivel 0
ARCHIVO ÚNICO - este será el único archivo que se actualiza con el progreso
"""

import toml
from pathlib import Path
from datetime import datetime
import sys

def load_compliance_matrix(matrix_path):
    """Carga la matriz inmutable para extraer IDs de requisitos"""
    print(f"Cargando matriz inmutable: {matrix_path}")
    with open(matrix_path, 'r', encoding='utf-8') as f:
        matrix = toml.load(f)
    
    total_features = matrix['metadata']['total_features']
    print(f"Total requisitos en matriz: {total_features}")
    return matrix

def extract_all_requirement_ids(matrix):
    """Extrae todos los IDs de requisitos de la matriz"""
    all_ids = []
    
    for section_key, section_data in matrix.items():
        if section_key.startswith('section_') and 'requirements' in section_data:
            for req in section_data['requirements']:
                all_ids.append(req['id'])
    
    print(f"IDs extraídos: {len(all_ids)}")
    return sorted(all_ids)

def create_initial_status(matrix, requirement_ids):
    """Crea estructura inicial del archivo de estado"""
    status_data = {
        'metadata': {
            'last_updated': datetime.now().isoformat(),
            'matrix_version': matrix['metadata']['version'],
            'total_requirements': len(requirement_ids),
            'note': 'Este es el ÚNICO archivo de estado. NO crear copias o duplicados.',
            'warning': 'Solo este archivo se modifica. La matriz ISO_COMPLIANCE_MATRIX.toml es INMUTABLE.'
        }
    }
    
    # Inicializar estado para cada requisito en nivel 0
    print("Inicializando estado de requisitos...")
    for req_id in requirement_ids:
        # Usar formato "status.{id}" para estructura plana en TOML
        status_key = f'status."{req_id}"'
        status_data[status_key] = {
            'level': 0,                    # Nivel 0 = No implementado
            'implementation': '',          # Ruta al código (vacío inicialmente)
            'test_file': '',              # Ruta al test (vacío inicialmente)  
            'verified': False,            # False hasta nivel 3+
            'last_checked': 'never',      # Timestamp de última verificación
            'notes': 'Pendiente de verificación'
        }
    
    # Agregar estadísticas iniciales
    status_data['statistics'] = {
        'level_0_count': len(requirement_ids),  # Todos en nivel 0 inicialmente
        'level_1_count': 0,
        'level_2_count': 0,
        'level_3_count': 0,
        'level_4_count': 0,
        'average_level': 0.0,
        'compliance_percentage': 0.0,
        'last_calculated': datetime.now().isoformat()
    }
    
    return status_data

def save_status_file(status_data, output_path):
    """Guarda el archivo único de estado"""
    print(f"Guardando archivo de estado: {output_path}")
    
    with open(output_path, 'w', encoding='utf-8') as f:
        # Header con advertencias importantes
        f.write("# ISO 32000-1:2008 Verification Status\n")
        f.write("# ARCHIVO ÚNICO DE ESTADO - NO CREAR COPIAS\n")
        f.write("# Este es el ÚNICO archivo que se modifica para tracking\n")
        f.write("# La matriz ISO_COMPLIANCE_MATRIX.toml es INMUTABLE\n")
        f.write(f"# Generado: {datetime.now().isoformat()}\n")
        f.write(f"# Total requisitos: {status_data['metadata']['total_requirements']}\n\n")
        
        toml.dump(status_data, f)
    
    print(f"✓ Archivo de estado creado: {output_path}")

def main():
    # Rutas fijas - SIEMPRE las mismas
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    matrix_path = project_root / "ISO_COMPLIANCE_MATRIX.toml"
    status_path = project_root / "ISO_VERIFICATION_STATUS.toml"
    
    print("=== Creación de Archivo de Estado Inicial ===")
    print(f"Matriz (inmutable): {matrix_path}")
    print(f"Estado (modificable): {status_path}")
    
    if not matrix_path.exists():
        print(f"ERROR: No se encuentra la matriz: {matrix_path}")
        print("Ejecuta primero: python3 scripts/convert_json_to_matrix.py")
        sys.exit(1)
    
    if status_path.exists():
        response = input(f"ADVERTENCIA: {status_path} ya existe. ¿Sobrescribir? (y/N): ")
        if response.lower() != 'y':
            print("Operación cancelada.")
            sys.exit(0)
    
    # Cargar matriz inmutable
    matrix = load_compliance_matrix(matrix_path)
    
    # Extraer IDs de requisitos
    requirement_ids = extract_all_requirement_ids(matrix)
    
    # Crear estado inicial
    print(f"\nCreando estado inicial para {len(requirement_ids)} requisitos...")
    status_data = create_initial_status(matrix, requirement_ids)
    
    # Guardar archivo único de estado
    save_status_file(status_data, status_path)
    
    print(f"\n✓ Archivo de estado inicial creado")
    print(f"✓ Todos los {len(requirement_ids)} requisitos inicializados en nivel 0")
    print(f"✓ Estado guardado en: {status_path}")
    
    print("\nPróximos pasos:")
    print("1. Modificar código Rust para leer ambos archivos")
    print("2. Arreglar tests unitarios")
    print("3. Crear scripts de actualización de estado")

if __name__ == '__main__':
    main()