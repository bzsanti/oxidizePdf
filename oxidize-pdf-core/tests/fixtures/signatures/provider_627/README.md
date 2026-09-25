Public verification vectors generated with OpenSSL 3 for issue #627.
Private keys were ephemeral and discarded; these are not credentials.
Message bytes are in message.bin. RSA: 2048 bits, PKCS#1 v1.5 and PSS
(MGF1 digest and salt length equal to message digest); ECDSA: P-256/P-384
with SHA-256/SHA-384; Ed25519: pure EdDSA. SPKI and signatures are DER/raw
as required by their algorithms. Tests mutate each message/signature/key.

The `_wrong.spki` files contain independently generated, valid public keys for
negative verification tests. RSA-PSS `salt_0` and `salt_32` are both signed with
the private counterpart of `rsa_wrong.spki`; only salt_32 matches the SHA-256
AlgorithmIdentifier policy. Keys were generated with `openssl genpkey` using
RSA 2048, EC P-256/P-384, and ED25519; public keys with `openssl pkey -pubout
-outform DER`. RSA/ECDSA messages were signed with `openssl dgst -sha256`
(or -sha384/-sha512), RSA-PSS with `rsa_padding_mode:pss` and explicit salt
length; Ed25519 with `openssl pkeyutl -sign -rawin`. Regeneration creates new
keys and bytes; committed vectors are fixed and require no OpenSSL at test time.
