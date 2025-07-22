#!/bin/bash

# PDF Compatibility Verification Script
# This script runs comprehensive PDF analysis using both oxidize-pdf parser and oxidize-pdf-render
# to identify compatibility issues and help improve the parser.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script configuration
FIXTURES_DIR="${1:-tests/fixtures}"
OUTPUT_DIR="compatibility_reports"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BATCH_SIZE="${2:-50}"  # Default batch size is 50

# Print header
echo -e "${BLUE}🔍 PDF Compatibility Verification Tool${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# Check if fixtures directory exists
if [ ! -d "$FIXTURES_DIR" ]; then
    echo -e "${RED}Error: Fixtures directory not found: $FIXTURES_DIR${NC}"
    echo "Usage: $0 [path_to_pdf_directory] [batch_size]"
    echo "Example: $0 tests/fixtures 50"
    exit 1
fi

# Check if oxidize-pdf-render exists
if [ ! -d "../oxidize-pdf-render" ]; then
    echo -e "${YELLOW}Warning: oxidize-pdf-render not found in ../oxidize-pdf-render${NC}"
    echo "Please ensure oxidize-pdf-render is cloned as a sibling directory."
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo -e "${GREEN}📁 PDF Directory: $FIXTURES_DIR${NC}"
echo -e "${GREEN}📊 Output Directory: $OUTPUT_DIR${NC}"
echo -e "${GREEN}📦 Batch Size: $BATCH_SIZE PDFs per batch${NC}"
echo ""

# Function to check dependencies
check_dependencies() {
    echo -e "${BLUE}Checking dependencies...${NC}"
    
    # Check Rust toolchain
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: Rust/Cargo not found. Please install Rust.${NC}"
        exit 1
    fi
    
    # Check Python
    if ! command -v python3 &> /dev/null; then
        echo -e "${RED}Error: Python 3 not found. Please install Python 3.${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ All dependencies found${NC}"
    echo ""
}

# Function to build projects
build_projects() {
    echo -e "${BLUE}Building oxidize-pdf...${NC}"
    cargo build --release --bin oxidizepdf
    
    echo -e "${BLUE}Building oxidize-pdf-render examples...${NC}"
    cd ../oxidize-pdf-render
    cargo build --release --example test_render
    cd - > /dev/null
    
    echo -e "${GREEN}✅ Build completed${NC}"
    echo ""
}

# Function to run Python analysis
run_python_analysis() {
    echo -e "${BLUE}Running Python-based batch analysis...${NC}"
    
    # Use batch processing with configurable batch size
    BATCH_SIZE="${BATCH_SIZE:-50}"
    echo -e "${YELLOW}Using batch size: $BATCH_SIZE PDFs per batch${NC}"
    
    python3 analyze_pdfs_batch.py "$FIXTURES_DIR" --batch-size "$BATCH_SIZE" > "$OUTPUT_DIR/python_analysis_$TIMESTAMP.txt" 2>&1
    
    # Extract key metrics from output
    if [ -f "$OUTPUT_DIR/python_analysis_$TIMESTAMP.txt" ]; then
        echo -e "${GREEN}Python batch analysis completed. Final metrics:${NC}"
        grep -E "(Total PDFs processed:|Both successful:|Parse only:|Render only:|Both failed:)" "$OUTPUT_DIR/python_analysis_$TIMESTAMP.txt" | tail -5
    fi
    echo ""
}

# Function to run Rust analysis
run_rust_analysis() {
    echo -e "${BLUE}Running Rust-based analysis...${NC}"
    
    cargo run --release --example analyze_pdf_with_render > "$OUTPUT_DIR/rust_analysis_$TIMESTAMP.txt" 2>&1
    
    # Extract key metrics from output
    if [ -f "$OUTPUT_DIR/rust_analysis_$TIMESTAMP.txt" ]; then
        echo -e "${GREEN}Rust analysis completed. Key metrics:${NC}"
        grep -E "(Total PDFs:|Both successful:|Parse only:|Render only:|Both failed:)" "$OUTPUT_DIR/rust_analysis_$TIMESTAMP.txt" | tail -5
    fi
    echo ""
}

