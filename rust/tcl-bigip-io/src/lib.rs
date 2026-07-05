// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pure-Rust f5 input layer.
//!
//! This crate owns the I/O foundations the `f5-query` verbs build on:
//!
//! * [`ucs`] — UCS archive handling: a plain UCS is a gzip-tar of `/config`;
//!   an encrypted UCS is an `OpenPGP` *symmetric* message (F5 KB K5437) whose
//!   plaintext is that same gzip-tar.
//! * [`openpgp`] — a minimal, dependency-free `OpenPGP` symmetric decryptor
//!   (S2K + AES-CFB + SHA-1 MDC) — entirely pure Rust, no shelling out to `gpg`.
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

pub use openpgp::{OpenPgpError, decrypt_symmetric};
pub use paths::{
    LoadedConfig, PassphraseOptions, PathError, load_paths, read_path, resolve_passphrase,
};
pub use ucs::{
    DEFAULT_PASSPHRASE_ENV, MemberScope, PassphraseProvider, UcsError, UcsFileEntry,
    decrypt_if_encrypted, is_pgp_bytes, is_ucs_bytes, list_ucs_members, read_ucs_member,
    ucs_archive_to_scf, ucs_to_scf,
};
