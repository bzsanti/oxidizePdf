//! RC4 encryption algorithm implementation

/// RC4 key for encryption/decryption
#[derive(Debug, Clone)]
pub struct Rc4Key {
    /// Key bytes
    pub key: Vec<u8>,
}

impl Rc4Key {
    /// Create a new RC4 key
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }

    /// Create from slice
    pub fn from_slice(key: &[u8]) -> Self {
        Self { key: key.to_vec() }
    }
}

/// RC4 cipher state
pub struct Rc4 {
    /// State array
    s: [u8; 256],
    /// Index i
    i: usize,
    /// Index j
    j: usize,
}

impl Rc4 {
    /// Create a new RC4 cipher with the given key
    pub fn new(key: &Rc4Key) -> Self {
        let mut s = [0u8; 256];

        // Initialize state array
        for (i, byte) in s.iter_mut().enumerate() {
            *byte = i as u8;
        }

        // Key scheduling algorithm (KSA)
        let mut j = 0usize;
        for i in 0..256 {
            j = (j + s[i] as usize + key.key[i % key.key.len()] as usize) % 256;
            s.swap(i, j);
        }

        Self { s, i: 0, j: 0 }
    }

    /// Process data (encrypt or decrypt - RC4 is symmetric)
    pub fn process(&mut self, data: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(data.len());

        for &byte in data {
            // Pseudo-random generation algorithm (PRGA)
            self.i = (self.i + 1) % 256;
            self.j = (self.j + self.s[self.i] as usize) % 256;
            self.s.swap(self.i, self.j);

            let k = self.s[(self.s[self.i] as usize + self.s[self.j] as usize) % 256];
            output.push(byte ^ k);
        }

        output
    }

    /// Process data in place
    pub fn process_in_place(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            // PRGA
            self.i = (self.i + 1) % 256;
            self.j = (self.j + self.s[self.i] as usize) % 256;
            self.s.swap(self.i, self.j);

            let k = self.s[(self.s[self.i] as usize + self.s[self.j] as usize) % 256];
            *byte ^= k;
        }
    }
}

/// Encrypt data using RC4
#[allow(dead_code)]
pub fn rc4_encrypt(key: &Rc4Key, data: &[u8]) -> Vec<u8> {
    let mut cipher = Rc4::new(key);
    cipher.process(data)
}

