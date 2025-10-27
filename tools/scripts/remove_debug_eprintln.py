#!/usr/bin/env python3
"""
Remove DEBUG eprintln! statements from Rust files.

This script identifies and removes all eprintln! macros that contain "DEBUG:"
from specified Rust files. It handles both single-line and multi-line eprintln!
statements while preserving code structure and indentation.

Features:
- Creates .bak backups before modification
- Handles multi-line eprintln! statements
- Preserves surrounding code indentation
- Provides detailed reporting
"""

import re
import sys
from pathlib import Path
from typing import List, Tuple, Optional


class DebugEprintlnRemover:
    """Remove DEBUG eprintln! statements from Rust files."""

    # Pattern to match complete eprintln! blocks (single or multi-line)
    # This pattern finds:
    # 1. Optional leading whitespace (captured for indentation)
    # 2. eprintln!( opening
    # 3. Content up to closing ) including newlines (non-greedy)
    # 4. Trailing semicolon
    # 5. Optional trailing comma and/or newline
    EPRINTLN_PATTERN = re.compile(
        r'([ \t]*)eprintln!\(\s*(?:[^()]*|\((?:[^()]*|\([^()]*\))*\))*?\);?',
        re.MULTILINE | re.DOTALL
    )

    def __init__(self, verbose: bool = False):
        """Initialize the remover.

        Args:
            verbose: If True, print detailed information about each removal.
        """
        self.verbose = verbose
        self.stats = {
            'files_processed': 0,
            'files_modified': 0,
            'total_removed': 0,
            'files': {}
        }

    def _contains_debug_marker(self, match_text: str) -> bool:
        """Check if the eprintln! statement contains DEBUG marker.

        Args:
            match_text: The matched eprintln! text.

        Returns:
            True if the statement contains "DEBUG:", False otherwise.
        """
        return 'DEBUG:' in match_text

    def _remove_trailing_newline(self, content: str, end_pos: int) -> Tuple[int, int]:
        """Check if we should remove trailing newline after the statement.

        Args:
            content: The full file content.
            end_pos: The end position of the match.

        Returns:
            Tuple of (adjusted_start, adjusted_end) for the removal span.
        """
        # If there's a newline immediately after, include it in removal
        if end_pos < len(content) and content[end_pos] == '\n':
            return (end_pos, end_pos + 1)
        elif end_pos < len(content) - 1 and content[end_pos:end_pos+2] == '\r\n':
            return (end_pos, end_pos + 2)
        return (end_pos, end_pos)

    def process_file(self, filepath: Path) -> bool:
        """Process a single Rust file and remove DEBUG eprintln! statements.

        Args:
            filepath: Path to the Rust file.

        Returns:
            True if file was modified, False otherwise.
        """
        if not filepath.exists():
            print(f"ERROR: File not found: {filepath}")
            return False

        # Read original content
        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                original_content = f.read()
        except Exception as e:
            print(f"ERROR: Failed to read {filepath}: {e}")
            return False

        # Find all eprintln! statements
        removed_statements = []
        new_content = original_content

        # Find all matches
        matches = list(self.EPRINTLN_PATTERN.finditer(original_content))

        if not matches:
            self.stats['files_processed'] += 1
            self.stats['files'][str(filepath)] = {'removed': 0, 'status': 'no_eprintln'}
            return False

        # Process matches in reverse order to maintain positions
        for match in reversed(matches):
            match_text = match.group(0)

            # Check if this statement contains DEBUG marker
            if not self._contains_debug_marker(match_text):
                continue

            # Extract the indentation from the match
            indentation = match.group(1)

            removed_statements.append({
                'line_content': match_text.strip(),
                'position': match.start(),
                'indentation': len(indentation)
            })

            if self.verbose:
                # Calculate line number for reporting
                line_num = original_content[:match.start()].count('\n') + 1
                print(f"  Removing DEBUG eprintln! at line {line_num}:")
                print(f"    {match_text.strip()[:80]}...")

            # Determine the span to remove (including trailing newline)
            start = match.start()
            end = match.end()

            # Check for trailing newline
            newline_start, newline_end = self._remove_trailing_newline(
                original_content, end
            )

            if newline_start > 0:
                end = newline_end

            # Remove the statement
            new_content = new_content[:start] + new_content[end:]

        # Only write back if we removed something
        if removed_statements:
            # Create backup
            backup_path = filepath.with_suffix(filepath.suffix + '.bak')
            try:
                with open(backup_path, 'w', encoding='utf-8') as f:
                    f.write(original_content)
                if self.verbose:
                    print(f"  Created backup: {backup_path}")
            except Exception as e:
                print(f"ERROR: Failed to create backup {backup_path}: {e}")
                return False

            # Write modified content
            try:
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.write(new_content)
                if self.verbose:
                    print(f"  Modified file: {filepath}")
            except Exception as e:
                print(f"ERROR: Failed to write {filepath}: {e}")
                return False

            # Update stats
            self.stats['files_modified'] += 1
            self.stats['total_removed'] += len(removed_statements)
            self.stats['files'][str(filepath)] = {
                'removed': len(removed_statements),
                'status': 'modified'
            }

            return True

        self.stats['files_processed'] += 1
        self.stats['files'][str(filepath)] = {'removed': 0, 'status': 'no_debug'}
        return False

    def process_files(self, filepaths: List[Path]) -> None:
        """Process multiple files.

        Args:
            filepaths: List of paths to process.
        """
        print(f"Processing {len(filepaths)} file(s)...")
        print("-" * 70)

        for filepath in filepaths:
            if self.verbose:
                print(f"\nProcessing: {filepath}")

            self.process_file(filepath)

        self.stats['files_processed'] = len(filepaths)
        self._print_summary()

    def _print_summary(self) -> None:
        """Print a summary of the processing results."""
        print("-" * 70)
        print("\nSUMMARY:")
        print(f"  Files processed: {self.stats['files_processed']}")
        print(f"  Files modified: {self.stats['files_modified']}")
        print(f"  Total statements removed: {self.stats['total_removed']}")

        if self.stats['files_modified'] > 0:
            print("\nDetailed results:")
            for filepath, info in self.stats['files'].items():
                if info['status'] == 'modified':
                    print(f"  {filepath}: {info['removed']} statement(s) removed")

        if self.stats['total_removed'] > 0:
            print("\nBackups created with .bak extension")
            print("To restore: cp file.rs.bak file.rs")


