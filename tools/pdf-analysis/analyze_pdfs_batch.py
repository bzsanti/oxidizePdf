#!/usr/bin/env python3
"""
Batch PDF analysis script with render validation.
Processes PDFs in configurable batches for better performance and progress tracking.
"""

import os
import sys
import subprocess
import re
import json
import time
import argparse
from collections import defaultdict
from pathlib import Path
from datetime import datetime
import signal

# Global flag for graceful shutdown
interrupted = False

def signal_handler(sig, frame):
    global interrupted
    interrupted = True
    print("\n\n⚠️  Gracefully stopping after current batch...")
    print("Progress will be saved. Use --resume to continue later.")

signal.signal(signal.SIGINT, signal_handler)

def analyze_pdf_parsing(pdf_path):
    """Analyze a single PDF file using oxidize-pdf parser"""
    result = subprocess.run(
        ['cargo', 'run', '--bin', 'oxidizepdf', '--', 'info', str(pdf_path)],
        capture_output=True,
        text=True,
        cwd='.'
    )
    return result

def analyze_pdf_rendering(pdf_path):
    """Analyze a single PDF file using oxidize-pdf-render"""
    output_path = f"/tmp/render_test_{os.path.basename(pdf_path)}.png"
    
    result = subprocess.run(
        ['cargo', 'run', '--manifest-path', '../oxidize-pdf-render/Cargo.toml', 
         '--example', 'test_render', '--', str(pdf_path), output_path],
        capture_output=True,
        text=True,
        timeout=10  # 10 second timeout per PDF
    )
    
    # Clean up output file if it exists
    if os.path.exists(output_path):
        os.remove(output_path)
    
    return result

def categorize_parse_error(stderr):
    """Categorize the parsing error based on stderr output"""
    if "circular reference" in stderr:
        return "PageTreeError: circular reference"
    elif "MissingKey" in stderr:
        match = re.search(r'MissingKey\("([^"]+)"\)', stderr)
        if match:
            key = match.group(1)
            return f"MissingKey: {key}"
        return "MissingKey"
    elif "Invalid header" in stderr or "InvalidHeader" in stderr:
        return "InvalidHeader"
    elif "xref" in stderr.lower() or "XrefError" in stderr:
        if "Invalid xref table" in stderr:
            return "XrefError: Invalid xref table"
        return "XrefError"
    elif "PageCount" in stderr:
        return "PageCount: Other"
    elif "PageTreeError" in stderr:
        return "PageTreeError: Other"
    elif "encrypted" in stderr.lower() or "encryption" in stderr.lower():
        return "EncryptionNotSupported"
    elif "EmptyFile" in stderr:
        return "EmptyFile"
    else:
        return "Other"

def categorize_render_error(stderr):
    """Categorize the rendering error based on stderr output"""
    if "NotImplemented" in stderr:
        return "NotImplemented: Missing feature"
    elif "InvalidColorSpace" in stderr:
        return "InvalidColorSpace"
    elif "InvalidFont" in stderr or "FontError" in stderr:
        return "FontError"
    elif "ImageDecoding" in stderr:
        return "ImageDecoding: Unsupported format"
    elif "ContentStream" in stderr:
        return "ContentStream: Invalid operations"
    elif "circular reference" in stderr:
        return "CircularReference"
    elif "encrypted" in stderr.lower():
        return "EncryptionNotSupported"
    elif "timeout" in stderr.lower():
        return "Timeout"
    else:
        return "Other rendering error"

def save_checkpoint(checkpoint_file, processed_files, stats):
    """Save progress checkpoint for resuming"""
    # Convert Path objects to strings for JSON serialization
    processed_files_str = [str(p) for p in processed_files]
    
    # Convert defaultdicts to regular dicts for JSON
    stats_copy = stats.copy()
    stats_copy['parse_error_types'] = dict(stats['parse_error_types'])
    stats_copy['render_error_types'] = dict(stats['render_error_types'])
    
    checkpoint = {
        'timestamp': datetime.now().isoformat(),
        'processed_files': processed_files_str,
        'stats': stats_copy
    }
    with open(checkpoint_file, 'w') as f:
        json.dump(checkpoint, f, indent=2)

def load_checkpoint(checkpoint_file):
    """Load previous checkpoint if exists"""
    if os.path.exists(checkpoint_file):
        with open(checkpoint_file, 'r') as f:
            return json.load(f)
    return None

