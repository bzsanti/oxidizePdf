Public ECDSA verification vectors copied unchanged from oxidize-webpki tests/fixtures
for issue #828. Generated independently with OpenSSL 3 for issue #627.
Private keys were ephemeral and discarded. These files contain no credentials.

message.bin is signed with P-256/SHA-256 and P-384/SHA-384. Public keys use DER
SubjectPublicKeyInfo; signatures use DER ECDSA encoding. *_wrong.spki are valid,
independently generated keys for negative controls. Verification tests need no OpenSSL.

Generation commands: openssl genpkey with EC P-256/P-384; openssl pkey -pubout
-outform DER; openssl dgst -sha256/-sha384 -sign. Regeneration changes keys and
signatures, so the committed vectors are fixed interoperability evidence.
