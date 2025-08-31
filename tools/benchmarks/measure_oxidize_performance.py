#!/usr/bin/env python3
"""
Measure actual performance of oxidize-pdf library
Tests real examples and compares against claimed performance metrics
"""

import subprocess
import time
import os
import json
import statistics
from pathlib import Path
from typing import List, Dict, Any
from datetime import datetime

class PerformanceTest:
    def __init__(self, name: str, example_name: str, expected_pages: int = 1):
        self.name = name
        self.example_name = example_name
        self.expected_pages = expected_pages
        
class PerformanceResult:
    def __init__(self, test_name: str, success: bool, execution_time_ms: float,
                 file_size_bytes: int = 0, pages: int = 1, error: str = None):
        self.test_name = test_name
        self.success = success
        self.execution_time_ms = execution_time_ms
        self.file_size_bytes = file_size_bytes
        self.pages = pages
        self.error = error
        self.pages_per_second = pages / (execution_time_ms / 1000) if execution_time_ms > 0 else 0
        
    def to_dict(self) -> Dict[str, Any]:
        return {
            'test_name': self.test_name,
            'success': self.success,
            'execution_time_ms': self.execution_time_ms,
            'file_size_bytes': self.file_size_bytes,
            'file_size_kb': self.file_size_bytes / 1024,
            'pages': self.pages,
            'pages_per_second': self.pages_per_second,
            'error': self.error,
            'timestamp': datetime.now().isoformat()
        }

def run_cargo_example(example_name: str, iterations: int = 3) -> List[PerformanceResult]:
    """Run a cargo example multiple times and measure performance"""
    results = []
    
    for i in range(iterations):
        print(f"  Running iteration {i+1}/{iterations}...")
        
        start_time = time.time()
        
        try:
            # Run the example in release mode for accurate performance
            result = subprocess.run([
                "cargo", "run", "--release", "--example", example_name
            ], capture_output=True, text=True, cwd="../../", timeout=60)
            
            end_time = time.time()
            execution_time = (end_time - start_time) * 1000  # Convert to ms
            
            if result.returncode == 0:
                # Try to find generated PDF and measure size
                pdf_files = list(Path("../../examples/results/").glob("*.pdf"))
                latest_pdf = max(pdf_files, key=os.path.getctime) if pdf_files else None
                
                file_size = os.path.getsize(latest_pdf) if latest_pdf else 0
                
                # Extract pages info from output if available
                pages = 1
                if "pages" in result.stdout.lower():
                    try:
                        # Simple heuristic to extract page count
                        lines = result.stdout.split('\n')
                        for line in lines:
                            if "page" in line.lower() and any(char.isdigit() for char in line):
                                numbers = [int(s) for s in line.split() if s.isdigit()]
                                if numbers:
                                    pages = max(numbers)  # Take the largest number as page count
                    except:
                        pages = 1
                
                perf_result = PerformanceResult(
                    test_name=example_name,
                    success=True,
                    execution_time_ms=execution_time,
                    file_size_bytes=file_size,
                    pages=pages
                )
            else:
                perf_result = PerformanceResult(
                    test_name=example_name,
                    success=False,
                    execution_time_ms=execution_time,
                    error=result.stderr
                )
                
        except subprocess.TimeoutExpired:
            perf_result = PerformanceResult(
                test_name=example_name,
                success=False,
                execution_time_ms=60000,  # 60 seconds timeout
                error="Timeout after 60 seconds"
            )
        except Exception as e:
            perf_result = PerformanceResult(
                test_name=example_name,
                success=False,
                execution_time_ms=0,
                error=str(e)
            )
            
        results.append(perf_result)
    
    return results

def run_unit_tests_with_timing() -> PerformanceResult:
    """Run unit tests and measure performance"""
    print("Running unit tests with timing...")
    
    start_time = time.time()
    
    try:
        result = subprocess.run([
            "cargo", "test", "--release", "--lib"
        ], capture_output=True, text=True, cwd="../../", timeout=300)
        
        end_time = time.time()
        execution_time = (end_time - start_time) * 1000
        
        # Extract test count from output
        test_count = 0
        if result.returncode == 0 and "test result:" in result.stdout:
            try:
                result_line = [line for line in result.stdout.split('\n') if 'test result:' in line][-1]
                # Parse something like "test result: ok. 123 passed; 0 failed"
                parts = result_line.split()
                passed_idx = next(i for i, part in enumerate(parts) if 'passed' in part)
                test_count = int(parts[passed_idx - 1])
            except:
                test_count = 1
        
        return PerformanceResult(
            test_name="unit_tests",
            success=result.returncode == 0,
            execution_time_ms=execution_time,
            pages=test_count,  # Use test count as "pages" for consistency
            error=result.stderr if result.returncode != 0 else None
        )
        
    except Exception as e:
        return PerformanceResult(
            test_name="unit_tests",
            success=False,
            execution_time_ms=0,
            error=str(e)
        )

