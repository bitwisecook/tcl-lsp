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

//! The closed value domains of the `SslicTcl` vocabulary.
//!
//! Each table is the exhaustive legal set for the argument it is attached to,
//! so the rows carrying it also list that index in `closed_value_args` and a
//! literal outside the set is reported (W127). The one deliberate exception is
//! [`PROTOCOL_VERSIONS`], which lists the canonical spellings as completion
//! hints while the loader also accepts aliases — an open set.

use crate::hover::ArgValue;

const fn value(value: &'static str, detail: &'static str) -> ArgValue {
    ArgValue {
        value,
        detail,
        ..ArgValue::DEFAULT
    }
}

const fn boolean(value: &'static str, detail: &'static str, code: i64) -> ArgValue {
    ArgValue {
        value,
        detail,
        code: Some(code),
        ..ArgValue::DEFAULT
    }
}

/// `BOOL` — Tcl's own boolean spellings, all eight of them.
pub(super) const BOOLS: &[ArgValue] = &[
    boolean("true", "true", 1),
    boolean("false", "false", 0),
    boolean("yes", "true", 1),
    boolean("no", "false", 0),
    boolean("on", "true", 1),
    boolean("off", "false", 0),
    boolean("1", "true", 1),
    boolean("0", "false", 0),
];

/// `CLIENT` — the root programs a `trust-program` block can restate.
pub(super) const CLIENTS: &[ArgValue] = &[
    value("mozilla", "the Mozilla NSS root program"),
    value("chrome", "the Chrome Root Store"),
    value("apple", "the Apple root program"),
    value("microsoft", "the Microsoft Trusted Root Program"),
    value("android", "the Android system trust store"),
    value("openjdk", "the OpenJDK cacerts trust store"),
];

/// `STATUS` — how a protocol version or cipher suite is rated.
pub(super) const STATUSES: &[ArgValue] = &[
    value("recommended", "preferred; deploy this"),
    value("acceptable", "permitted, but not preferred"),
    value("deprecated", "still interoperable, but on the way out"),
    value("prohibited", "must not be offered"),
];

/// `SEVERITY` — the weight a failing policy check carries.
pub(super) const SEVERITIES: &[ArgValue] = &[
    value("info", "informational only"),
    value("warning", "a finding that does not fail the endpoint"),
    value("error", "a failing finding"),
    value("critical", "a failing finding that overrides the grade"),
];

/// `GRADE` — the assurance grades, best first.
pub(super) const GRADES: &[ArgValue] = &[
    value("A+", "the highest grade"),
    value("A", "a strong configuration"),
    value("B", "a sound configuration with reservations"),
    value("C", "a weak configuration"),
    value("D", "a poor configuration"),
    value("E", "a failing configuration"),
    value("F", "the lowest grade"),
];

/// `VERSION` — the canonical protocol-version spellings.
///
/// **Not** a closed set: the loader also accepts aliases (`TLSv1.2`,
/// `tls12`, …) and normalises them, so these are offered as completions and a
/// spelling outside them is not an error.
pub(super) const PROTOCOL_VERSIONS: &[ArgValue] = &[
    value("ssl2", "SSL 2.0 — broken; never offer it"),
    value("ssl3", "SSL 3.0 — broken; never offer it"),
    value("tls1.0", "TLS 1.0 — deprecated (RFC 8996)"),
    value("tls1.1", "TLS 1.1 — deprecated (RFC 8996)"),
    value("tls1.2", "TLS 1.2"),
    value("tls1.3", "TLS 1.3"),
];

/// The one legal `schema` value of a `testssl-import` block.
pub(super) const TESTSSL_SCHEMAS: &[ArgValue] = &[value(
    "1",
    "the only schema version this vocabulary defines",
)];
