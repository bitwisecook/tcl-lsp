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

//! SSL certificate inventory & expiry (plus the private key that backs each).
//!
//! Answers the everyday "which certs are expiring / already expired, what do
//! they front, and is the private key passphrase-protected?" question (see e.g.
//! the r/f5networks certificate-expiry automation threads) directly from a
//! config export or UCS backup — no device access, no OpenSSL, nothing leaves
//! the browser.
//!
//! Two data sources are combined, file-first:
//!
//! * The **real certificate** parsed out of the UCS filestore. A modern BIG-IP
//!   export carries a *metadata-free* `sys file ssl-cert` stanza — subject /
//!   issuer / validity / SANs live only in the PEM, not in the config — so the
//!   caller reads each cert's `cache-path` member out of the archive and hands
//!   them in as `cert_pems`; the same pure `x509_parse` the engine uses fills
//!   the row and recovers the two things the config never carries: the issue
//!   date (not-before) and the full trust chain.
//! * The `sys file ssl-cert` **config metadata**, used as the fallback when the
//!   file isn't available (a bare `bigip.conf` with no filestore).
//!
//! Each cert is cross-referenced against the SSL profiles (and, through them,
//! the virtual servers) that use it, and its chain is reconstructed by walking
//! issuer → subject across the leaf, any bundled certs and the referenced CA
//! chain. Days-until-expiry is computed live in the browser against the viewer's
//! clock (see `certs.js`), so the tab is accurate whenever the report is opened.

use std::collections::{BTreeSet, HashMap};

use serde_json::{Map, Value as J};
use tcl_bigip::canonical::Canon;
use tcl_bigip::model::ModelObject;
use tcl_bigip::parser::driver::parse_bigip_conf;

use crate::jutil::{bstr, sarr};
use crate::query::Source;

/// A canonical field (snake-cased) as a `&str`.
fn cf<'a>(m: &'a Map<String, J>, key: &str) -> &'a str {
    m.get(key).and_then(J::as_str).unwrap_or("")
}

/// Whether `s` is still an `f5mku` `$M$…` ciphertext (not yet decrypted).
fn is_encrypted(s: &str) -> bool {
    s.starts_with("$M$")
}

/// Strip one layer of surrounding double quotes (a TMSH metadata artifact).
fn unquote(s: &str) -> &str {
    let t = s.trim();
    t.strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(t)
}

/// Pull the CN out of an RFC 4514 distinguished name (`CN=…,O=…`).
fn cn(dn: &str) -> String {
    let dn = unquote(dn);
    // Split on unescaped commas.
    let mut parts: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut esc = false;
    for ch in dn.chars() {
        if esc {
            buf.push(ch);
            esc = false;
        } else if ch == '\\' {
            buf.push(ch);
            esc = true;
        } else if ch == ',' {
            parts.push(std::mem::take(&mut buf));
        } else {
            buf.push(ch);
        }
    }
    parts.push(buf);
    for p in &parts {
        let p = p.trim();
        // Compare the first three *bytes* (byte slicing is always safe);
        // `p[..3]` panics when byte 3 falls inside a multi-byte character
        // (e.g. a DN component `O=émission`). When the prefix is the ASCII
        // "CN=", byte offset 3 is a valid char boundary, so `p[3..]` is safe.
        if p.as_bytes()
            .get(..3)
            .is_some_and(|b| b.eq_ignore_ascii_case(b"CN="))
        {
            return p[3..].to_string();
        }
    }
    String::new()
}

/// ISO-8601 (`2026-08-13T12:47:21+00:00`) → unix-seconds string, or `""`.
fn epoch(iso: &str) -> String {
    if iso.is_empty() {
        return String::new();
    }
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|d| d.timestamp().to_string())
        .unwrap_or_default()
}

/// ISO-8601 → `Aug 13 2026 12:47 UTC`, or `""`.
fn human_date(iso: &str) -> String {
    if iso.is_empty() {
        return String::new();
    }
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|d| d.format("%b %d %Y %H:%M UTC").to_string())
        .unwrap_or_default()
}

/// `RSAPublicKey` → `rsa`; `EC` → `ec` — matches the config wording.
fn key_type_from_x509(key_alg: &str) -> String {
    let a = key_alg.to_ascii_lowercase();
    if a.contains("rsa") {
        "rsa".into()
    } else if a.contains("ec") || a.contains("elliptic") {
        "ec".into()
    } else if a.contains("dsa") {
        "dsa".into()
    } else {
        key_alg.to_string()
    }
}

