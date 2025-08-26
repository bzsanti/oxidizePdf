#!/usr/bin/env python3
"""
Script para corregir la estructura del ISO_VERIFICATION_STATUS.toml
Convierte ["status.10.1"] a [status."10.1"] para proper parsing
"""

import re
import sys
from pathlib import Path

def fix_toml_structure(content):
    """
    Convierte la estructura de keys incorrecta a correcta:
    De: ["status.10.1"]
    A:  [status."10.1"]
    """
    # Patrón para encontrar todas las keys de status
    pattern = r'\["status\.([^"]+)"\]'
    
    def replacement(match):
        requirement_id = match.group(1)
        return f'[status."{requirement_id}"]'
    
    fixed_content = re.sub(pattern, replacement, content)
    
    return fixed_content

def main():
    project_root = Path(__file__).parent.parent
    status_file = project_root / "ISO_VERIFICATION_STATUS.toml"
    
    if not status_file.exists():
        print(f"❌ Error: No se encontró {status_file}")
        sys.exit(1)
    
    print(f"📄 Corrigiendo estructura TOML: {status_file}")
    
    # Crear backup
    backup_file = status_file.with_suffix(status_file.suffix + '.structure_backup')
    content = status_file.read_text()
    backup_file.write_text(content)
    print(f"💾 Backup creado: {backup_file}")
    
    # Mostrar muestra de estructura original
    original_keys = re.findall(r'\["status\.[^]]+\]', content)[:3]
    if original_keys:
        print("\n🔍 Estructura original (muestra):")
        for key in original_keys:
            print(f"  {key}")
    
    # Aplicar corrección
    print("\n🔧 Corrigiendo estructura...")
    fixed_content = fix_toml_structure(content)
    
    # Verificar cambios
    if content != fixed_content:
        # Escribir archivo corregido
        status_file.write_text(fixed_content)
        
        # Mostrar muestra corregida
        fixed_keys = re.findall(r'\[status\.[^]]+\]', fixed_content)[:3]
        print("\n✅ Nueva estructura (muestra):")
        for key in fixed_keys:
            print(f"  {key}")
        
        print(f"\n✅ Estructura corregida: {status_file}")
        
        # Validar con Python TOML
        try:
            import toml
            data = toml.loads(fixed_content)
            status_count = len(data.get('status', {}))
            print(f"\n🎯 Validación: {status_count} entradas de status parseadas correctamente")
        except Exception as e:
            print(f"\n❌ Error de validación TOML: {e}")
            # Restaurar backup
            status_file.write_text(content)
            print("🔄 Backup restaurado")
    else:
        print("\n🤔 No se encontraron cambios de estructura necesarios")

if __name__ == "__main__":
    main()