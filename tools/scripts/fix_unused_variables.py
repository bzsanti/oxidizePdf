#!/usr/bin/env python3
"""
Fix unused variable warnings by prefixing with underscore.

This script scans for unused variable warnings and automatically
adds underscore prefixes to fix them.
"""

import re
import sys
from pathlib import Path
from typing import List, Tuple


class UnusedVariableFixer:
    """Fix unused variable warnings in Rust files."""

    def __init__(self, verbose: bool = False):
        """Initialize the fixer.

        Args:
            verbose: If True, print detailed information.
        """
        self.verbose = verbose
        self.stats = {
            'files_processed': 0,
            'files_modified': 0,
            'total_fixed': 0,
            'files': {}
        }

    def fix_file(self, filepath: Path, fixes: List[Tuple[int, str]]) -> bool:
        """Fix unused variables in a file.

        Args:
            filepath: Path to the file.
            fixes: List of tuples (line_number, variable_name).

        Returns:
            True if file was modified, False otherwise.
        """
        if not filepath.exists():
            print(f"ERROR: File not found: {filepath}")
            return False

        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                lines = f.readlines()
        except Exception as e:
            print(f"ERROR: Failed to read {filepath}: {e}")
            return False

        modified = False
        fixed_count = 0

        # Process fixes in reverse order to maintain line numbers
        for line_num, var_name in reversed(fixes):
            # line_num is 1-indexed, but list is 0-indexed
            idx = line_num - 1

            if idx < 0 or idx >= len(lines):
                continue

            line = lines[idx]

            # Skip if already has underscore
            if f'_{var_name}' in line:
                continue

            # Match the variable in different contexts
            patterns = [
                (f'\\b{var_name}\\b(?=\\))', f'_{var_name}'),  # Before )
                (f'\\b{var_name}\\b(?=,)', f'_{var_name}'),   # Before ,
                (f'\\b{var_name}\\b(?=\\s*:)', f'_{var_name}'),  # Before :
            ]

            new_line = line
            for pattern, replacement in patterns:
                if re.search(pattern, new_line):
                    new_line = re.sub(pattern, replacement, new_line)
                    if self.verbose:
                        print(f"  Line {line_num}: {var_name} → _{var_name}")
                    fixed_count += 1
                    modified = True
                    break

            if modified:
                lines[idx] = new_line

        if modified:
            # Create backup
            backup_path = filepath.with_suffix(filepath.suffix + '.bak')
            try:
                with open(backup_path, 'w', encoding='utf-8') as f:
                    f.writelines(lines)
            except Exception as e:
                print(f"ERROR: Failed to create backup {backup_path}: {e}")
                return False

            # Write modified content
            try:
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.writelines(lines)
            except Exception as e:
                print(f"ERROR: Failed to write {filepath}: {e}")
                return False

            self.stats['files_modified'] += 1
            self.stats['total_fixed'] += fixed_count
            self.stats['files'][str(filepath)] = fixed_count

            return True

        return False

    def process_compiler_output(self, output: str) -> None:
        """Parse compiler output and fix unused variable warnings.

        Args:
            output: The output from cargo build.
        """
        fixes_by_file = {}

        # Pattern: "warning: unused variable: `varname`"
        # Followed by: "--> path/to/file.rs:LINE:COL"
        pattern = r'warning: unused variable: `(\w+)`\s+\n\s+-->\s+([^:]+):(\d+):'

        for match in re.finditer(pattern, output):
            var_name = match.group(1)
            filepath = match.group(2)
            line_num = int(match.group(3))

            if filepath not in fixes_by_file:
                fixes_by_file[filepath] = []

            fixes_by_file[filepath].append((line_num, var_name))

        if not fixes_by_file:
            print("No unused variable warnings found")
            return

        print(f"Found {len(fixes_by_file)} file(s) with unused variables")
        print("-" * 70)

        for filepath_str, fixes in fixes_by_file.items():
            filepath = Path(filepath_str)
            if self.verbose:
                print(f"\nProcessing: {filepath}")
                for line_num, var_name in fixes:
                    print(f"  Line {line_num}: {var_name}")

            self.fix_file(filepath, fixes)

        self.stats['files_processed'] = len(fixes_by_file)
        self._print_summary()

    def _print_summary(self) -> None:
        """Print a summary of the processing results."""
        print("-" * 70)
        print("\nSUMMARY:")
        print(f"  Files processed: {self.stats['files_processed']}")
        print(f"  Files modified: {self.stats['files_modified']}")
        print(f"  Total variables fixed: {self.stats['total_fixed']}")


def main():
    """Main entry point."""
    verbose = '-v' in sys.argv or '--verbose' in sys.argv
    help_requested = '-h' in sys.argv or '--help' in sys.argv

    if help_requested:
        print(__doc__)
        sys.exit(0)

    # Read from stdin if available, otherwise run cargo build
    if not sys.stdin.isatty():
        output = sys.stdin.read()
    else:
        import subprocess
        result = subprocess.run(['cargo', 'build', '--lib'], capture_output=True, text=True)
        output = result.stdout + result.stderr

    fixer = UnusedVariableFixer(verbose=verbose)
    fixer.process_compiler_output(output)

    sys.exit(0)


if __name__ == '__main__':
    main()