/// Parse every PEM `CERTIFICATE` block in `pem` to `x509_parse` field maps.
fn x509_dicts(pem: &str) -> Vec<Map<String, J>> {
    tcl_bigip_query::probes::x509_parse_all(pem)
        .iter()
        .filter_map(|v| serde_json::from_str::<J>(&tcl_bigip_query::jsonfmt::to_compact(v)).ok())
        .filter_map(|j| match j {
            J::Object(m) => Some(m),
            _ => None,
        })
        .collect()
}

/// One node of the trust graph, keyed later by its subject DN.
#[derive(Clone)]
struct Node {
    subject: String,
    issuer: String,
    cn: String,
    issuer_cn: String,
    not_after: String,
    self_signed: bool,
}

fn node_of(x: &Map<String, J>) -> Node {
    let subject = cf(x, "subject").to_string();
    let issuer = cf(x, "issuer").to_string();
    let self_signed = !subject.is_empty() && subject == issuer;
    Node {
        cn: cn(&subject),
        issuer_cn: cn(&issuer),
        not_after: cf(x, "not_after").to_string(),
        self_signed,
        subject,
        issuer,
    }
}

/// Walk issuer → subject from `leaf` to a self-signed root (or a gap).
fn resolve_chain(leaf: &Map<String, J>, pool: &HashMap<String, Node>) -> J {
    let start = node_of(leaf);
    if start.subject.is_empty() {
        return J::Array(vec![]);
    }
    let mut chain: Vec<Node> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut cur = Some(start);
    while let Some(node) = cur {
        if seen.contains(&node.subject) {
            break;
        }
        seen.insert(node.subject.clone());
        let stop = node.self_signed;
        let next_issuer = node.issuer.clone();
        chain.push(node);
        if stop {
            break;
        }
        cur = pool.get(&next_issuer).cloned();
    }
    let complete = chain.last().is_some_and(|n| n.self_signed);
    let mut out: Vec<J> = Vec::new();
    for (i, link) in chain.iter().enumerate() {
        let role = if i == 0 {
            "leaf"
        } else if link.self_signed {
            "root"
        } else {
            "intermediate"
        };
        let mut m = Map::new();
        m.insert("role".into(), J::String(role.into()));
        m.insert(
            "cn".into(),
            J::String(if link.cn.is_empty() {
                link.subject.clone()
            } else {
                link.cn.clone()
            }),
        );
        m.insert("subject".into(), J::String(link.subject.clone()));
        m.insert(
            "issuerCn".into(),
            J::String(if link.issuer_cn.is_empty() {
                link.issuer.clone()
            } else {
                link.issuer_cn.clone()
            }),
        );
        m.insert("notAfter".into(), J::String(human_date(&link.not_after)));
        m.insert("selfSigned".into(), J::Bool(link.self_signed));
        out.push(J::Object(m));
    }
    // A trailing non-self-signed link means we couldn't reach the root.
    if let (false, Some(last)) = (complete, chain.last()) {
        let mut m = Map::new();
        m.insert("role".into(), J::String("gap".into()));
        m.insert(
            "cn".into(),
            J::String(if last.issuer_cn.is_empty() {
                last.issuer.clone()
            } else {
                last.issuer_cn.clone()
            }),
        );
        m.insert("subject".into(), J::String(String::new()));
        m.insert("issuerCn".into(), J::String(String::new()));
        m.insert("notAfter".into(), J::String(String::new()));
        m.insert("selfSigned".into(), J::Bool(false));
        out.push(J::Object(m));
    }
    J::Array(out)
}

/// One parsed cert, before cross-referencing.
struct RawCert {
    fields: Map<String, J>,
    full_path: String,
    /// x509 dicts parsed from the filestore PEM (leaf first), if available.
    x509: Vec<Map<String, J>>,
}

