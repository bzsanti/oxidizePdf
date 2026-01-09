//! PDF encryption support according to ISO 32000-1 Chapter 7.6
//!
//! This module provides encryption using RC4 40-bit and 128-bit algorithms,
//! AES-128 and AES-256 encryption, supporting Standard Security Handler
//! (Revision 2, 3, 4, 5, and 6) and Public Key Security Handler.

mod aes;
mod crypt_filters;
mod embedded_files;
mod encryption_dict;
mod object_encryption;
mod permissions;
mod permissions_enforcement;
mod public_key;
mod rc4;
mod standard_security;

pub use aes::{generate_iv, Aes, AesError, AesKey, AesKeySize};
pub use crypt_filters::{AuthEvent, CryptFilterManager, FunctionalCryptFilter, SecurityHandler};
pub use embedded_files::{EmbeddedFileEncryption, ExtendedEncryptionDict};
pub use encryption_dict::{
    CryptFilter, CryptFilterMethod, EncryptionAlgorithm, EncryptionDictionary, StreamFilter,
    StringFilter,
};
pub use object_encryption::{DocumentEncryption, ObjectEncryptor};
pub use permissions::{PermissionFlags, Permissions};
pub use permissions_enforcement::{
    LogLevel, PermissionCallback, PermissionCheckResult, PermissionEvent, PermissionOperation,
    PermissionsValidator, RuntimePermissions, RuntimePermissionsBuilder,
};
pub use public_key::{PublicKeyEncryptionDict, PublicKeySecurityHandler, Recipient, SubFilter};
pub use rc4::{Rc4, Rc4Key};
pub use standard_security::{
    compute_hash_r6_algorithm_2b, EncryptionKey, OwnerPassword, SecurityHandlerRevision,
    StandardSecurityHandler, UserPassword,
};

#[cfg(test)]
mod tests;
