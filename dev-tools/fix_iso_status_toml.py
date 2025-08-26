#!/usr/bin/env python3
"""
Script para reparar el archivo ISO_VERIFICATION_STATUS.toml corrupto.
Corrige las keys con múltiples niveles de escaping.
"""

import re
import sys
from pathlib import Path

def fix_toml_keys(content):
    """
    Arregla las keys corruptas con múltiples escapes.
    Convierte: ["status.\\\\\\\\\\\\\\\"10.1\\\\\\\\\\\\\\\""]
    A: ["status.10.1"]
    """
    # Encontrar todas las keys corruptas y extraer solo el número/identificador
    pattern = r'\["status\.[\\\"]*([0-9.]+)[\\\"]*"\]'
    
    def replacement(match):
        # Extraer solo el identificador limpio
        clean_id = match.group(1)
        return f'["status.{clean_id}"]'
    
    # Aplicar la corrección
    fixed_content = re.sub(pattern, replacement, content)
    
    # Patrón alternativo para casos más complejos
    alt_pattern = r'\["status\..*?([0-9]+(?:\.[0-9]+)*).*?"\]'
    lines = fixed_content.split('\n')
    
    for i, line in enumerate(lines):
        if '"status.' in line and '\\' in line:
            # Buscar números en la línea
            numbers = re.findall(r'\b([0-9]+(?:\.[0-9]+)*)\b', line)
            if numbers:
                # Usar el primer número encontrado como ID
                clean_id = numbers[0]
                lines[i] = f'["status.{clean_id}"]'
    
    return '\n'.join(lines)

def backup_file(filepath):
    """Crear backup del archivo original"""
    backup_path = filepath.with_suffix(filepath.suffix + '.backup')
    backup_path.write_text(filepath.read_text())
    return backup_path

def main():
    # Buscar archivo ISO_VERIFICATION_STATUS.toml
    project_root = Path(__file__).parent.parent
    status_file = project_root / "ISO_VERIFICATION_STATUS.toml"
    
    if not status_file.exists():
        print(f"❌ Error: No se encontró {status_file}")
        sys.exit(1)
    
    print(f"📄 Procesando: {status_file}")
    
    # Crear backup
    backup_path = backup_file(status_file)
    print(f"💾 Backup creado: {backup_path}")
    
    # Leer contenido
    content = status_file.read_text()
    
    # Mostrar muestra de keys corruptas
    corrupted_keys = re.findall(r'\["status\.[^]]+\]', content)[:5]
    if corrupted_keys:
        print("\n🔍 Keys corruptas encontradas (muestra):")
        for key in corrupted_keys:
            print(f"  {key}")
    
    # Aplicar correcciones
    print("\n🔧 Aplicando correcciones...")
    fixed_content = fix_toml_keys(content)
    
    # Verificar si se hicieron cambios
    changes_made = content != fixed_content
    
    if changes_made:
        # Escribir archivo corregido
        status_file.write_text(fixed_content)
        
        # Mostrar muestra de keys corregidas
        fixed_keys = re.findall(r'\["status\.[^]]+\]', fixed_content)[:5]
        print("\n✅ Keys corregidas (muestra):")
        for key in fixed_keys:
            print(f"  {key}")
        
        print(f"\n✅ Archivo reparado exitosamente: {status_file}")
    else:
        print("\n🤔 No se encontraron keys corruptas para arreglar")
    
    # Contar total de entradas
    status_entries = len(re.findall(r'\["status\.[^]]+\]', fixed_content))
    print(f"\n📊 Total de entradas de estado: {status_entries}")

if __name__ == "__main__":
    main()