def get_available_examples() -> List[str]:
    """Get list of available examples"""
    try:
        result = subprocess.run([
            "cargo", "run", "--example"
        ], capture_output=True, text=True, cwd="../../")
        
        if "error: " in result.stderr and "example" in result.stderr:
            # Extract example names from error message
            lines = result.stderr.split('\n')
            examples = []
            in_example_list = False
            
            for line in lines:
                if "Available examples:" in line:
                    in_example_list = True
                    continue
                elif in_example_list and line.strip():
                    example_name = line.strip()
                    if example_name and not example_name.startswith('error:'):
                        examples.append(example_name)
                elif in_example_list and not line.strip():
                    break
            
            return examples
    except:
        pass
    
    # Fallback: known working examples
    return [
        'font_spacing_test',
        'charts_comprehensive_test',
        'advanced_tables_example',
    ]

def analyze_performance_claims() -> Dict[str, Any]:
    """Analyze claimed vs actual performance"""
    
    # Load claimed performance from docs if available
    claimed_performance = {
        'pages_per_second': 215,  # From CLAUDE.md
        'pdf_parsing_success_rate': 97.2,  # From CLAUDE.md 
        'total_tests': 3491,  # From CLAUDE.md
    }
    
    return {
        'claimed': claimed_performance,
        'analysis': {
            'pages_per_second_claimed': 215,
            'parsing_success_claimed': 0.972,
            'total_tests_claimed': 3491
        }
    }

def main():
    print("oxidize-pdf Performance Analysis")
    print("=" * 50)
    
    # Create results directory
    os.makedirs("../../examples/results", exist_ok=True)
    
    # Get available examples
    examples = get_available_examples()
    print(f"Found examples: {examples}")
    
    all_results = []
    
    # Test unit tests performance
    print("\n1. Testing unit test performance...")
    test_result = run_unit_tests_with_timing()
    all_results.append(test_result)
    print(f"   Result: {test_result.execution_time_ms:.0f}ms, {test_result.success}")
    
    # Test examples
    print(f"\n2. Testing {len(examples)} examples...")
    
    for example in examples:
        print(f"\nTesting {example}...")
        try:
            results = run_cargo_example(example, iterations=2)
            all_results.extend(results)
            
            # Show quick stats
            successful = [r for r in results if r.success]
            if successful:
                avg_time = statistics.mean([r.execution_time_ms for r in successful])
                avg_pages_per_sec = statistics.mean([r.pages_per_second for r in successful])
                print(f"   Avg time: {avg_time:.0f}ms, Pages/sec: {avg_pages_per_sec:.1f}")
            else:
                print(f"   Failed: {results[0].error if results else 'Unknown error'}")
                
        except Exception as e:
            print(f"   Error running {example}: {e}")
    
    # Analyze results
    print("\n" + "=" * 50)
    print("PERFORMANCE ANALYSIS")
    print("=" * 50)
    
    successful_results = [r for r in all_results if r.success and r.execution_time_ms > 0]
    
    if successful_results:
        # Calculate aggregate statistics
        total_time = sum(r.execution_time_ms for r in successful_results)
        total_pages = sum(r.pages for r in successful_results)
        avg_pages_per_sec = statistics.mean([r.pages_per_second for r in successful_results])
        
        print(f"Total successful tests: {len(successful_results)}")
        print(f"Total execution time: {total_time:.0f}ms")
        print(f"Total pages processed: {total_pages}")
        print(f"Average pages/second: {avg_pages_per_sec:.1f}")
        
        # Compare with claims
        performance_claims = analyze_performance_claims()
        claimed_pages_per_sec = performance_claims['claimed']['pages_per_second']
        
        print(f"\nComparison with claims:")
        print(f"  Claimed pages/second: {claimed_pages_per_sec}")
        print(f"  Measured pages/second: {avg_pages_per_sec:.1f}")
        print(f"  Performance ratio: {avg_pages_per_sec/claimed_pages_per_sec:.2f}x")
        
        if avg_pages_per_sec >= claimed_pages_per_sec:
            print("  ✅ Performance claims VALIDATED")
        else:
            print("  ❌ Performance claims NOT VALIDATED")
    else:
        print("No successful tests to analyze")
    
    # Save detailed results
    report = {
        'timestamp': datetime.now().isoformat(),
        'summary': {
            'total_tests': len(all_results),
            'successful_tests': len(successful_results),
            'failed_tests': len(all_results) - len(successful_results),
            'avg_pages_per_second': avg_pages_per_sec if successful_results else 0,
        },
        'detailed_results': [r.to_dict() for r in all_results],
        'performance_claims_analysis': analyze_performance_claims()
    }
    
    with open('oxidize_performance_analysis.json', 'w') as f:
        json.dump(report, f, indent=2)
    
    print(f"\nDetailed report saved to: oxidize_performance_analysis.json")
    
    # Show failed tests
    failed_results = [r for r in all_results if not r.success]
    if failed_results:
        print(f"\nFailed tests ({len(failed_results)}):")
        for r in failed_results:
            print(f"  {r.test_name}: {r.error}")

if __name__ == "__main__":
    main()