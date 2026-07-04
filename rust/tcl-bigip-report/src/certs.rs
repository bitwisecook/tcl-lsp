//! SSL certificate inventory & expiry (plus the private key that backs each).
//!
//! Answers the everyday "which certs are expiring / already expired, what do
//! they front, and is the private key passphrase-protected?" question (see e.g.
//! the r/f5networks certificate-expiry automation threads) directly from a
//! config export or UCS backup — no device access, no OpenSSL, nothing leaves
//! the browser.
//!
//! BIG-IP stores certificate metadata *in the config itself*: every
//! `sys file ssl-cert` stanza carries `expiration-string`, `expiration-date`
//! (epoch seconds), `subject`, `issuer`, `fingerprint`, `key-type`/`key-size`
//! and the subject-alternative-name list; the paired `sys file ssl-key` carries
//! the private-key `security-type` and its (master-key-encrypted) `passphrase`.
//! When the report is generated with the unit master key (`f5mku -K`), those
//! `$M$…` passphrases are decrypted upstream (see [`crate::decrypt_secrets`]) so
//! this tab shows them in clear.
//!
//! The `f5-query` DSL only projects the `ltm` module, so — unlike the rest of
//! the report — the certificate/key list is read from the parsed [`tcl_bigip`]
//! model directly (via the same `parse_bigip_conf` the engine is built on)
//! rather than through a query. Each cert is cross-referenced against the SSL
//! profiles (and, through them, the virtual servers) that use it. Days-until-
//! expiry is computed live in the browser against the viewer's clock (see
//! `certs.js`), so the tab is accurate whenever the report is opened.

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

/// One parsed cert, before cross-referencing.
struct RawCert {
    fields: Map<String, J>,
    full_path: String,
}

pub(crate) fn collect_certs(sources: &[Source], device: &Map<String, J>) -> J {
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

    // Parse once, collecting certs and the ssl-key objects (by full-path).
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
                        raw_certs.push(RawCert { fields, full_path });
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

    let mut certs = Vec::new();
    for rc in raw_certs {
        let f = &rc.fields;
        let full_path = rc.full_path;
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

        // Which profiles use this cert (as server/client cert or chain)? Collect
        // the key paths those profiles pair with it, too.
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
        // Fallback: pair `foo.crt` with `foo.key` by name.
        if let Some(stem) = full_path.strip_suffix(".crt") {
            key_candidates.push(format!("{stem}.key"));
        }
        let key_fields = key_candidates.iter().find_map(|kp| keys.get(kp));

        let mut cert = Map::new();
        cert.insert("name".into(), J::String(name));
        cert.insert("fullPath".into(), J::String(full_path));
        cert.insert("subject".into(), J::String(cf(f, "subject").into()));
        cert.insert("issuer".into(), J::String(cf(f, "issuer").into()));
        cert.insert(
            "expirationString".into(),
            J::String(cf(f, "expiration_string").into()),
        );
        cert.insert(
            "expirationDate".into(),
            J::String(cf(f, "expiration_date").into()),
        );
        cert.insert("fingerprint".into(), J::String(cf(f, "fingerprint").into()));
        cert.insert("keyType".into(), J::String(cf(f, "key_type").into()));
        cert.insert("keySize".into(), J::String(cf(f, "key_size").into()));
        cert.insert(
            "serialNumber".into(),
            J::String(cf(f, "serial_number").into()),
        );
        cert.insert(
            "subjectAlternativeName".into(),
            J::String(cf(f, "subject_alternative_name").into()),
        );
        cert.insert("isBundle".into(), J::String(cf(f, "is_bundle").into()));
        cert.insert("sourcePath".into(), J::String(cf(f, "source_path").into()));

        // Private-key pairing + passphrase (decrypted when the master key was
        // supplied at generation time; otherwise the `$M$…` ciphertext).
        let (key_path, key_sec, key_pass) = key_fields.map_or(("", "", ""), |kf| {
            (
                cf(kf, "full_path"),
                cf(kf, "security_type"),
                cf(kf, "passphrase"),
            )
        });
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
