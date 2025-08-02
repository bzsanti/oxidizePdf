# Error-Focused Analysis Report

This report focuses specifically on parsing errors and provides actionable insights.

## 🔤 Character Encoding Issues

Found 38 character encoding errors. These are now visible after fixing the circular reference false positives.

### Common Problematic Characters


### Recommended Solutions

1. Implement proper encoding detection (Latin-1, UTF-8, etc.)
2. Add fallback handling for unknown characters
3. Implement lenient parsing mode for character issues
4. Consider using replacement characters for unparseable bytes

## 📋 Failed Files Analysis

### 0184c79b-9922-4514-a9ed-3919070ca099.pdf
- **Size:** 132084 bytes
- **PDF Version:** 1.4
- **Error Type:** Encryption
- **Error:** PDF is encrypted. Decryption is not currently supported in the community edition

### 04.ANNEX 2 PQQ rev.01.pdf
- **Size:** 554176 bytes
- **PDF Version:** Unknown
- **Error Type:** SyntaxError
- **Error:** Syntax error at position 93604: Unterminated string

### 1002579 - FIRMADO.pdf
- **Size:** 3380402 bytes
- **PDF Version:** Unknown
- **Error Type:** Encryption
- **Error:** PDF is encrypted. Decryption is not currently supported in the community edition

### 1002579.pdf
- **Size:** 3398053 bytes
- **PDF Version:** 1.7
- **Error Type:** SyntaxError
- **Error:** Syntax error at position 23: Expected generation number

### 14062025211250.pdf
- **Size:** 91494 bytes
- **PDF Version:** 1.4
- **Error Type:** Encryption
- **Error:** PDF is encrypted. Decryption is not currently supported in the community edition

### 1749664155633.pdf
- **Size:** 917834 bytes
- **PDF Version:** 1.5
- **Error Type:** InvalidStructure
- **Error:** Invalid object reference: 55 0 R

### 191121_TD-K1TG-104896_TELEFONICA DE ESPAÑA_415.97.pdf
- **Size:** 97206 bytes
- **PDF Version:** 1.4
- **Error Type:** SyntaxError
- **Error:** Syntax error at position 115153: Expected generation number

### 1HSN221000481395.pdf
- **Size:** 289786 bytes
- **PDF Version:** 1.4
- **Error Type:** SyntaxError
- **Error:** Syntax error at position 472344: Expected generation number

### 1HSN221100056583.pdf
- **Size:** 551485 bytes
- **PDF Version:** 1.4
- **Error Type:** SyntaxError
- **Error:** Syntax error at position 924617: Expected generation number

### 20062024 Burwell Asset Management Service Agreement (vs09)_EXE.pdf
- **Size:** 1780204 bytes
- **PDF Version:** 1.7
- **Error Type:** InvalidStructure
- **Error:** Invalid object reference: 3 0 R

