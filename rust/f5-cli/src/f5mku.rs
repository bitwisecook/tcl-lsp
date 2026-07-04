//! F5 master-key (`f5mku`) secret encryption / decryption.
//!
//! The implementation now lives in the standalone, wasm-friendly
//! [`tcl_f5mku`] crate (so the in-browser BIG-IP report generator can decrypt
//! secrets too); this module re-exports it to keep the `f5_cli::f5mku` path
//! stable for the `encrypt-secrets` / `decrypt-secrets` verbs.

pub use tcl_f5mku::{F5MkuError, decrypt, encrypt, extract_salt, is_ciphertext};
