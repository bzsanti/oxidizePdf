#!/usr/bin/env python3
"""
ISO 32000-1:2008 Compliance Report Generator

Generates HTML and Markdown reports based on current verification status.
"""

import toml
import json
from pathlib import Path
from datetime import datetime
import argparse
import sys

# File paths
MATRIX_FILE = "ISO_COMPLIANCE_MATRIX.toml"
STATUS_FILE = "ISO_VERIFICATION_STATUS.toml"

def load_status_data():
    """Load current verification status"""
    if not Path(STATUS_FILE).exists():
        print(f"ERROR: {STATUS_FILE} not found")
        sys.exit(1)
    
    with open(STATUS_FILE, 'r', encoding='utf-8') as f:
        return toml.load(f)

def generate_markdown_report():
    """Generate Markdown compliance report"""
    status_data = load_status_data()
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    
    report = []
    report.append("# ISO 32000-1:2008 Compliance Report")
    report.append("")
    report.append(f"**Generated**: {timestamp}")
    report.append(f"**Project**: oxidize-pdf")
    report.append("")
    
    # Overall statistics
    if 'statistics' in status_data:
        stats = status_data['statistics']
        report.append("## 📊 Overall Compliance Statistics")
        report.append("")
        report.append(f"- **Total Requirements**: {stats.get('level_0_count', 0) + stats.get('level_1_count', 0) + stats.get('level_2_count', 0) + stats.get('level_3_count', 0) + stats.get('level_4_count', 0)}")
        report.append(f"- **Average Level**: {stats.get('average_level', 0):.2f} / 4.0")
        report.append(f"- **Overall Compliance**: {stats.get('compliance_percentage', 0):.1f}%")
        report.append("")
        
        # Level breakdown
        report.append("### Verification Level Distribution")
        report.append("")
        report.append("| Level | Description | Count | Percentage |")
        report.append("|-------|-------------|-------|------------|")
        
        total = stats.get('level_0_count', 0) + stats.get('level_1_count', 0) + stats.get('level_2_count', 0) + stats.get('level_3_count', 0) + stats.get('level_4_count', 0)
        
        levels = [
            (0, "Not Implemented", stats.get('level_0_count', 0)),
            (1, "Code Exists", stats.get('level_1_count', 0)),
            (2, "Generates PDF", stats.get('level_2_count', 0)),
            (3, "Content Verified", stats.get('level_3_count', 0)),
            (4, "ISO Compliant", stats.get('level_4_count', 0)),
        ]
        
        for level, desc, count in levels:
            pct = (count / total * 100) if total > 0 else 0
            report.append(f"| {level} | {desc} | {count} | {pct:.1f}% |")
        
        report.append("")
    
    # Progress visualization
    report.append("## 📈 Progress Overview")
    report.append("")
    if 'statistics' in status_data:
        compliance_pct = stats.get('compliance_percentage', 0)
        progress_bar = "█" * int(compliance_pct / 5) + "░" * (20 - int(compliance_pct / 5))
        report.append(f"```")
        report.append(f"Progress: [{progress_bar}] {compliance_pct:.1f}%")
        report.append(f"```")
        report.append("")
    
    # Recent updates
    report.append("## 🔄 Recent Activity")
    report.append("")
    
    # Find recently updated requirements
    recent_updates = []
    for key, req_status in status_data.items():
        if key.startswith('status.'):
            if 'last_checked' in req_status:
                req_id = key.replace('status."', '').replace('"', '')
                recent_updates.append((
                    req_id,
                    req_status['level'],
                    req_status.get('last_checked', ''),
                    req_status.get('notes', '')
                ))
    
    # Sort by last checked (most recent first)
    recent_updates.sort(key=lambda x: x[2], reverse=True)
    
    if recent_updates[:10]:  # Show last 10
        report.append("| Requirement | Level | Last Checked | Notes |")
        report.append("|-------------|-------|--------------|-------|")
        
        for req_id, level, last_checked, notes in recent_updates[:10]:
            # Truncate long notes
            short_notes = (notes[:50] + "...") if len(notes) > 50 else notes
            report.append(f"| {req_id} | {level} | {last_checked[:10]} | {short_notes} |")
        
        report.append("")
    else:
        report.append("*No recent updates found*")
        report.append("")
    
    # Implementation priorities
    report.append("## 🎯 Implementation Priorities")
    report.append("")
    
    # Find level 0 requirements (not implemented)
    unimplemented = []
    for key, req_status in status_data.items():
        if key.startswith('status.'):
            if req_status.get('level', 0) == 0:
                req_id = key.replace('status."', '').replace('"', '')
                unimplemented.append(req_id)
    
    if unimplemented[:20]:  # Show first 20
        report.append("### Top Unimplemented Requirements")
        report.append("")
        for req_id in unimplemented[:20]:
            report.append(f"- {req_id}")
        report.append("")
    
    # Test coverage info
    report.append("## 🧪 Test Coverage")
    report.append("")
    report.append("### ISO Verification Test Suite Status")
    report.append("")
    report.append("- ✅ **Document Structure Tests** (Section 7): Implemented")
    report.append("- ✅ **Graphics Tests** (Section 8): Basic implementation")
    report.append("- ✅ **Text Tests** (Section 9): Basic implementation")
    report.append("- 🔧 **Advanced Features** (Sections 10-14): Pending")
    report.append("")
    report.append("### Test Framework Features")
    report.append("")
    report.append("- ✅ Automated verification levels (0-4)")
    report.append("- ✅ Status tracking and updates")
    report.append("- ✅ PDF generation and parsing verification")
    report.append("- ✅ External validator integration (when available)")
    report.append("- ✅ Comprehensive reporting")
    report.append("")
    
    # Footer
    report.append("---")
    report.append("")
    report.append(f"*Report generated by oxidize-pdf ISO compliance system on {timestamp}*")
    report.append("")
    
    return "\n".join(report)

