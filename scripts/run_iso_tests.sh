#!/bin/bash
# Script de ejecución completa de tests ISO
# Ejecuta todos los tests de verificación ISO y genera reportes

set -e

echo "🚀 Iniciando ejecución completa de tests ISO"
echo "=============================================="

# Cambiar al directorio del proyecto
cd "$(dirname "$0")/.."

echo "📍 Directorio de trabajo: $(pwd)"

# Ejecutar tests de verificación ISO
echo ""
echo "🧪 Ejecutando tests de verificación ISO..."
cargo test --test iso_document_catalog_tests -- --nocapture

# Verificar estadísticas actualizadas
echo ""
echo "📊 Estadísticas de cumplimiento actualizadas:"
python3 scripts/update_verification_status.py --stats

# Generar reporte de cumplimiento
echo ""
echo "📝 Generando reporte de cumplimiento..."
python3 scripts/generate_compliance_report.py --format both --output reports/

# Mostrar algunos requisitos actualizados
echo ""
echo "🔍 Estado de requisitos clave:"
echo "Requisito 7.687 (Catalog Type):"
python3 scripts/update_verification_status.py --show 7.687

echo ""
echo "✅ Ejecución completa de tests ISO finalizada"
echo "📂 Reportes generados en: reports/"
echo "🎯 Revisa las estadísticas arriba para ver el progreso"