def process_batch(pdf_batch, stats, processed_files):
    """Process a batch of PDFs and update statistics"""
    batch_start = time.time()
    
    for pdf_file in pdf_batch:
        if pdf_file in processed_files:
            continue
            
        # Test parsing
        parse_result = analyze_pdf_parsing(pdf_file)
        parse_success = parse_result.returncode == 0
        
        # Test rendering
        render_success = False
        render_error = None
        
        try:
            render_result = analyze_pdf_rendering(pdf_file)
            render_success = render_result.returncode == 0
            if not render_success:
                render_error = categorize_render_error(render_result.stderr)
        except subprocess.TimeoutExpired:
            render_error = "Timeout"
        except Exception as e:
            render_error = f"Exception: {str(e)}"
        
        # Update statistics
        if parse_success:
            stats['parse_successful'] += 1
        else:
            stats['parse_failed'] += 1
            parse_error = categorize_parse_error(parse_result.stderr)
            stats['parse_error_types'][parse_error] += 1
        
        if render_success:
            stats['render_successful'] += 1
        else:
            stats['render_failed'] += 1
            if render_error:
                stats['render_error_types'][render_error] += 1
        
        # Categorize combined results
        if parse_success and render_success:
            stats['both_successful'] += 1
        elif parse_success and not render_success:
            stats['parse_only_success'] += 1
            stats['compatibility_issues'].append({
                'file': pdf_file.name,
                'issue': 'Parses but fails to render',
                'render_error': render_error
            })
        elif not parse_success and render_success:
            stats['render_only_success'] += 1
            stats['compatibility_issues'].append({
                'file': pdf_file.name,
                'issue': 'Renders but fails to parse',
                'parse_error': parse_error if 'parse_error' in locals() else 'Unknown'
            })
        else:
            stats['both_failed'] += 1
        
        processed_files.add(pdf_file)
        stats['total_processed'] += 1
    
    batch_duration = time.time() - batch_start
    return batch_duration

def print_batch_summary(batch_num, batch_size, stats, batch_duration):
    """Print summary after each batch"""
    total = stats['total_processed']
    print(f"\n📊 Batch {batch_num} Summary (PDFs {total-batch_size+1}-{total}):")
    print(f"⏱️  Batch processing time: {batch_duration:.2f}s ({batch_size/batch_duration:.1f} PDFs/sec)")
    
    # Current totals
    print(f"\n📈 Cumulative Results ({total} PDFs processed):")
    if total > 0:
        print(f"  Parsing: {stats['parse_successful']}/{total} ({stats['parse_successful']/total*100:.1f}%)")
        print(f"  Rendering: {stats['render_successful']}/{total} ({stats['render_successful']/total*100:.1f}%)")
        print(f"  Both successful: {stats['both_successful']} ({stats['both_successful']/total*100:.1f}%)")
        print(f"  Parse only: {stats['parse_only_success']} ({stats['parse_only_success']/total*100:.1f}%)")

