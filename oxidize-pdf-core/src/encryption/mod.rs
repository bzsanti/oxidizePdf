//! PDF encryption support according to ISO 32000-1 Chapter 7.6
//!
//! This module provides encryption using RC4 40-bit and 128-bit algorithms,
//! AES-128 and AES-256 encryption, supporting Standard Security Handler
//! (Revision 2, 3, 4, 5, and 6).

mod aes;
mod encryption_dict;
mod permissions;
mod rc4;
mod standard_security;

pub use aes::{generate_iv, Aes, AesError, AesKey, AesKeySize};
pub use encryption_dict::{
    CryptFilter, CryptFilterMethod, EncryptionAlgorithm, EncryptionDictionary, StreamFilter,
    StringFilter,
};
pub use permissions::{PermissionFlags, Permissions};
pub use rc4::{Rc4, Rc4Key};
pub use standard_security::{
    EncryptionKey, OwnerPassword, SecurityHandlerRevision, StandardSecurityHandler, UserPassword,
};
