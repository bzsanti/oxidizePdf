#!/usr/bin/env python3
"""
Script para actualizar el estado de verificación de requisitos ISO.
IMPORTANTE: Solo modifica ISO_VERIFICATION_STATUS.toml, NUNCA la matriz.
"""

import toml
from pathlib import Path
from datetime import datetime
import sys
import argparse

# ARCHIVOS ÚNICOS - SIEMPRE los mismos (desde el directorio raíz del proyecto)
import os
script_dir = Path(__file__).parent
project_root = script_dir.parent
MATRIX_FILE = project_root / "ISO_COMPLIANCE_MATRIX.toml"  # READ ONLY
STATUS_FILE = project_root / "ISO_VERIFICATION_STATUS.toml"  # READ/WRITE ONLY

def load_current_status():
    """Carga el estado actual de verificación"""
    if not Path(STATUS_FILE).exists():
        print(f"ERROR: No se encuentra {STATUS_FILE}")
        sys.exit(1)
    
    with open(STATUS_FILE, 'r', encoding='utf-8') as f:
        return toml.load(f)

def update_requirement_status(req_id, level, implementation="", test_file="", notes=""):
    """Actualiza el estado de un requisito específico"""
    print(f"📝 Actualizando requisito {req_id} a nivel {level}")
    
    status_data = load_current_status()
    
    # Validar nivel
    if level not in [0, 1, 2, 3, 4]:
        print(f"ERROR: Nivel {level} inválido. Debe ser 0-4")
        return False
    
    # Actualizar estado del requisito  
    status_key = f'status.\\\\\\\\\\\\\\"{req_id}\\\\\\\\\\\\\\"'
    
    if status_key not in status_data:
        print(f"ERROR: Requisito {req_id} no encontrado en el estado")
        return False
    
    status_data[status_key]['level'] = level
    status_data[status_key]['verified'] = level >= 3
    status_data[status_key]['last_checked'] = datetime.now().isoformat()
    
    if implementation:
        status_data[status_key]['implementation'] = implementation
    if test_file:
        status_data[status_key]['test_file'] = test_file
    if notes:
        status_data[status_key]['notes'] = notes
    
    # Actualizar metadata
    status_data['metadata']['last_updated'] = datetime.now().isoformat()
    
    # Recalcular estadísticas
    recalculate_statistics(status_data)
    
    # Guardar EN EL MISMO ARCHIVO
    with open(STATUS_FILE, 'w', encoding='utf-8') as f:
        f.write("# ISO 32000-1:2008 Verification Status\n")
        f.write("# ARCHIVO ÚNICO DE ESTADO - NO CREAR COPIAS\n")
        f.write("# Este es el ÚNICO archivo que se modifica para tracking\n")
        f.write("# La matriz ISO_COMPLIANCE_MATRIX.toml es INMUTABLE\n")
        f.write(f"# Actualizado: {datetime.now().isoformat()}\n\n")
        
        toml.dump(status_data, f)
    
    print(f"✅ Requisito {req_id} actualizado a nivel {level}")
    return True

def recalculate_statistics(status_data):
    """Recalcula las estadísticas basadas en el estado actual"""
    level_counts = [0, 0, 0, 0, 0]  # levels 0-4
    total_level = 0
    total_requirements = 0
    
    for key, req_status in status_data.items():
        if key.startswith('status.'):
            level = req_status['level']
            if 0 <= level <= 4:
                level_counts[level] += 1
                total_level += level
                total_requirements += 1
    
    average_level = total_level / total_requirements if total_requirements > 0 else 0.0
    
    status_data['statistics'] = {
        'level_0_count': level_counts[0],
        'level_1_count': level_counts[1],
        'level_2_count': level_counts[2],
        'level_3_count': level_counts[3],
        'level_4_count': level_counts[4],
        'average_level': round(average_level, 3),
        'compliance_percentage': round((average_level / 4.0) * 100.0, 2),
        'last_calculated': datetime.now().isoformat(),
    }