/// Decrypt data using RC4 (same as encrypt for RC4)
#[allow(dead_code)]
pub fn rc4_decrypt(key: &Rc4Key, data: &[u8]) -> Vec<u8> {
    rc4_encrypt(key, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc4_key_creation() {
        let key = Rc4Key::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(key.key, vec![1, 2, 3, 4, 5]);

        let key2 = Rc4Key::from_slice(&[6, 7, 8]);
        assert_eq!(key2.key, vec![6, 7, 8]);
    }

    #[test]
    fn test_rc4_encryption_decryption() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let plaintext = b"Hello, World!";

        // Encrypt
        let ciphertext = rc4_encrypt(&key, plaintext);
        assert_ne!(ciphertext, plaintext);

        // Decrypt
        let decrypted = rc4_decrypt(&key, &ciphertext);
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_rc4_process_in_place() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let mut data = b"Test data".to_vec();
        let original = data.clone();

        // Encrypt in place
        let mut cipher = Rc4::new(&key);
        cipher.process_in_place(&mut data);
        assert_ne!(data, original);

        // Decrypt in place
        let mut cipher = Rc4::new(&key);
        cipher.process_in_place(&mut data);
        assert_eq!(data, original);
    }

    #[test]
    fn test_rc4_known_vectors() {
        // Test vector from RFC 6229
        let key = Rc4Key::from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);
        let mut cipher = Rc4::new(&key);

        // Generate keystream
        let zeros = vec![0u8; 16];
        let keystream = cipher.process(&zeros);

        // First 16 bytes of keystream for key [01 02 03 04 05]
        let expected = [
            0xb2, 0x39, 0x63, 0x05, 0xf0, 0x3d, 0xc0, 0x27, 0xcc, 0xc3, 0x52, 0x4a, 0x0a, 0x11,
            0x18, 0xa8,
        ];

        assert_eq!(&keystream[..16], &expected[..]);
    }

    #[test]
    fn test_rc4_key_debug() {
        let key = Rc4Key::new(vec![1, 2, 3, 4, 5]);
        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("Rc4Key"));
    }

    #[test]
    fn test_rc4_key_clone() {
        let key = Rc4Key::new(vec![1, 2, 3, 4, 5]);
        let cloned_key = key.clone();
        assert_eq!(key.key, cloned_key.key);
    }

    #[test]
    fn test_rc4_empty_data() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let empty_data = vec![];

        let result = rc4_encrypt(&key, &empty_data);
        assert!(result.is_empty());

        let result = rc4_decrypt(&key, &empty_data);
        assert!(result.is_empty());
    }

    #[test]
    fn test_rc4_single_byte() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let single_byte = vec![0xFF];

        let encrypted = rc4_encrypt(&key, &single_byte);
        assert_eq!(encrypted.len(), 1);

        let decrypted = rc4_decrypt(&key, &encrypted);
        assert_eq!(decrypted, single_byte);
    }

    #[test]
    fn test_rc4_large_data() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let large_data = vec![0x42; 1000];

        let encrypted = rc4_encrypt(&key, &large_data);
        assert_eq!(encrypted.len(), 1000);
        assert_ne!(encrypted, large_data); // Should be different

        let decrypted = rc4_decrypt(&key, &encrypted);
        assert_eq!(decrypted, large_data);
    }

    #[test]
    fn test_rc4_different_keys_different_output() {
        let data = b"Hello, World!";

        let key1 = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let key2 = Rc4Key::new(vec![0x05, 0x04, 0x03, 0x02, 0x01]);

        let encrypted1 = rc4_encrypt(&key1, data);
        let encrypted2 = rc4_encrypt(&key2, data);

        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_rc4_various_key_lengths() {
        let data = b"Test data";

        // Test different key lengths
        let key_8bit = Rc4Key::new(vec![0x01]);
        let key_40bit = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let key_128bit = Rc4Key::new(vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ]);

        let enc1 = rc4_encrypt(&key_8bit, data);
        let enc2 = rc4_encrypt(&key_40bit, data);
        let enc3 = rc4_encrypt(&key_128bit, data);

        // All should produce different outputs
        assert_ne!(enc1, enc2);
        assert_ne!(enc2, enc3);
        assert_ne!(enc1, enc3);

        // But all should decrypt correctly
        assert_eq!(rc4_decrypt(&key_8bit, &enc1), data);
        assert_eq!(rc4_decrypt(&key_40bit, &enc2), data);
        assert_eq!(rc4_decrypt(&key_128bit, &enc3), data);
    }

    #[test]
    fn test_rc4_process_in_place_empty() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let mut data = vec![];

        let mut cipher = Rc4::new(&key);
        cipher.process_in_place(&mut data);

        assert!(data.is_empty());
    }

    #[test]
    fn test_rc4_process_consistency() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let data = b"Consistency test data";

        // Test that process() and process_in_place() give same results
        let result1 = {
            let mut cipher = Rc4::new(&key);
            cipher.process(data)
        };

        let result2 = {
            let mut cipher = Rc4::new(&key);
            let mut data_copy = data.to_vec();
            cipher.process_in_place(&mut data_copy);
            data_copy
        };

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_rc4_stateful_processing() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let data1 = b"First chunk";
        let data2 = b"Second chunk";

        // Process in chunks using same cipher instance
        let mut cipher = Rc4::new(&key);
        let enc1 = cipher.process(data1);
        let enc2 = cipher.process(data2);

        // Process all at once using new cipher instance
        let mut cipher2 = Rc4::new(&key);
        let mut combined_data = Vec::new();
        combined_data.extend_from_slice(data1);
        combined_data.extend_from_slice(data2);
        let enc_combined = cipher2.process(&combined_data);

        // Results should be the same
        let chunked_result = [enc1, enc2].concat();
        assert_eq!(chunked_result, enc_combined);
    }

    #[test]
    fn test_rc4_binary_data() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let binary_data = vec![0x00, 0xFF, 0x80, 0x7F, 0x01, 0xFE, 0x55, 0xAA];

        let encrypted = rc4_encrypt(&key, &binary_data);
        let decrypted = rc4_decrypt(&key, &encrypted);

        assert_eq!(decrypted, binary_data);
    }
}