pub(crate) fn collect_certs(
    sources: &[Source],
    device: &Map<String, J>,
    cert_pems: &HashMap<String, String>,
) -> J {
    // profile full-path -> [virtual names] using it, for the reverse map.
    let mut profile_to_virtuals: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(J::Array(virtuals)) = device.get("virtuals") {
        for v in virtuals {
            if let Some(vm) = v.as_object() {
                let vname = bstr(vm, "name").to_string();
                for p in sarr(vm, "profiles") {
                    profile_to_virtuals
                        .entry(p.to_string())
                        .or_default()
                        .push(vname.clone());
                }
            }
        }
    }
    let profiles = device.get("profiles").and_then(J::as_array);

    // Parse once, collecting certs (with their real PEM parse) and ssl-keys.
    let mut raw_certs: Vec<RawCert> = Vec::new();
    let mut keys: HashMap<String, Map<String, J>> = HashMap::new();
    for (_uri, scf) in sources {
        let config = parse_bigip_conf(scf, "Common");
        for placed in &config.objects {
            match &placed.object {
                ModelObject::SysFileSslCert(c) => {
                    if let J::Object(fields) = c.canon_fields() {
                        let full_path = {
                            let fp = cf(&fields, "full_path");
                            if fp.is_empty() {
                                placed.full_path.clone()
                            } else {
                                fp.to_string()
                            }
                        };
                        // Locate the PEM: cache-path first, then source-path.
                        let member = {
                            let cp = cf(&fields, "cache_path");
                            if cp.is_empty() {
                                let sp = cf(&fields, "source_path");
                                sp.strip_prefix("file://").unwrap_or(sp).to_string()
                            } else {
                                cp.to_string()
                            }
                        };
                        let x509 = cert_pems
                            .get(&member)
                            .map(|pem| x509_dicts(pem))
                            .unwrap_or_default();
                        raw_certs.push(RawCert {
                            fields,
                            full_path,
                            x509,
                        });
                    }
                }
                ModelObject::SysFileSslKey(k) => {
                    if let J::Object(fields) = k.canon_fields() {
                        let fp = cf(&fields, "full_path");
                        let fp = if fp.is_empty() {
                            placed.full_path.clone()
                        } else {
                            fp.to_string()
                        };
                        keys.insert(fp, fields);
                    }
                }
                _ => {}
            }
        }
    }

    // Trust-graph pool: subject DN -> node, over every leaf + embedded cert.
    let mut pool: HashMap<String, Node> = HashMap::new();
    for rc in &raw_certs {
        for x in &rc.x509 {
            let n = node_of(x);
            if !n.subject.is_empty() {
                pool.entry(n.subject.clone()).or_insert(n);
            }
        }
    }

    let mut certs = Vec::new();
    for rc in &raw_certs {
        let f = &rc.fields;
        let full_path = rc.full_path.clone();
        let leaf = rc.x509.first();
        let name = {
            let n = cf(f, "name");
            if n.is_empty() {
                full_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&full_path)
                    .to_string()
            } else {
                n.to_string()
            }
        };

        // Which profiles use this cert (as server/client cert or chain)?
        let mut used_profiles: BTreeSet<String> = BTreeSet::new();
        let mut used_virtuals: BTreeSet<String> = BTreeSet::new();
        let mut key_candidates: Vec<String> = Vec::new();
        if let Some(profiles) = profiles {
            for p in profiles {
                if let Some(pm) = p.as_object()
                    && (bstr(pm, "cert") == full_path || bstr(pm, "chain") == full_path)
                {
                    let pfp = bstr(pm, "fullPath").to_string();
                    used_profiles.insert(bstr(pm, "name").to_string());
                    let key = bstr(pm, "key");
                    if !key.is_empty() {
                        key_candidates.push(key.to_string());
                    }
                    if let Some(vs) = profile_to_virtuals.get(&pfp) {
                        for vn in vs {
                            used_virtuals.insert(vn.clone());
                        }
                    }
                }
            }
        }
        if let Some(stem) = full_path.strip_suffix(".crt") {
            key_candidates.push(format!("{stem}.key"));
        }
        let key_fields = key_candidates.iter().find_map(|kp| keys.get(kp));

        // File-first, config-fallback for every field the real cert carries.
        let x_get = |k: &str| leaf.map_or("", |m| cf(m, k));
        let subject = {
            let x = x_get("subject");
            if x.is_empty() {
                unquote(cf(f, "subject")).to_string()
            } else {
                x.to_string()
            }
        };
        let issuer = {
            let x = x_get("issuer");
            if x.is_empty() {
                unquote(cf(f, "issuer")).to_string()
            } else {
                x.to_string()
            }
        };
        let san = {
            let sans = leaf.and_then(|m| m.get("sans")).and_then(J::as_array);
            if let Some(arr) = sans.filter(|a| !a.is_empty()) {
                arr.iter()
                    .filter_map(J::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                unquote(cf(f, "subject_alternative_name")).to_string()
            }
        };
        let not_after_iso = x_get("not_after");
        let exp_string = {
            let cfg = unquote(cf(f, "expiration_string"));
            if cfg.is_empty() {
                human_date(not_after_iso)
            } else {
                cfg.to_string()
            }
        };
        let exp_epoch = {
            let cfg = cf(f, "expiration_date");
            if cfg.is_empty() {
                epoch(not_after_iso)
            } else {
                cfg.to_string()
            }
        };
        let serial = {
            let x = x_get("serial");
            if x.is_empty() {
                cf(f, "serial_number").to_string()
            } else {
                x.to_string()
            }
        };
        let key_type = {
            let cfg = cf(f, "key_type");
            if cfg.is_empty() {
                key_type_from_x509(x_get("key_alg"))
            } else {
                cfg.to_string()
            }
        };
        let key_size = {
            let cfg = cf(f, "key_size");
            if cfg.is_empty() {
                leaf.and_then(|m| m.get("key_size"))
                    .and_then(J::as_i64)
                    .map(|n| n.to_string())
                    .unwrap_or_default()
            } else {
                cfg.to_string()
            }
        };
        let fingerprint = {
            let x = x_get("fingerprint_sha256");
            if x.is_empty() {
                cf(f, "fingerprint").to_string()
            } else {
                x.to_string()
            }
        };
        let is_bundle = {
            let cfg = cf(f, "is_bundle");
            if !cfg.is_empty() {
                cfg.to_string()
            } else if rc.x509.len() > 1 {
                "true".into()
            } else {
                String::new()
            }
        };
        let chain = leaf.map_or(J::Array(vec![]), |m| resolve_chain(m, &pool));

        let (key_path, key_sec, key_pass) = key_fields.map_or(("", "", ""), |kf| {
            (
                cf(kf, "full_path"),
                cf(kf, "security_type"),
                cf(kf, "passphrase"),
            )
        });

        let mut cert = Map::new();
        cert.insert("name".into(), J::String(name));
        cert.insert("fullPath".into(), J::String(full_path));
        cert.insert("cn".into(), J::String(cn(&subject)));
        cert.insert("subject".into(), J::String(subject));
        cert.insert("issuerCn".into(), J::String(cn(&issuer)));
        cert.insert("issuer".into(), J::String(issuer));
        cert.insert(
            "notBefore".into(),
            J::String(human_date(x_get("not_before"))),
        );
        cert.insert("expirationString".into(), J::String(exp_string));
        cert.insert("expirationDate".into(), J::String(exp_epoch));
        cert.insert("fingerprint".into(), J::String(fingerprint));
        cert.insert("keyType".into(), J::String(key_type));
        cert.insert("keySize".into(), J::String(key_size));
        cert.insert("sigAlg".into(), J::String(x_get("sig_alg").into()));
        cert.insert("serialNumber".into(), J::String(serial));
        cert.insert("subjectAlternativeName".into(), J::String(san));
        cert.insert("isBundle".into(), J::String(is_bundle));
        cert.insert("sourcePath".into(), J::String(cf(f, "source_path").into()));
        cert.insert("fromCertFile".into(), J::Bool(leaf.is_some()));
        cert.insert("chain".into(), chain);

        cert.insert("keyPath".into(), J::String(key_path.into()));
        cert.insert("keySecurityType".into(), J::String(key_sec.into()));
        cert.insert("keyPassphrase".into(), J::String(key_pass.into()));
        cert.insert(
            "keyPassphraseEncrypted".into(),
            J::Bool(is_encrypted(key_pass)),
        );
        cert.insert("hasKey".into(), J::Bool(key_fields.is_some()));
        cert.insert(
            "usedByProfiles".into(),
            J::Array(used_profiles.into_iter().map(J::String).collect()),
        );
        cert.insert(
            "usedByVirtuals".into(),
            J::Array(used_virtuals.into_iter().map(J::String).collect()),
        );
        certs.push(J::Object(cert));
    }

    J::Array(certs)
}

#[cfg(test)]
mod tests {
    use super::cn;

    #[test]
    fn cn_basic() {
        assert_eq!(cn("CN=example.com,O=Acme"), "example.com");
        assert_eq!(cn("cn=lower"), "lower");
    }

    #[test]
    fn cn_no_cn_component() {
        assert_eq!(cn("O=Acme,C=US"), "");
    }

    #[test]
    fn cn_multibyte_component_does_not_panic() {
        // A DN component whose 3rd byte is inside a multi-byte
        // character (`O=émission`, where `é` occupies bytes 2-3) must not
        // panic the CN scan.
        assert_eq!(cn("O=émission,CN=host.example.com"), "host.example.com");
        // A component shorter than 3 bytes and a bare multi-byte string.
        assert_eq!(cn("é"), "");
        assert_eq!(cn("O=é"), "");
    }
}