def main():
    """Main entry point."""
    # Define files to process
    files_to_process = [
        Path('oxidize-pdf-core/src/parser/xref_stream.rs'),
        Path('oxidize-pdf-core/src/parser/xref.rs'),
        Path('oxidize-pdf-core/src/parser/reader.rs'),
        Path('oxidize-pdf-core/src/operations/extract_images.rs'),
    ]

    # Check if running from project root
    if not Path('Cargo.toml').exists():
        print("ERROR: Must be run from project root (where Cargo.toml is located)")
        sys.exit(1)

    # Parse command line arguments
    verbose = '-v' in sys.argv or '--verbose' in sys.argv
    help_requested = '-h' in sys.argv or '--help' in sys.argv

    if help_requested:
        print(__doc__)
        print("\nUsage: python remove_debug_eprintln.py [options]")
        print("\nOptions:")
        print("  -v, --verbose    Print detailed information about each removal")
        print("  -h, --help       Show this help message")
        print("\nFiles processed:")
        for f in files_to_process:
            print(f"  - {f}")
        sys.exit(0)

    # Create remover and process files
    remover = DebugEprintlnRemover(verbose=verbose)
    remover.process_files(files_to_process)

    # Exit with appropriate code
    sys.exit(0 if remover.stats['total_removed'] == 0 or remover.stats['files_modified'] > 0 else 1)


if __name__ == '__main__':
    main()
