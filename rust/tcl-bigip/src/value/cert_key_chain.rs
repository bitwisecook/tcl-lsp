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

//! Typed BIG-IP cert-key-chain entry.

use std::fmt;

/// One `cert-key-chain` keyed-block entry.
///
/// `name` is the entry's bare name. `cert` / `key` / `chain` /
/// `passphrase` are the optional sub-references; empty when the
/// corresponding line was omitted. `raw` preserves the original body.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct CertKeyChain {
    /// Entry name (the keyed-block key).
    pub name: String,
    /// `cert` sub-reference.
    pub cert: String,
    /// `key` sub-reference.
    pub key: String,
    /// `chain` sub-reference.
    pub chain: String,
    /// `passphrase` value.
    pub passphrase: String,
    /// Original brace-body text (stripped).
    pub raw: String,
}

impl CertKeyChain {
    /// Build an entry from the keyed-block parser output.
    #[must_use]
    pub fn from_raw(name: &str, body: &str) -> Self {
        let (mut cert, mut key, mut chain, mut passphrase) =
            (String::new(), String::new(), String::new(), String::new());
        let tokens: Vec<&str> = body.split_whitespace().collect();
        for (i, tok) in tokens.iter().enumerate() {
            if i + 1 >= tokens.len() {
                continue;
            }
            let value = tokens[i + 1].to_owned();
            match *tok {
                "cert" => cert = value,
                "key" => key = value,
                "chain" => chain = value,
                "passphrase" => passphrase = value,
                _ => {}
            }
        }
        Self {
            name: name.to_owned(),
            cert,
            key,
            chain,
            passphrase,
            raw: body.trim().to_owned(),
        }
    }

    /// Yield `(kind, full-path)` pairs for every sub-reference, in a
    /// stable order.
    #[must_use]
    pub fn references(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        if !self.cert.is_empty() {
            out.push(("sys file ssl-cert".to_owned(), self.cert.clone()));
        }
        if !self.key.is_empty() {
            out.push(("sys file ssl-key".to_owned(), self.key.clone()));
        }
        if !self.chain.is_empty() {
            out.push(("sys file ssl-cert".to_owned(), self.chain.clone()));
        }
        out
    }
}

impl fmt::Display for CertKeyChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.raw.is_empty() {
            return write!(f, "{} {{ {} }}", self.name, self.raw);
        }
        let mut parts: Vec<String> = Vec::new();
        if !self.cert.is_empty() {
            parts.push(format!("cert {}", self.cert));
        }
        if !self.key.is_empty() {
            parts.push(format!("key {}", self.key));
        }
        if !self.chain.is_empty() {
            parts.push(format!("chain {}", self.chain));
        }
        if !self.passphrase.is_empty() {
            parts.push(format!("passphrase {}", self.passphrase));
        }
        let body = parts.join(" ");
        if body.is_empty() {
            write!(f, "{} {{ }}", self.name)
        } else {
            write!(f, "{} {{ {body} }}", self.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_raw_and_references() {
        let c = CertKeyChain::from_raw(
            "cert_a",
            "cert /Common/cert_a.crt key /Common/cert_a.key chain /Common/ca_bundle.crt",
        );
        assert_eq!(c.cert, "/Common/cert_a.crt");
        assert_eq!(c.key, "/Common/cert_a.key");
        assert_eq!(c.chain, "/Common/ca_bundle.crt");
        assert_eq!(c.references().len(), 3);
        assert_eq!(
            c.to_string(),
            "cert_a { cert /Common/cert_a.crt key /Common/cert_a.key chain /Common/ca_bundle.crt }"
        );
    }

    #[test]
    fn structured_render_when_no_raw() {
        let c = CertKeyChain {
            name: "cert_b".to_owned(),
            cert: "/Common/cert_b.crt".to_owned(),
            key: "/Common/cert_b.key".to_owned(),
            ..Default::default()
        };
        assert_eq!(
            c.to_string(),
            "cert_b { cert /Common/cert_b.crt key /Common/cert_b.key }"
        );
    }
}
