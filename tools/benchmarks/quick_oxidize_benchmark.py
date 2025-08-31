#!/usr/bin/env python3
"""
Quick oxidize-pdf performance benchmark
Measures actual performance of our current implementation
"""

import subprocess
import time
import os
import json
from pathlib import Path

def run_timed_command(cmd, description="Command", timeout=60):
    """Run a command and measure execution time"""
    print(f"Running: {description}")
    
    start_time = time.time()
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=timeout)
        end_time = time.time()
        
        execution_time = (end_time - start_time) * 1000  # Convert to ms
        success = result.returncode == 0
        
        return {
            'description': description,
            'command': cmd,
            'success': success,
            'execution_time_ms': execution_time,
            'stdout': result.stdout,
            'stderr': result.stderr if not success else None
        }
    except subprocess.TimeoutExpired:
        return {
            'description': description,
            'command': cmd,
            'success': False,
            'execution_time_ms': timeout * 1000,
            'error': f'Timeout after {timeout} seconds'
        }
    except Exception as e:
        return {
            'description': description,
            'command': cmd,
            'success': False,
            'execution_time_ms': 0,
            'error': str(e)
        }

def main():
    os.chdir('../../')  # Go to project root
    
    print("Quick oxidize-pdf Performance Benchmark")
    print("=" * 50)
    
    # Test configurations
    tests = [
        {
            'name': 'font_spacing_test',
            'cmd': './target/release/examples/font_spacing_test',
            'build_cmd': 'cargo build --release --example font_spacing_test',
            'expected_output': 'examples/results/font_spacing_test.pdf'
        },
        {
            'name': 'charts_comprehensive_test', 
            'cmd': './target/release/examples/charts_comprehensive_test',
            'build_cmd': 'cargo build --release --example charts_comprehensive_test',
            'expected_output': 'examples/results/charts_comprehensive_test.pdf'
        }
    ]
    
    results = []
    
    # Ensure results directory exists
    os.makedirs('examples/results', exist_ok=True)
    
    for test in tests:
        print(f"\n--- Testing {test['name']} ---")
        
        # Build first
        build_result = run_timed_command(test['build_cmd'], f"Building {test['name']}")
        
        if not build_result['success']:
            print(f"❌ Build failed: {build_result.get('error', 'Unknown error')}")
            continue
            
        print(f"✅ Build successful ({build_result['execution_time_ms']:.0f}ms)")
        
        # Run multiple iterations for accuracy
        execution_times = []
        file_sizes = []
        
        for i in range(3):
            # Clean old file if exists
            if os.path.exists(test['expected_output']):
                os.remove(test['expected_output'])
            
            # Run the test
            run_result = run_timed_command(test['cmd'], f"{test['name']} iteration {i+1}")
            
            if run_result['success']:
                execution_times.append(run_result['execution_time_ms'])
                
                # Check output file
                if os.path.exists(test['expected_output']):
                    file_size = os.path.getsize(test['expected_output'])
                    file_sizes.append(file_size)
                    print(f"  Run {i+1}: {run_result['execution_time_ms']:.0f}ms, {file_size/1024:.1f}KB")
                else:
                    print(f"  Run {i+1}: {run_result['execution_time_ms']:.0f}ms, output file not found")
            else:
                print(f"  Run {i+1}: FAILED - {run_result.get('error', 'Unknown error')}")
        
        if execution_times:
            avg_time = sum(execution_times) / len(execution_times)
            avg_size = sum(file_sizes) / len(file_sizes) if file_sizes else 0
            
            # Calculate pages per second (assume 1 page for simple tests)
            pages = 1
            if 'charts' in test['name']:
                pages = 1  # Charts example creates 1 complex page
            
            pages_per_second = pages / (avg_time / 1000) if avg_time > 0 else 0
            
            result = {
                'test_name': test['name'],
                'avg_execution_time_ms': avg_time,
                'min_execution_time_ms': min(execution_times),
                'max_execution_time_ms': max(execution_times),
                'avg_file_size_bytes': avg_size,
                'avg_file_size_kb': avg_size / 1024,
                'pages': pages,
                'pages_per_second': pages_per_second,
                'iterations': len(execution_times)
            }
            
            results.append(result)
            
            print(f"📊 Average: {avg_time:.0f}ms, {avg_size/1024:.1f}KB, {pages_per_second:.1f} pages/sec")
        else:
            print("❌ All iterations failed")
    
    # Summary
    print("\n" + "=" * 50)
    print("PERFORMANCE SUMMARY")
    print("=" * 50)
    
    if results:
        total_avg_time = sum(r['avg_execution_time_ms'] for r in results) / len(results)
        total_pages_per_sec = sum(r['pages_per_second'] for r in results) / len(results)
        
        print(f"Tests run: {len(results)}")
        print(f"Average execution time: {total_avg_time:.0f}ms")
        print(f"Average pages/second: {total_pages_per_sec:.1f}")
        
        print(f"\nDetailed results:")
        for r in results:
            print(f"  {r['test_name']}: {r['avg_execution_time_ms']:.0f}ms ({r['pages_per_second']:.1f} pages/sec)")
        
        # Compare with claims
        claimed_pages_per_sec = 215  # From CLAUDE.md
        print(f"\nComparison with claimed performance:")
        print(f"  Claimed: {claimed_pages_per_sec} pages/second")
        print(f"  Measured: {total_pages_per_sec:.1f} pages/second")
        
        if total_pages_per_sec >= claimed_pages_per_sec:
            print("  ✅ PERFORMANCE CLAIMS VALIDATED")
        elif total_pages_per_sec >= claimed_pages_per_sec * 0.8:  # Within 80%
            print("  ⚠️  PERFORMANCE CLAIMS MOSTLY VALIDATED")
        else:
            print("  ❌ PERFORMANCE CLAIMS NOT VALIDATED")
            
        # Note about methodology
        print(f"\nNote: This is a simple benchmark with basic PDFs.")
        print(f"Real-world performance may vary based on document complexity.")
        
    else:
        print("No successful tests to analyze")
    
    # Save results
    with open('tools/benchmarks/oxidize_quick_benchmark.json', 'w') as f:
        json.dump({
            'timestamp': time.time(),
            'summary': {
                'tests_run': len(results),
                'avg_pages_per_second': total_pages_per_sec if results else 0,
                'claimed_pages_per_second': 215
            },
            'detailed_results': results
        }, f, indent=2)
    
    print(f"\nResults saved to: tools/benchmarks/oxidize_quick_benchmark.json")

if __name__ == "__main__":
    main()