# Error-Focused Analysis Report

This report focuses specifically on parsing errors and provides actionable insights.

## 🔤 Character Encoding Issues

Found 1 character encoding errors. These are now visible after fixing the circular reference false positives.

### Common Problematic Characters


### Recommended Solutions

1. Implement proper encoding detection (Latin-1, UTF-8, etc.)
2. Add fallback handling for unknown characters
3. Implement lenient parsing mode for character issues
4. Consider using replacement characters for unparseable bytes

## 📋 Failed Files Analysis

### MI TARJETA.pdf
- **Size:** 413908 bytes
- **PDF Version:** Unknown
- **Error Type:** CharacterEncoding
- **Error:** Syntax error at position 413269: Unexpected character: ÿ

### audit - audit.pdf
- **Size:** 57316 bytes
- **PDF Version:** Unknown
- **Error Type:** Encryption
- **Error:** PDF is encrypted. Decryption is not currently supported in the community edition