def main():
    parser = argparse.ArgumentParser(description='Batch PDF analysis with render validation')
    parser.add_argument('pdf_dir', help='Directory containing PDF files')
    parser.add_argument('--batch-size', type=int, default=50, help='Number of PDFs per batch (default: 50)')
    parser.add_argument('--resume', action='store_true', help='Resume from previous checkpoint')
    parser.add_argument('--checkpoint-file', default='pdf_analysis_checkpoint.json', 
                        help='Checkpoint file name (default: pdf_analysis_checkpoint.json)')
    
    args = parser.parse_args()
    
    # Find all PDF files
    pdf_path = Path(args.pdf_dir)
    if not pdf_path.exists():
        print(f"Error: Directory {args.pdf_dir} does not exist")
        return
    
    pdf_files = list(pdf_path.glob("*.pdf"))
    if not pdf_files:
        print(f"No PDF files found in {args.pdf_dir}")
        return
    
    print(f"🔍 Batch PDF Compatibility Analysis")
    print(f"===================================")
    print(f"Total PDFs found: {len(pdf_files)}")
    print(f"Batch size: {args.batch_size}")
    print(f"Checkpoint file: {args.checkpoint_file}")
    
    # Initialize or load checkpoint
    processed_files = set()
    stats = {
        'total_processed': 0,
        'parse_successful': 0,
        'parse_failed': 0,
        'render_successful': 0,
        'render_failed': 0,
        'both_successful': 0,
        'parse_only_success': 0,
        'render_only_success': 0,
        'both_failed': 0,
        'parse_error_types': defaultdict(int),
        'render_error_types': defaultdict(int),
        'compatibility_issues': []
    }
    
    if args.resume:
        checkpoint = load_checkpoint(args.checkpoint_file)
        if checkpoint:
            processed_files = set(Path(f) for f in checkpoint['processed_files'])
            stats = checkpoint['stats']
            # Convert defaultdicts back
            stats['parse_error_types'] = defaultdict(int, stats['parse_error_types'])
            stats['render_error_types'] = defaultdict(int, stats['render_error_types'])
            print(f"\n✅ Resuming from checkpoint: {stats['total_processed']} PDFs already processed")
    
    # Filter out already processed files
    remaining_files = [f for f in pdf_files if f not in processed_files]
    print(f"📋 Remaining PDFs to process: {len(remaining_files)}")
    
    if not remaining_files:
        print("\n✅ All PDFs have been processed!")
        return
    
    # Process in batches
    total_start_time = time.time()
    batch_num = (stats['total_processed'] // args.batch_size) + 1
    
    for i in range(0, len(remaining_files), args.batch_size):
        if interrupted:
            break
            
        batch = remaining_files[i:i + args.batch_size]
        print(f"\n🔄 Processing batch {batch_num} ({len(batch)} PDFs)...")
        
        batch_duration = process_batch(batch, stats, processed_files)
        
        # Print batch summary
        print_batch_summary(batch_num, len(batch), stats, batch_duration)
        
        # Save checkpoint after each batch
        save_checkpoint(args.checkpoint_file, processed_files, stats)
        
        batch_num += 1
    
    # Final report
    total_duration = time.time() - total_start_time
    
    print(f"\n\n{'='*60}")
    print(f"📊 FINAL PDF COMPATIBILITY ANALYSIS REPORT")
    print(f"{'='*60}")
    print(f"\nDirectory: {args.pdf_dir}")
    print(f"Total PDFs processed: {stats['total_processed']}")
    print(f"Total analysis time: {total_duration:.2f}s ({stats['total_processed']/total_duration:.1f} PDFs/sec)")
    
    if stats['total_processed'] > 0:
        print(f"\n📄 Parsing Results (oxidize-pdf):")
        print(f"  ✅ Successful: {stats['parse_successful']} ({stats['parse_successful']/stats['total_processed']*100:.1f}%)")
        print(f"  ❌ Failed: {stats['parse_failed']} ({stats['parse_failed']/stats['total_processed']*100:.1f}%)")
        
        print(f"\n🎨 Rendering Results (oxidize-pdf-render):")
        print(f"  ✅ Successful: {stats['render_successful']} ({stats['render_successful']/stats['total_processed']*100:.1f}%)")
        print(f"  ❌ Failed: {stats['render_failed']} ({stats['render_failed']/stats['total_processed']*100:.1f}%)")
        
        print(f"\n🔄 Combined Analysis:")
        print(f"  ✅✅ Both successful: {stats['both_successful']} ({stats['both_successful']/stats['total_processed']*100:.1f}%)")
        print(f"  ✅❌ Parse only: {stats['parse_only_success']} ({stats['parse_only_success']/stats['total_processed']*100:.1f}%)")
        print(f"  ❌✅ Render only: {stats['render_only_success']} ({stats['render_only_success']/stats['total_processed']*100:.1f}%)")
        print(f"  ❌❌ Both failed: {stats['both_failed']} ({stats['both_failed']/stats['total_processed']*100:.1f}%)")
    
    # Save final report
    report_filename = f"pdf_batch_analysis_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    final_report = {
        'analysis_date': datetime.now().isoformat(),
        'total_pdfs': stats['total_processed'],
        'duration_seconds': total_duration,
        'parsing': {
            'successful': stats['parse_successful'],
            'failed': stats['parse_failed'],
            'success_rate': stats['parse_successful']/stats['total_processed']*100 if stats['total_processed'] > 0 else 0
        },
        'rendering': {
            'successful': stats['render_successful'],
            'failed': stats['render_failed'],
            'success_rate': stats['render_successful']/stats['total_processed']*100 if stats['total_processed'] > 0 else 0
        },
        'combined': {
            'both_successful': stats['both_successful'],
            'parse_only': stats['parse_only_success'],
            'render_only': stats['render_only_success'],
            'both_failed': stats['both_failed']
        },
        'parse_errors': dict(stats['parse_error_types']),
        'render_errors': dict(stats['render_error_types']),
        'compatibility_issues': stats['compatibility_issues'][:100]  # Save top 100
    }
    
    with open(report_filename, 'w') as f:
        json.dump(final_report, f, indent=2)
    
    print(f"\n💾 Final report saved to: {report_filename}")
    
    if interrupted:
        print(f"\n⚠️  Analysis was interrupted. Use --resume to continue from checkpoint.")
    else:
        print(f"\n✅ Analysis completed successfully!")
        # Clean up checkpoint file
        if os.path.exists(args.checkpoint_file):
            os.remove(args.checkpoint_file)

if __name__ == "__main__":
    main()