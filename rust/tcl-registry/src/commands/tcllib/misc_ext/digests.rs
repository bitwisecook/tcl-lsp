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

//! The incremental message-digest APIs of `md5`, `sha1` and `sha256`.

use super::{Arity, Row};

/// The `md5` package.
pub(super) const MD5_CMDS: &[Row] = &[
    (
        "md5::MD5Init",
        Arity::exact(0),
        &["md5::MD5Init"],
        "Begins a new MD5 hash.",
    ),
    (
        "md5::MD5Update",
        Arity::exact(2),
        &["md5::MD5Update token data"],
        "Add data to the hash identified by token.",
    ),
    (
        "md5::MD5Final",
        Arity::exact(1),
        &["md5::MD5Final token"],
        "Returns the hash value and releases any resources held by this token.",
    ),
    (
        "md5::HMACInit",
        Arity::exact(1),
        &["md5::HMACInit key"],
        "This is equivalent to the ::md5::MD5Init command except that it requires the key that will be included in the HMA",
    ),
    (
        "md5::HMACUpdate",
        Arity::exact(2),
        &["md5::HMACUpdate token data"],
        "tcllib command.",
    ),
    (
        "md5::HMACFinal",
        Arity::exact(1),
        &["md5::HMACFinal token"],
        "These commands are identical to the MD5 equivalent commands.",
    ),
];

// The `report` package's commands all carry dedicated specs (arg roles, hover
// snippets, the `report::report` object-method class, and the
// `report::defstyle` style-script scoped environment) that the flat `Row` table
// cannot express — see `report__report.rs`, `report__defstyle.rs`,
// `report__rmstyle.rs`, `report__stylearguments.rs`, `report__stylebody.rs`,
// and `report__styles.rs`.

/// The `sha1` package.
pub(super) const SHA1_CMDS: &[Row] = &[
    (
        "sha1::hmac",
        Arity::exact(2),
        &["sha1::hmac key string"],
        "tcllib command.",
    ),
    (
        "sha1::SHA1Init",
        Arity::exact(0),
        &["sha1::SHA1Init"],
        "Begins a new SHA1 hash.",
    ),
    (
        "sha1::SHA1Update",
        Arity::exact(2),
        &["sha1::SHA1Update token data"],
        "Add data to the hash identified by token.",
    ),
    (
        "sha1::SHA1Final",
        Arity::exact(1),
        &["sha1::SHA1Final token"],
        "Returns the hash value and releases any resources held by this token.",
    ),
    (
        "sha1::HMACInit",
        Arity::exact(1),
        &["sha1::HMACInit key"],
        "This is equivalent to the ::sha1::SHA1Init command except that it requires the key that will be included in the H",
    ),
    (
        "sha1::HMACUpdate",
        Arity::exact(2),
        &["sha1::HMACUpdate token data"],
        "tcllib command.",
    ),
    (
        "sha1::HMACFinal",
        Arity::exact(1),
        &["sha1::HMACFinal token"],
        "These commands are identical to the SHA1 equivalent commands.",
    ),
];

/// The `sha2` package.
pub(super) const SHA2_CMDS: &[Row] = &[
    (
        "sha2::hmac",
        Arity::exact(2),
        &["sha2::hmac key string"],
        "tcllib command.",
    ),
    (
        "sha2::SHA256Init",
        Arity::exact(0),
        &["sha2::SHA256Init"],
        "tcllib command.",
    ),
    (
        "sha2::SHA224Init",
        Arity::exact(0),
        &["sha2::SHA224Init"],
        "Begins a new SHA256/SHA224 hash.",
    ),
    (
        "sha2::SHA256Update",
        Arity::exact(2),
        &["sha2::SHA256Update token data"],
        "Add data to the hash identified by token.",
    ),
    (
        "sha2::SHA256Final",
        Arity::exact(1),
        &["sha2::SHA256Final token"],
        "tcllib command.",
    ),
    (
        "sha2::SHA224Final",
        Arity::exact(1),
        &["sha2::SHA224Final token"],
        "Returns the hash value and releases any resources held by this token.",
    ),
    (
        "sha2::HMACInit",
        Arity::exact(1),
        &["sha2::HMACInit key"],
        "This is equivalent to the ::sha2::SHA256Init command except that it requires the key that will be included in the",
    ),
    (
        "sha2::HMACUpdate",
        Arity::exact(2),
        &["sha2::HMACUpdate token data"],
        "tcllib command.",
    ),
    (
        "sha2::HMACFinal",
        Arity::exact(1),
        &["sha2::HMACFinal token"],
        "These commands are identical to the SHA256 equivalent commands.",
    ),
];
