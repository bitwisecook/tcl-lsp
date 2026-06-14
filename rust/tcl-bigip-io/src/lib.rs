//! Pure-Rust f5 input layer.
//!
//! This crate owns the I/O foundations the `f5-query` verbs build on:
//!
//! * [`ucs`] — UCS archive handling: a plain UCS is a gzip-tar of `/config`;
//!   an encrypted UCS is an `OpenPGP` *symmetric* message (F5 KB K5437) whose
//!   plaintext is that same gzip-tar.
//! * [`openpgp`] — a minimal, dependency-free `OpenPGP` symmetric decryptor
//!   (S2K + AES-CFB + SHA-1 MDC), a faithful port of the Python `_openpgp` /
//!   `_aes` fallback — entirely pure Rust, no shelling out to `gpg`.
//! * [`paths`] — the `read_path` / `load_paths` input resolver: read a path
//!   (or stdin), transparently extract a `.ucs` (plain or encrypted) to SCF,
//!   and hand back the source text (and parsed [`tcl_bigip::BigipConfig`]).
//!
//! The cryptographic core ([`ucs`] / [`openpgp`]) is UI-agnostic and does no
//! file I/O: bytes in → SCF text / bytes out, with everything (decrypt →
//! gunzip → untar) handled on in-memory cursors so a UCS's SSL private keys
//! never touch disk. File/stdin I/O lives only in [`paths`], the thin input
//! shell the binaries call.

#![forbid(unsafe_code)]

mod aes_cfb;
pub mod openpgp;
pub mod paths;
pub mod ucs;

pub use openpgp::{decrypt_symmetric, OpenPgpError};
pub use paths::{
    load_paths, read_path, resolve_passphrase, LoadedConfig, PassphraseOptions, PathError,
};
pub use ucs::{
    decrypt_if_encrypted, is_pgp_bytes, is_ucs_bytes, read_ucs_member, ucs_archive_to_scf,
    ucs_to_scf, PassphraseProvider, UcsError, DEFAULT_PASSPHRASE_ENV,
};
