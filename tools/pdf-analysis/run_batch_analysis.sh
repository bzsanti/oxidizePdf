#!/bin/bash

# Colors
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🚀 Starting batch PDF analysis from checkpoint...${NC}"
echo ""

# Function to monitor progress
monitor_progress() {
    while true; do
        if [ -f pdf_analysis_checkpoint.json ]; then
            PROCESSED=$(grep -o '"total_processed": [0-9]*' pdf_analysis_checkpoint.json | grep -o '[0-9]*' | tail -1)
            if [ -n "$PROCESSED" ]; then
                PERCENTAGE=$(echo "scale=1; $PROCESSED * 100 / 749" | bc)
                echo -ne "\r${GREEN}Progress: $PROCESSED/749 PDFs ($PERCENTAGE%)${NC}   "
            fi
        fi
        sleep 5
    done
}

# Start monitoring in background
monitor_progress &
MONITOR_PID=$!

# Run the analysis
python3 analyze_pdfs_batch.py tests/fixtures/ --batch-size 50 --resume

# Kill the monitor
kill $MONITOR_PID 2>/dev/null

echo ""
echo -e "${GREEN}✅ Analysis completed!${NC}"

# Show final statistics if report exists
REPORT=$(ls -t pdf_batch_analysis_*.json 2>/dev/null | head -1)
if [ -n "$REPORT" ]; then
    echo ""
    echo -e "${BLUE}📊 Final Results:${NC}"
    python3 -c "
import json
with open('$REPORT', 'r') as f:
    data = json.load(f)
    print(f\"Total PDFs: {data['total_pdfs']}\")
    print(f\"Parse success: {data['parsing']['successful']} ({data['parsing']['success_rate']:.1f}%)\")
    print(f\"Render success: {data['rendering']['successful']} ({data['rendering']['success_rate']:.1f}%)\")
    print(f\"Both successful: {data['combined']['both_successful']}\")
    print(f\"Compatibility issues: {len(data['compatibility_issues'])}\")
"
fi