# Function to compare results
compare_results() {
    echo -e "${BLUE}Comparing analysis results...${NC}"
    
    # Create comparison report
    COMPARISON_FILE="$OUTPUT_DIR/comparison_report_$TIMESTAMP.txt"
    
    echo "PDF Compatibility Analysis Comparison" > "$COMPARISON_FILE"
    echo "=====================================" >> "$COMPARISON_FILE"
    echo "Generated: $(date)" >> "$COMPARISON_FILE"
    echo "" >> "$COMPARISON_FILE"
    
    echo "Python Analysis Results:" >> "$COMPARISON_FILE"
    echo "------------------------" >> "$COMPARISON_FILE"
    if [ -f "$OUTPUT_DIR/python_analysis_$TIMESTAMP.txt" ]; then
        grep -E "(Total PDFs:|successful:|failed:)" "$OUTPUT_DIR/python_analysis_$TIMESTAMP.txt" >> "$COMPARISON_FILE"
    fi
    echo "" >> "$COMPARISON_FILE"
    
    echo "Rust Analysis Results:" >> "$COMPARISON_FILE"
    echo "----------------------" >> "$COMPARISON_FILE"
    if [ -f "$OUTPUT_DIR/rust_analysis_$TIMESTAMP.txt" ]; then
        grep -E "(Total PDFs:|successful:|failed:)" "$OUTPUT_DIR/rust_analysis_$TIMESTAMP.txt" >> "$COMPARISON_FILE"
    fi
    
    echo -e "${GREEN}✅ Comparison report saved to: $COMPARISON_FILE${NC}"
}

# Function to generate summary
generate_summary() {
    echo ""
    echo -e "${BLUE}📊 Analysis Summary${NC}"
    echo -e "${BLUE}==================${NC}"
    
    # Count PDFs
    PDF_COUNT=$(find "$FIXTURES_DIR" -name "*.pdf" | wc -l)
    echo -e "Total PDFs analyzed: ${YELLOW}$PDF_COUNT${NC}"
    
    # Check for JSON report from Python analysis
    JSON_REPORT=$(ls -t pdf_compatibility_report_*.json 2>/dev/null | head -1)
    if [ -n "$JSON_REPORT" ]; then
        echo -e "Detailed JSON report: ${GREEN}$JSON_REPORT${NC}"
        
        # Extract key metrics using Python
        python3 -c "
import json
with open('$JSON_REPORT', 'r') as f:
    data = json.load(f)
    print(f\"Parse success rate: {data['parsing']['success_rate']:.1f}%\")
    print(f\"Render success rate: {data['rendering']['success_rate']:.1f}%\")
    print(f\"Both successful: {data['combined']['both_successful']} PDFs\")
    print(f\"Compatibility issues: {len(data['compatibility_issues'])}\")
"
    fi
    
    echo ""
    echo -e "${BLUE}📁 All reports saved in: $OUTPUT_DIR/${NC}"
    echo ""
    echo -e "${GREEN}✅ Compatibility verification completed!${NC}"
}

# Main execution
main() {
    check_dependencies
    build_projects
    run_python_analysis
    run_rust_analysis
    compare_results
    generate_summary
}

# Run main function
main

# Make reports easily accessible
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "1. Review compatibility issues in: $OUTPUT_DIR/"
echo "2. Check JSON report for detailed error breakdown"
echo "3. Use findings to improve oxidize-pdf parser"
echo ""
echo -e "${BLUE}Batch processing options:${NC}"
echo "- To use different batch size: $0 $FIXTURES_DIR 25"
echo "- To resume interrupted analysis: python3 analyze_pdfs_batch.py $FIXTURES_DIR --resume"
echo ""