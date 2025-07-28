//! PDF encryption support according to ISO 32000-1 Chapter 7.6
//!
//! This module provides basic encryption using RC4 40-bit and 128-bit algorithms,
//! supporting Standard Security Handler (Revision 2, 3, and 4).

mod encryption_dict;
mod permissions;
mod rc4;
mod standard_security;

pub use encryption_dict::{
    CryptFilter, CryptFilterMethod, EncryptionAlgorithm, EncryptionDictionary, StreamFilter,
    StringFilter,
};
pub use permissions::{PermissionFlags, Permissions};
pub use rc4::{Rc4, Rc4Key};
pub use standard_security::{
    EncryptionKey, OwnerPassword, SecurityHandlerRevision, StandardSecurityHandler, UserPassword,
};