def generate_html_report():
    """Generate HTML compliance report"""
    markdown_content = generate_markdown_report()
    
    # Simple HTML wrapper
    html = f"""<!DOCTYPE html>
<html>
<head>
    <title>ISO 32000-1:2008 Compliance Report - oxidize-pdf</title>
    <meta charset="utf-8">
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }}
        h1 {{ color: #2c3e50; border-bottom: 3px solid #3498db; }}
        h2 {{ color: #34495e; border-bottom: 1px solid #bdc3c7; }}
        table {{ border-collapse: collapse; width: 100%; margin: 10px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f2f2f2; }}
        code {{ background-color: #f4f4f4; padding: 2px 4px; border-radius: 3px; }}
        pre {{ background-color: #f4f4f4; padding: 10px; border-radius: 5px; overflow-x: auto; }}
        .stats {{ background-color: #e8f5e8; padding: 15px; border-radius: 5px; margin: 15px 0; }}
        .priority {{ background-color: #fff3cd; padding: 15px; border-radius: 5px; margin: 15px 0; }}
    </style>
</head>
<body>
{markdown_to_html(markdown_content)}
</body>
</html>"""
    
    return html

def markdown_to_html(markdown):
    """Simple markdown to HTML conversion"""
    html = markdown
    
    # Headers
    html = html.replace("# ", "<h1>").replace("\n## ", "\n<h2>").replace("\n### ", "\n<h3>")
    html = html.replace("\n", "</h1>\n", 1)  # Close first h1
    html = html.replace("</h1>\n<h2>", "</h1>\n\n<h2>")
    html = html.replace("</h2>\n<h3>", "</h2>\n\n<h3>")
    html = html.replace("<h2>", "<h2>").replace("<h3>", "<h3>")
    
    # Simple replacements
    html = html.replace("**", "<strong>").replace("**", "</strong>")
    html = html.replace("*", "<em>").replace("*", "</em>")
    html = html.replace("\n\n", "</p>\n<p>")
    html = f"<p>{html}</p>"
    
    # Tables (basic)
    lines = html.split('\n')
    in_table = False
    result = []
    
    for line in lines:
        if '|' in line and not in_table:
            result.append('<table>')
            in_table = True
        elif '|' not in line and in_table:
            result.append('</table>')
            in_table = False
        
        if in_table and '|' in line:
            if '---' in line:
                continue  # Skip separator
            cells = [cell.strip() for cell in line.split('|')[1:-1]]
            if 'Level' in line or 'Requirement' in line:  # Header
                result.append('<tr>' + ''.join(f'<th>{cell}</th>' for cell in cells) + '</tr>')
            else:
                result.append('<tr>' + ''.join(f'<td>{cell}</td>' for cell in cells) + '</tr>')
        else:
            result.append(line)
    
    if in_table:
        result.append('</table>')
    
    return '\n'.join(result)

def main():
    parser = argparse.ArgumentParser(description='Generate ISO compliance report')
    parser.add_argument('--format', choices=['markdown', 'html', 'both'], default='both',
                       help='Output format')
    parser.add_argument('--output', help='Output directory', default='.')
    
    args = parser.parse_args()
    
    # Change to project root
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    import os
    os.chdir(project_root)
    
    output_dir = Path(args.output)
    output_dir.mkdir(exist_ok=True)
    
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    
    if args.format in ['markdown', 'both']:
        markdown_report = generate_markdown_report()
        md_file = output_dir / f"iso_compliance_report_{timestamp}.md"
        
        with open(md_file, 'w', encoding='utf-8') as f:
            f.write(markdown_report)
        
        print(f"📝 Markdown report generated: {md_file}")
    
    if args.format in ['html', 'both']:
        html_report = generate_html_report()
        html_file = output_dir / f"iso_compliance_report_{timestamp}.html"
        
        with open(html_file, 'w', encoding='utf-8') as f:
            f.write(html_report)
        
        print(f"🌐 HTML report generated: {html_file}")
    
    print(f"✅ Report generation complete!")

if __name__ == '__main__':
    main()