def show_requirement_status(req_id):
    """Muestra el estado actual de un requisito"""
    status_data = load_current_status()
    status_key = f'status.\\\\\\\\\\\\\\"{req_id}\\\\\\\\\\\\\\"'
    
    # Debug: mostrar claves disponibles que contengan el ID
    matching_keys = [k for k in status_data.keys() if req_id in k]
    if matching_keys:
        print(f"🔍 Claves que contienen '{req_id}': {matching_keys[:3]}...")
    
    if status_key not in status_data:
        print(f"❌ Requisito {req_id} no encontrado")
        print(f"🔍 Buscando clave: {status_key}")
        # Buscar claves similares
        similar_keys = [k for k in status_data.keys() if k.startswith('status.')][:5]
        print(f"🔍 Primeras claves de estado: {similar_keys}")
        return
    
    req_status = status_data[status_key]
    print(f"\n📋 Estado de requisito {req_id}:")
    print(f"   Nivel: {req_status['level']}")
    print(f"   Verificado: {req_status['verified']}")
    print(f"   Implementación: {req_status['implementation'] or 'N/A'}")
    print(f"   Test: {req_status['test_file'] or 'N/A'}")
    print(f"   Última verificación: {req_status['last_checked']}")
    print(f"   Notas: {req_status['notes']}")

def show_overall_statistics():
    """Muestra estadísticas generales"""
    status_data = load_current_status()
    stats = status_data['statistics']
    
    print("\n📊 Estadísticas de Cumplimiento ISO")
    print("=" * 40)
    print(f"Total requisitos: {stats['level_0_count'] + stats['level_1_count'] + stats['level_2_count'] + stats['level_3_count'] + stats['level_4_count']}")
    print(f"Nivel 0 (No impl.): {stats['level_0_count']}")
    print(f"Nivel 1 (Código): {stats['level_1_count']}")
    print(f"Nivel 2 (PDF): {stats['level_2_count']}")
    print(f"Nivel 3 (Verificado): {stats['level_3_count']}")
    print(f"Nivel 4 (ISO compliant): {stats['level_4_count']}")
    print(f"Nivel promedio: {stats['average_level']}")
    print(f"Cumplimiento: {stats['compliance_percentage']}%")
    print(f"Última actualización: {stats['last_calculated']}")

def main():
    parser = argparse.ArgumentParser(description='Actualizar estado de verificación ISO')
    parser.add_argument('--req-id', help='ID del requisito (ej: 7.5.2.1)')
    parser.add_argument('--level', type=int, choices=[0,1,2,3,4], help='Nuevo nivel (0-4)')
    parser.add_argument('--implementation', help='Ruta al código de implementación')
    parser.add_argument('--test-file', help='Ruta al archivo de test')
    parser.add_argument('--notes', help='Notas adicionales')
    parser.add_argument('--show', help='Mostrar estado de un requisito')
    parser.add_argument('--stats', action='store_true', help='Mostrar estadísticas generales')
    
    args = parser.parse_args()
    
    # Verificar que el archivo de estado existe
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    
    # Cambiar al directorio del proyecto
    import os
    os.chdir(project_root)
    
    if args.stats:
        show_overall_statistics()
    elif args.show:
        show_requirement_status(args.show)
    elif args.req_id and args.level is not None:
        update_requirement_status(
            args.req_id, 
            args.level,
            args.implementation or "",
            args.test_file or "",
            args.notes or ""
        )
    else:
        print("Uso:")
        print("  python3 scripts/update_verification_status.py --req-id 7.5.2.1 --level 3")
        print("  python3 scripts/update_verification_status.py --show 7.5.2.1")
        print("  python3 scripts/update_verification_status.py --stats")
        
        # Mostrar ejemplos
        print("\nEjemplos:")
        print("  # Marcar requisito como implementado con código")
        print("  python3 scripts/update_verification_status.py --req-id 7.5.2.1 --level 1 --implementation src/document.rs:156")
        print("\n  # Marcar como verificado con test")
        print("  python3 scripts/update_verification_status.py --req-id 7.5.2.1 --level 3 --test-file tests/iso_verification/test_catalog.rs")
        print("\n  # Ver estado actual")
        print("  python3 scripts/update_verification_status.py --show 7.5.2.1")

if __name__ == '__main__':
    main()