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

//! Live network-probe layer.
//!
//! Two surfaces, with different output guarantees:
//!
//! - **Byte-for-byte (golden-tested)**: [`x509_parse`] turns a PEM cert into the
//!   x509 parse dict (subject / issuer / serial
//!   / validity / SANs / fingerprint / key-alg / key-size / sig-alg / version
//!   / public-key PEM), plus [`x509_eq`] and [`x509_from_config`] which are
//!   deterministic projections. The `--enable-probes` gating error is also
//!   byte-for-byte.
//! - **Faithful-but-not-golden**: the live network probes (`dns`, `rev_dns`,
//!   `ping`, `portping`, `traceroute`, `socket_get`, `tls_handshake`, the
//!   `url_*` HTTP family). These do real I/O with a structure / output shape
//!   matching the reference output, but are not asserted byte-for-byte against
//!   live results (the test env has no reliable network).
//!
//! Every network probe is gated by `ctx.probes_enabled` (the `--enable-probes`
//! flag). The pure x509 helpers are NOT gated.

#[cfg(feature = "probes")]
use std::io::{Read as _, Write as _};
#[cfg(feature = "probes")]
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
#[cfg(feature = "probes")]
use std::time::{Duration, Instant};

use base64::Engine as _;
use indexmap::IndexMap;
use sha2::{Digest as _, Sha256};
use x509_parser::prelude::*;
use x509_parser::public_key::PublicKey;

use crate::errors::QueryError;
use crate::value::Value;

/// The `--enable-probes` gate. Reproduces the error text verbatim.
#[cfg(feature = "probes")]
pub(crate) fn require_probes(name: &str, enabled: bool) -> Result<(), QueryError> {
    if enabled {
        Ok(())
    } else {
        Err(QueryError::builtin(format!(
            "{name}: network probes are disabled; pass --enable-probes \
             to opt in for this query.  Probes always go to the network \
             and are gated by default so an offline read-only query \
             never accidentally reaches out."
        )))
    }
}

// x509_parse — the deterministic, byte-for-byte surface

/// Parse a PEM certificate into the rich x509 parse dict.
///
/// Produces the x509 parse dict (subject / issuer / serial / … keys).
/// Raises [`QueryError::Builtin`] for input that doesn't decode as a cert,
/// reproducing `not a PEM certificate (...)` wording shape.
///
/// Public so the CLI's `ucs_cert` reader (which reads a PEM out of a UCS in a
/// higher layer) can produce the same `x509_parse`-shaped value the builtin
/// returns.
pub fn x509_parse(pem: &str) -> Result<Value, QueryError> {
    let der = match x509_parser::pem::parse_x509_pem(pem.as_bytes()) {
        Ok((_, pem)) => pem.contents,
        Err(e) => {
            return Err(QueryError::builtin(format!(
                "x509_parse: not a PEM certificate ({e})"
            )));
        }
    };
    x509_parse_der(&der)
}

/// Parse every PEM `CERTIFICATE` block in a file into `x509_parse` dicts.
///
/// A BIG-IP filestore entry is frequently a *bundle* — the leaf followed by its
/// intermediate(s) and (sometimes) the root, all concatenated. The report's
/// chain view needs every link, so this returns one dict per parseable
/// `CERTIFICATE` block in document order (leaf first). Blocks that aren't
/// certificates (or don't decode) are skipped rather than failing the whole
/// read, so a stray key/DH-params block in the same file is tolerated.
#[must_use]
pub fn x509_parse_all(pem: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for item in Pem::iter_from_buffer(pem.as_bytes()) {
        let Ok(block) = item else { break };
        if block.label != "CERTIFICATE" {
            continue;
        }
        if let Ok(v) = x509_parse_der(&block.contents) {
            out.push(v);
        }
    }
    out
}

/// Parse a single DER cert into the `x509_parse`-shaped dict.
pub(crate) fn x509_parse_der(der: &[u8]) -> Result<Value, QueryError> {
    let (_, cert) = X509Certificate::from_der(der)
        .map_err(|e| QueryError::builtin(format!("x509_parse: not a PEM certificate ({e})")))?;

    let fingerprint = hex_upper(&Sha256::digest(der));

    let mut m: IndexMap<String, Value> = IndexMap::new();
    m.insert(
        "subject".to_owned(),
        Value::Str(rfc4514_string(cert.subject())),
    );
    m.insert(
        "issuer".to_owned(),
        Value::Str(rfc4514_string(cert.issuer())),
    );
    m.insert(
        "not_before".to_owned(),
        Value::Str(iso_utc(cert.validity().not_before.timestamp())),
    );
    m.insert(
        "not_after".to_owned(),
        Value::Str(iso_utc(cert.validity().not_after.timestamp())),
    );
    m.insert("serial".to_owned(), Value::Str(serial_hex(&cert)));
    m.insert("fingerprint_sha256".to_owned(), Value::Str(fingerprint));
    m.insert("sans".to_owned(), Value::List(collect_sans(&cert)));

    let spki = &cert.tbs_certificate.subject_pki;
    let (key_alg, key_size) = key_alg_and_size(spki);
    m.insert(
        "key_alg".to_owned(),
        key_alg.map_or(Value::Null, Value::Str),
    );
    m.insert(
        "key_size".to_owned(),
        key_size.map_or(Value::Null, |s| Value::Int(i64::from(s))),
    );
    m.insert(
        "sig_alg".to_owned(),
        Value::Str(sig_alg_name(
            &cert.signature_algorithm.algorithm.to_id_string(),
        )),
    );
    m.insert(
        "version".to_owned(),
        Value::Str(version_name(cert.version.0)),
    );
    m.insert(
        "public_key_pem".to_owned(),
        Value::Str(spki_to_pem(spki.raw)),
    );
    Ok(Value::Object(m))
}

fn hex_upper(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02X}");
    }
    s
}

/// Format a UTC unix timestamp as ISO-8601 for a tz-aware UTC datetime:
/// `YYYY-MM-DDTHH:MM:SS+00:00`.
fn iso_utc(timestamp: i64) -> String {
    use chrono::{TimeZone as _, Utc};
    let dt = Utc.timestamp_opt(timestamp, 0).single().unwrap_or_default();
    dt.format("%Y-%m-%dT%H:%M:%S+00:00").to_string()
}

/// Serial as uppercase hex with no leading zeros (e.g. `12345678`, `3E8`).
fn serial_hex(cert: &X509Certificate<'_>) -> String {
    // `tbs_certificate.serial` is a `BigUint`. `{:X}` gives uppercase hex with
    // no leading zeros. Zero serials render as `0`.
    format!("{:X}", cert.tbs_certificate.serial)
}

/// Render an RFC 4514 string from an x509-parser `X509Name`.
///
/// An RFC 4514 string emits the most-specific RDN first (CN before
/// O before C), short attribute names (CN / O / OU / ST / C / …), and a `,`
/// separator with no surrounding space. x509-parser iterates RDNs in
/// least-specific-first (C, ST, O, CN) order, so we reverse.
fn rfc4514_string(name: &X509Name<'_>) -> String {
    let mut parts: Vec<String> = Vec::new();
    for rdn in name.iter() {
        // A single RDN can carry multiple attribute-value pairs (joined with
        // `+` in RFC 4514); the certs we target carry one each.
        let mut piece: Vec<String> = Vec::new();
        for attr in rdn.iter() {
            let key = rdn_short_name(&attr.attr_type().to_id_string());
            let val = attr.attr_value().as_str().map_or_else(
                |_| attr.attr_value().as_bytes().escape_ascii().to_string(),
                ToOwned::to_owned,
            );
            piece.push(format!("{key}={}", rfc4514_escape(&val)));
        }
        parts.push(piece.join("+"));
    }
    parts.reverse();
    parts.join(",")
}

/// Escape a value for an RFC 4514 string:
/// leading `#`, leading / trailing space, and the set `,+"\<>;` are
/// backslash-escaped. Sufficient for the cert subjects we target.
fn rfc4514_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let chars: Vec<char> = value.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let at_start = i == 0;
        let at_end = i == chars.len() - 1;
        let needs = matches!(c, ',' | '+' | '"' | '\\' | '<' | '>' | ';')
            || (at_start && (c == '#' || c == ' '))
            || (at_end && c == ' ');
        if needs {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Map an attribute-type OID to the short name the RFC 4514 string
/// uses. Unknown OIDs fall back to the dotted string
/// (emitted as `OID.dotted=value` — but the common DN attributes are covered).
fn rdn_short_name(oid: &str) -> String {
    match oid {
        "2.5.4.3" => "CN",
        "2.5.4.6" => "C",
        "2.5.4.7" => "L",
        "2.5.4.8" => "ST",
        "2.5.4.9" => "STREET",
        "2.5.4.10" => "O",
        "2.5.4.11" => "OU",
        "2.5.4.5" => "2.5.4.5",
        "0.9.2342.19200300.100.1.25" => "DC",
        "0.9.2342.19200300.100.1.1" => "UID",
        _ => return oid.to_owned(),
    }
    .to_owned()
}

/// Collect SANs as the string form of each general name:
/// DNS names and the dotted-string IP form, in cert order.
fn collect_sans(cert: &X509Certificate<'_>) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(Some(ext)) = cert.subject_alternative_name() {
        for gn in &ext.value.general_names {
            let s = match gn {
                GeneralName::DNSName(d) => (*d).to_owned(),
                GeneralName::RFC822Name(e) => (*e).to_owned(),
                GeneralName::URI(u) => (*u).to_owned(),
                GeneralName::IPAddress(bytes) => ip_bytes_to_string(bytes),
                GeneralName::DirectoryName(n) => rfc4514_string(n),
                other => format!("{other:?}"),
            };
            out.push(Value::Str(s));
        }
    }
    out
}

/// Render an IP from its SAN octet form to its canonical string.
fn ip_bytes_to_string(bytes: &[u8]) -> String {
    match bytes.len() {
        4 => std::net::Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]).to_string(),
        16 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(bytes);
            std::net::Ipv6Addr::from(octets).to_string()
        }
        _ => bytes.escape_ascii().to_string(),
    }
}

/// `(key_alg, key_size)` where `key_alg` is the public key's type name and
/// `key_size` is its bit length (or `None` when unavailable).
fn key_alg_and_size(spki: &SubjectPublicKeyInfo<'_>) -> (Option<String>, Option<u32>) {
    let oid = spki.algorithm.algorithm.to_id_string();
    let key_alg = pubkey_class_name(&oid);
    let key_size = match spki.parsed() {
        Ok(PublicKey::RSA(rsa)) => Some(rsa.key_size() as u32),
        Ok(PublicKey::EC(ec)) => Some(ec_curve_bits(spki).unwrap_or_else(|| ec.key_size() as u32)),
        Ok(PublicKey::DSA(_)) => dsa_key_size(spki),
        // Ed25519 / Ed448 have no key-size → None.
        _ => None,
    };
    (key_alg, key_size)
}

/// Map a public-key OID to the public key's type name.
fn pubkey_class_name(oid: &str) -> Option<String> {
    let name = match oid {
        "1.2.840.113549.1.1.1" => "RSAPublicKey",
        "1.2.840.10045.2.1" => "ECPublicKey",
        "1.2.840.10040.4.1" => "DSAPublicKey",
        "1.3.101.112" => "Ed25519PublicKey",
        "1.3.101.113" => "Ed448PublicKey",
        "1.3.101.110" => "X25519PublicKey",
        "1.3.101.111" => "X448PublicKey",
        _ => return None,
    };
    Some(name.to_owned())
}

/// EC key size from the named-curve OID — the field size in bits
/// (521 for P-521, not the 528 a byte-rounded measure gives).
fn ec_curve_bits(spki: &SubjectPublicKeyInfo<'_>) -> Option<u32> {
    let oid = spki.algorithm.parameters.as_ref()?.as_oid().ok()?;
    Some(match oid.to_id_string().as_str() {
        "1.2.840.10045.3.1.1" => 192, // prime192v1 / secp192r1
        "1.3.132.0.33" => 224,        // secp224r1
        "1.2.840.10045.3.1.7" => 256, // prime256v1 / secp256r1
        "1.3.132.0.10" => 256,        // secp256k1
        "1.3.132.0.34" => 384,        // secp384r1
        "1.3.132.0.35" => 521,        // secp521r1
        _ => return None,
    })
}

/// DSA key size — the bit length of the prime `p` parameter.
fn dsa_key_size(spki: &SubjectPublicKeyInfo<'_>) -> Option<u32> {
    use x509_parser::der_parser::asn1_rs::{FromDer, Integer};
    let params = spki.algorithm.parameters.as_ref()?;
    let seq = params.as_sequence().ok()?;
    // The first element of the DSS-Parms sequence is the prime `p`; its bit
    // length is the DSA key size.
    let (_, p) = Integer::from_der(&seq.content).ok()?;
    Some(be_int_bit_length(p.as_ref()))
}

/// Bit length of a big-endian two's-complement DER `INTEGER`'s magnitude —
/// drop the leading sign byte, then count from the highest set bit.
fn be_int_bit_length(bytes: &[u8]) -> u32 {
    // Skip leading 0x00 padding bytes (DER inserts one for a positive integer
    // whose top bit is set).
    let trimmed = bytes
        .iter()
        .skip_while(|&&b| b == 0)
        .copied()
        .collect::<Vec<u8>>();
    match trimmed.first() {
        None => 0,
        Some(&first) => (trimmed.len() as u32 - 1) * 8 + (8 - first.leading_zeros()),
    }
}

/// Map a signature-algorithm OID to its short name.
fn sig_alg_name(oid: &str) -> String {
    let name = match oid {
        "1.2.840.113549.1.1.4" => "md5WithRSAEncryption",
        "1.2.840.113549.1.1.5" => "sha1WithRSAEncryption",
        "1.2.840.113549.1.1.11" => "sha256WithRSAEncryption",
        "1.2.840.113549.1.1.12" => "sha384WithRSAEncryption",
        "1.2.840.113549.1.1.13" => "sha512WithRSAEncryption",
        "1.2.840.113549.1.1.14" => "sha224WithRSAEncryption",
        "1.2.840.113549.1.1.10" => "rsassaPss",
        "1.2.840.10045.4.1" => "ecdsa-with-SHA1",
        "1.2.840.10045.4.3.1" => "ecdsa-with-SHA224",
        "1.2.840.10045.4.3.2" => "ecdsa-with-SHA256",
        "1.2.840.10045.4.3.3" => "ecdsa-with-SHA384",
        "1.2.840.10045.4.3.4" => "ecdsa-with-SHA512",
        "1.2.840.10040.4.3" => "dsa-with-sha1",
        "2.16.840.1.101.3.4.3.1" => "dsa-with-sha224",
        "2.16.840.1.101.3.4.3.2" => "dsa-with-sha256",
        "1.3.101.112" => "ed25519",
        "1.3.101.113" => "ed448",
        // Unrecognised OIDs map to "Unknown OID".
        _ => return "Unknown OID".to_owned(),
    };
    name.to_owned()
}

/// Map the DER version byte (0-indexed) to the certificate version name
/// (`v1` / `v3`).
fn version_name(raw: u32) -> String {
    match raw {
        0 => "v1",
        1 => "v2",
        2 => "v3",
        other => return format!("v{}", other + 1),
    }
    .to_owned()
}

/// Wrap a `SubjectPublicKeyInfo` DER blob as a `-----BEGIN PUBLIC KEY-----`
/// PEM (the `SubjectPublicKeyInfo` DER base64-wrapped in PEM armor).
fn spki_to_pem(spki_der: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(spki_der);
    let mut out = String::from("-----BEGIN PUBLIC KEY-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).expect("base64 is ASCII"));
        out.push('\n');
    }
    out.push_str("-----END PUBLIC KEY-----\n");
    out
}

// x509_eq / x509_from_config — deterministic projections

/// Structural equality of two parsed x509 dicts.
#[cfg(feature = "probes")]
pub(crate) fn x509_eq(left: &Value, right: &Value) -> bool {
    let (Value::Object(l), Value::Object(r)) = (as_dict(left), as_dict(right)) else {
        return crate::value::py_eq(left, right, 0);
    };
    let lfp = dict_str(&l, "fingerprint_sha256");
    let rfp = dict_str(&r, "fingerprint_sha256");
    if !lfp.is_empty() && !rfp.is_empty() {
        return lfp.eq_ignore_ascii_case(&rfp);
    }
    dict_str(&l, "subject") == dict_str(&r, "subject")
        && dict_str(&l, "issuer") == dict_str(&r, "issuer")
        && dict_str(&l, "serial").eq_ignore_ascii_case(&dict_str(&r, "serial"))
}

#[cfg(feature = "probes")]
fn as_dict(v: &Value) -> Value {
    match v {
        Value::ObjectRef(o) => Value::Object(o.fields.clone()),
        other => other.clone(),
    }
}

#[cfg(feature = "probes")]
fn dict_str(map: &IndexMap<String, Value>, key: &str) -> String {
    match map.get(key) {
        Some(Value::Str(s)) => s.clone(),
        Some(Value::PathRef(p)) => p.full_path.clone(),
        _ => String::new(),
    }
}

/// Map BIG-IP `key-type` tokens to the `key_alg` strings the x509 parse
/// uses.
#[cfg(feature = "probes")]
fn key_type_to_key_alg(key_type: &str) -> Option<&'static str> {
    Some(match key_type {
        "rsa-public" | "rsa-private" => "RSAPublicKey",
        "ec-public" | "ec-private" => "EllipticCurvePublicKey",
        "dsa-public" | "dsa-private" => "DSAPublicKey",
        "ed25519-public" | "ed25519-private" => "Ed25519PublicKey",
        "ed448-public" | "ed448-private" => "Ed448PublicKey",
        _ => return None,
    })
}

/// Project a BIG-IP config-object cert into the `x509_parse` shape. `cert`
/// is an `ObjectRef` (or dict) carrying the BIG-IP cert metadata fields.
#[cfg(feature = "probes")]
pub(crate) fn x509_from_config(cert: &Value) -> Result<Value, QueryError> {
    if matches!(cert, Value::Null) {
        return Err(QueryError::builtin("x509_from_config: cannot project None"));
    }
    let get = |name: &str| field_get(cert, name);

    let subject = get("subject");
    let issuer = get("issuer");
    let san_raw = get("subject_alternative_name");
    let mut sans: Vec<Value> = Vec::new();
    if !san_raw.is_empty() {
        for chunk in san_raw.split(',') {
            let token = chunk.trim();
            if let Some((_, host)) = token.split_once(':') {
                sans.push(Value::Str(host.trim().to_owned()));
            } else if !token.is_empty() {
                sans.push(Value::Str(token.to_owned()));
            }
        }
    }

    let mut fingerprint = get("fingerprint");
    if fingerprint.to_uppercase().starts_with("SHA256/") {
        fingerprint = fingerprint["SHA256/".len()..].to_owned();
    }
    let fingerprint = fingerprint.replace(':', "").to_uppercase();

    let serial_raw = get("serial_number");
    let serial_hex = if serial_raw.is_empty() {
        String::new()
    } else if let Ok(n) = serial_raw.parse::<u128>() {
        format!("{n:X}")
    } else {
        serial_raw.to_uppercase()
    };

    let version_raw = get("version");
    let version = if !version_raw.is_empty() && !version_raw.to_lowercase().starts_with('v') {
        format!("v{version_raw}")
    } else {
        version_raw.clone()
    };

    let key_size_raw = {
        let a = get("key_size");
        if a.is_empty() {
            get("certificate_key_size")
        } else {
            a
        }
    };
    let key_size = if key_size_raw.is_empty() {
        Value::Null
    } else {
        key_size_raw.parse::<i64>().map_or(Value::Null, Value::Int)
    };

    let key_alg =
        key_type_to_key_alg(&get("key_type")).map_or(Value::Null, |s| Value::Str(s.to_owned()));

    let mut not_after = parse_x509_date(&get("expiration_string"));
    if not_after.is_none() {
        let epoch_raw = get("expiration_date");
        if !epoch_raw.is_empty()
            && let Ok(secs) = epoch_raw.parse::<i64>()
        {
            not_after = Some(iso_utc(secs));
        }
    }

    let mut m: IndexMap<String, Value> = IndexMap::new();
    m.insert("subject".to_owned(), Value::Str(subject));
    m.insert("issuer".to_owned(), Value::Str(issuer));
    m.insert("not_before".to_owned(), Value::Null);
    m.insert(
        "not_after".to_owned(),
        not_after.map_or(Value::Null, Value::Str),
    );
    m.insert("serial".to_owned(), Value::Str(serial_hex));
    m.insert("fingerprint_sha256".to_owned(), Value::Str(fingerprint));
    m.insert("sans".to_owned(), Value::List(sans));
    m.insert("key_alg".to_owned(), key_alg);
    m.insert("key_size".to_owned(), key_size);
    m.insert("sig_alg".to_owned(), Value::Null);
    m.insert("version".to_owned(), Value::Str(version));
    m.insert("public_key_pem".to_owned(), Value::Null);
    Ok(Value::Object(m))
}

/// Read a config field — accept attribute / dict-key / `ObjectRef`
/// field form, with both `_`-and-`-` spellings, stripping surrounding quotes.
#[cfg(feature = "probes")]
fn field_get(cert: &Value, name: &str) -> String {
    let tmsh_name = name.replace('_', "-");
    let lookup = |fields: &IndexMap<String, Value>| -> String {
        for key in [tmsh_name.as_str(), name] {
            if let Some(v) = fields.get(key)
                && !matches!(v, Value::Null)
            {
                return scalar_str(v);
            }
        }
        String::new()
    };
    let raw = match cert {
        Value::ObjectRef(o) => lookup(&o.fields),
        Value::Object(map) => lookup(map),
        _ => String::new(),
    };
    let bytes: Vec<char> = raw.chars().collect();
    if bytes.len() >= 2 && bytes[0] == '"' && bytes[bytes.len() - 1] == '"' {
        bytes[1..bytes.len() - 1].iter().collect()
    } else {
        raw
    }
}

#[cfg(feature = "probes")]
fn scalar_str(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::PathRef(p) => p.full_path.clone(),
        Value::Int(i) => i.to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
        Value::Float(f) => crate::jsonfmt::py_float_repr(*f),
        Value::Null => "None".to_owned(),
        other => crate::jsonfmt::to_compact(other),
    }
}

/// Parse a TMSH `expiration-string` → ISO-8601 UTC.
#[cfg(feature = "probes")]
fn parse_x509_date(text: &str) -> Option<String> {
    use chrono::{NaiveDateTime, TimeZone as _, Utc};
    if text.is_empty() {
        return None;
    }
    // `%b %d %H:%M:%S %Y %Z` and the double-space variant; `%Z` is the GMT
    // suffix TMSH emits — chrono can't parse a named zone, so strip a trailing
    // ` GMT` / ` UTC` and parse the naive form as UTC.
    let trimmed = text
        .trim()
        .trim_end_matches(" GMT")
        .trim_end_matches(" UTC")
        .trim();
    for fmt in [
        "%b %d %H:%M:%S %Y",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            let dt = Utc.from_utc_datetime(&naive);
            return Some(iso_utc(dt.timestamp()));
        }
    }
    // `%Y-%m-%dT%H:%M:%S%z` — an explicit offset; parse without stripping.
    if let Ok(dt) = chrono::DateTime::parse_from_str(text.trim(), "%Y-%m-%dT%H:%M:%S%z") {
        return Some(iso_utc(dt.timestamp()));
    }
    None
}

// cert_load — read a cert file from disk into the x509_parse shape

/// Read a cert file from disk into the x509 parse shape.
///
/// PEM (one or many `CERTIFICATE` blocks) and single DER certs are parsed
/// byte-identically to `x509_parse`. A single cert returns the dict; multiple
/// return a list in file order. PKCS#12 (`.pfx` / `.p12`) is unsupported and
/// returns a clear `BuiltinError` (PKCS#12 decoding is not supported).
#[cfg(feature = "probes")]
pub(crate) fn cert_load(path: &str, _password: Option<&str>) -> Result<Value, QueryError> {
    let expanded = expanduser(path);
    let raw = match std::fs::read(&expanded) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(QueryError::builtin(format!(
                "cert_load: file not found: {expanded}"
            )));
        }
        Err(e) => {
            return Err(QueryError::builtin(format!(
                "cert_load: cannot read {expanded}: {e}"
            )));
        }
    };
    // PEM is the easy path — split on the CERTIFICATE markers and parse each.
    if raw.windows(b"-----BEGIN".len()).any(|w| w == b"-----BEGIN") {
        let text = match std::str::from_utf8(&raw) {
            Ok(t) => t,
            Err(e) => {
                return Err(QueryError::builtin(format!(
                    "cert_load: {expanded}: not UTF-8 PEM ({e})"
                )));
            }
        };
        let blocks = split_pem_blocks(text);
        if blocks.is_empty() {
            return Err(QueryError::builtin(format!(
                "cert_load: {expanded}: no PEM CERTIFICATE blocks found"
            )));
        }
        let mut parsed = Vec::with_capacity(blocks.len());
        for b in &blocks {
            parsed.push(x509_parse(b)?);
        }
        return Ok(if parsed.len() == 1 {
            parsed.into_iter().next().expect("len == 1")
        } else {
            Value::List(parsed)
        });
    }
    // PKCS#12 detection by extension — PKCS#12 decoding is not supported.
    let suffix = expanded
        .rsplit_once('.')
        .map_or(String::new(), |(_, s)| s.to_lowercase());
    if suffix == "pfx" || suffix == "p12" {
        return Err(QueryError::builtin(format!(
            "cert_load: {expanded}: PKCS#12 parsing needs the \
             ``cryptography`` package — install it to load .pfx / .p12 files"
        )));
    }
    // Assume a single DER cert.
    match x509_parse_der(&raw) {
        Ok(v) => Ok(v),
        Err(_) => Err(QueryError::builtin(format!(
            "cert_load: {expanded}: not PEM / DER / PKCS#12"
        ))),
    }
}

/// Tilde expansion of a leading `~` / `~/` in the path.
#[cfg(feature = "probes")]
fn expanduser(path: &str) -> String {
    if (path == "~" || path.starts_with("~/"))
        && let Some(home) = std::env::var_os("HOME")
    {
        let home = home.to_string_lossy();
        return if path == "~" {
            home.into_owned()
        } else {
            format!("{home}{}", &path[1..])
        };
    }
    path.to_owned()
}

/// Split into one PEM string per `CERTIFICATE` block, in file order
/// (key / other block types skipped).
#[cfg(feature = "probes")]
fn split_pem_blocks(text: &str) -> Vec<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut in_cert = false;
    let mut buf = String::new();
    for line in text.split_inclusive('\n') {
        let stripped = line.trim();
        if stripped.starts_with("-----BEGIN CERTIFICATE-----") {
            in_cert = true;
            buf.clear();
            buf.push_str(line);
            continue;
        }
        if in_cert {
            buf.push_str(line);
            if stripped.starts_with("-----END CERTIFICATE-----") {
                blocks.push(std::mem::take(&mut buf));
                in_cert = false;
            }
        }
    }
    blocks
}

// Live network probes (faithful-but-not-golden)

/// Build a result dict `{ok, rtt_ms, error}`.
#[cfg(feature = "probes")]
fn ok_rtt_error(ok: bool, rtt_ms: Option<f64>, error: Option<String>) -> Value {
    let mut m: IndexMap<String, Value> = IndexMap::new();
    m.insert("ok".to_owned(), Value::Bool(ok));
    m.insert(
        "rtt_ms".to_owned(),
        rtt_ms.map_or(Value::Null, Value::Float),
    );
    m.insert("error".to_owned(), error.map_or(Value::Null, Value::Str));
    Value::Object(m)
}

/// Forward DNS via the system resolver. Returns the
/// sorted, unique list of resolved addresses (empty on failure).
#[cfg(feature = "probes")]
pub(crate) fn dns(name: &str) -> Vec<Value> {
    // Mirrors `getaddrinfo(name, None, type=SOCK_STREAM)`. `(host, 0)` with
    // `to_socket_addrs` resolves both A and AAAA.
    let mut addrs: Vec<String> = match (name, 0u16).to_socket_addrs() {
        Ok(it) => it.map(|sa| sa.ip().to_string()).collect(),
        Err(_) => Vec::new(),
    };
    addrs.sort();
    addrs.dedup();
    addrs.into_iter().map(Value::Str).collect()
}

/// Reverse DNS (`gethostbyaddr`-style). Returns the
/// canonical name (empty on failure). `getnameinfo` yields one name, so the
/// alias list a fuller resolver collects is not reproduced (best-effort).
#[cfg(feature = "probes")]
pub(crate) fn rev_dns(ip: &str) -> Vec<Value> {
    let Ok(addr) = ip.parse::<std::net::IpAddr>() else {
        return Vec::new();
    };
    match dns_lookup::lookup_addr(&addr) {
        Ok(host) => vec![Value::Str(host)],
        Err(_) => Vec::new(),
    }
}

/// ICMP echo via the system `ping` command.
#[cfg(feature = "probes")]
pub(crate) fn ping(ip: &str) -> Value {
    let timeout_s = 2u64;
    let output = std::process::Command::new("ping")
        .args(["-c", "1", "-W", &timeout_s.to_string(), ip])
        .output();
    match output {
        Err(e) => ok_rtt_error(false, None, Some(e.to_string())),
        Ok(out) if !out.status.success() => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let msg = if !stderr.trim().is_empty() {
                stderr.trim().to_owned()
            } else if !stdout.trim().is_empty() {
                stdout.trim().to_owned()
            } else {
                "ping failed".to_owned()
            };
            ok_rtt_error(false, None, Some(msg).filter(|s| !s.is_empty()))
        }
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            ok_rtt_error(true, parse_ping_rtt(&stdout), None)
        }
    }
}

/// Pull the `time=NN ms` value out of `ping`'s first reply line.
#[cfg(feature = "probes")]
fn parse_ping_rtt(stdout: &str) -> Option<f64> {
    for line in stdout.lines() {
        if let Some((_, rest)) = line.split_once("time=") {
            let bit = rest.split_whitespace().next()?;
            return bit.trim_end_matches("ms").parse::<f64>().ok();
        }
    }
    None
}

/// TCP connect / UDP send timing.
#[cfg(feature = "probes")]
pub(crate) fn portping(ip: &str, port: i64, protocol: &str) -> Result<Value, QueryError> {
    let proto = protocol.to_lowercase();
    if proto != "tcp" && proto != "udp" {
        return Err(QueryError::builtin(format!(
            "portping: protocol must be 'tcp' or 'udp', got {}",
            crate::lexer::py_repr_str(protocol)
        )));
    }
    let timeout = Duration::from_secs(2);
    let target = format!("{ip}:{port}");
    let Ok(mut addrs) = target.to_socket_addrs() else {
        return Ok(ok_rtt_error(
            false,
            None,
            Some(format!("cannot resolve {target}")),
        ));
    };
    let Some(sa) = addrs.next() else {
        return Ok(ok_rtt_error(
            false,
            None,
            Some(format!("cannot resolve {target}")),
        ));
    };
    let start = Instant::now();
    if proto == "tcp" {
        match TcpStream::connect_timeout(&sa, timeout) {
            Ok(_) => Ok(ok_rtt_error(
                true,
                Some(start.elapsed().as_secs_f64() * 1000.0),
                None,
            )),
            Err(e) => Ok(ok_rtt_error(false, None, Some(e.to_string()))),
        }
    } else {
        let bind = if sa.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
        match UdpSocket::bind(bind).and_then(|s| s.send_to(b"", sa).map(|_| ())) {
            Ok(()) => Ok(ok_rtt_error(
                true,
                Some(start.elapsed().as_secs_f64() * 1000.0),
                None,
            )),
            Err(e) => Ok(ok_rtt_error(false, None, Some(e.to_string()))),
        }
    }
}

/// Subprocess `traceroute` invocation.
#[cfg(feature = "probes")]
pub(crate) fn traceroute(ip: &str) -> Value {
    let max_hops = 30u32;
    let timeout_s = 2u32;
    let output = std::process::Command::new("traceroute")
        .args([
            "-n",
            "-m",
            &max_hops.to_string(),
            "-w",
            &timeout_s.to_string(),
            ip,
        ])
        .output();
    let stdout = match output {
        Err(e) => {
            let mut m: IndexMap<String, Value> = IndexMap::new();
            m.insert("hop".to_owned(), Value::Int(0));
            m.insert("ip".to_owned(), Value::Null);
            m.insert("rtt_ms".to_owned(), Value::Null);
            m.insert("error".to_owned(), Value::Str(e.to_string()));
            return Value::List(vec![Value::Object(m)]);
        }
        Ok(out) => String::from_utf8_lossy(&out.stdout).into_owned(),
    };
    let mut hops: Vec<Value> = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || !line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let Ok(hop) = parts[0].parse::<i64>() else {
            continue;
        };
        let mut hop_ip: Option<String> = None;
        let mut rtt: Option<f64> = None;
        for token in &parts[1..] {
            if token.contains('.') || token.contains(':') {
                if hop_ip.is_none() && token.chars().any(|c| c.is_ascii_digit()) && *token != "ms" {
                    // A token that is purely a decimal number is an RTT, not an IP.
                    if token
                        .replacen('.', "", 1)
                        .chars()
                        .all(|c| c.is_ascii_digit())
                    {
                        rtt = token.parse::<f64>().ok();
                    } else {
                        hop_ip = Some((*token).to_owned());
                    }
                }
            } else if token
                .replacen('.', "", 1)
                .chars()
                .all(|c| c.is_ascii_digit())
                && !token.is_empty()
            {
                rtt = token.parse::<f64>().ok();
            }
        }
        let mut m: IndexMap<String, Value> = IndexMap::new();
        m.insert("hop".to_owned(), Value::Int(hop));
        m.insert("ip".to_owned(), hop_ip.map_or(Value::Null, Value::Str));
        m.insert("rtt_ms".to_owned(), rtt.map_or(Value::Null, Value::Float));
        hops.push(Value::Object(m));
    }
    Value::List(hops)
}

/// TCP connect, optional send, read a banner.
#[cfg(feature = "probes")]
pub(crate) fn socket_get(host: &str, port: i64, send: &str) -> Result<Value, QueryError> {
    let recv_max = 4096usize;
    let timeout = Duration::from_secs(5);
    let target = format!("{host}:{port}");
    let connect = || -> std::io::Result<Vec<u8>> {
        let sa = target
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| std::io::Error::other("name resolution failed"))?;
        let mut stream = TcpStream::connect_timeout(&sa, timeout)?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        if !send.is_empty() {
            stream.write_all(send.as_bytes())?;
        }
        let mut buf = vec![0u8; recv_max];
        let n = stream.read(&mut buf)?;
        buf.truncate(n);
        Ok(buf)
    };
    match connect() {
        Ok(data) => Ok(Value::Str(String::from_utf8_lossy(&data).into_owned())),
        Err(e) => Err(QueryError::builtin(format!("socket_get: {e}"))),
    }
}

/// TLS connect and report peer metadata.
///
/// Faithful-but-not-golden: produces the same dict shape the live path emits
/// (`protocol` / `cipher` / `peer_cert` / `alpn_selected` / `verify_status` /
/// `reason` / `error`). Verification failures don't abort — the peer cert is
/// captured from the (non-verifying) handshake so audit queries still get it.
#[cfg(feature = "probes")]
pub(crate) fn tls_handshake(
    host: &str,
    port: i64,
    sni: Option<&str>,
    ca_bundle: Option<&str>,
) -> Value {
    let sni_value = sni.unwrap_or(host);
    match tls::handshake(host, port, sni_value, ca_bundle) {
        Ok(v) => v,
        Err(e) => {
            let mut reason: IndexMap<String, Value> = IndexMap::new();
            reason.insert("kind".to_owned(), Value::Str("connection_error".to_owned()));
            reason.insert("message".to_owned(), Value::Str(e.clone()));
            reason.insert("fatal".to_owned(), Value::Bool(true));
            let mut m: IndexMap<String, Value> = IndexMap::new();
            m.insert("protocol".to_owned(), Value::Null);
            m.insert("cipher".to_owned(), Value::Null);
            m.insert("peer_cert".to_owned(), Value::Null);
            m.insert("alpn_selected".to_owned(), Value::Null);
            m.insert("verify_status".to_owned(), Value::Str("error".to_owned()));
            m.insert("error".to_owned(), Value::Str(e));
            m.insert("reason".to_owned(), Value::Object(reason));
            Value::Object(m)
        }
    }
}

/// Generic HTTP via `ureq`.
///
/// Faithful-but-not-golden: `{status, headers, body, body_json, peer_cert,
/// reason, error}`. The `peer_cert` capture from the live handshake is not
/// reproduced (ureq doesn't expose it) — it is always `null`, documented.
#[cfg(feature = "probes")]
pub(crate) fn url_request(
    method: &str,
    url: &str,
    body: Option<&str>,
    headers: &[(String, String)],
    ca_bundle: Option<&str>,
) -> Value {
    let scheme = url.split_once("://").map_or("", |(s, _)| s).to_lowercase();
    if scheme != "http" && scheme != "https" {
        let mut m: IndexMap<String, Value> = IndexMap::new();
        m.insert("status".to_owned(), Value::Null);
        m.insert("headers".to_owned(), Value::Object(IndexMap::new()));
        m.insert("body".to_owned(), Value::Str(String::new()));
        m.insert("body_json".to_owned(), Value::Null);
        m.insert(
            "error".to_owned(),
            Value::Str(format!(
                "unsupported URL scheme: {}",
                if scheme.is_empty() { "<none>" } else { &scheme }
            )),
        );
        return Value::Object(m);
    }
    http::request(method, url, body, headers, ca_bundle)
}

#[cfg(feature = "probes")]
mod http;
#[cfg(feature = "probes")]
mod tls;

// Registry wiring

#[cfg(feature = "probes")]
use crate::builtins::{BuiltinSpec, as_int, as_str, ctx, plain};
#[cfg(feature = "probes")]
use crate::eval::EvalContext;

/// Optional positional-string arg with a default, matching the
/// `protocol`=`tcp` / `send`=empty / `sni`=none parameter defaults.
#[cfg(feature = "probes")]
fn opt_str<'a>(
    args: &'a [Value],
    idx: usize,
    name: &str,
    default: &'a str,
) -> Result<std::borrow::Cow<'a, str>, QueryError> {
    match args.get(idx) {
        None => Ok(std::borrow::Cow::Borrowed(default)),
        Some(v) => Ok(std::borrow::Cow::Owned(as_str(v, name, idx + 1)?)),
    }
}

// `dns` / `rev_dns` are NOT gated (they have no probe-gate
// call) — a forward / reverse name lookup is treated as benign. They stay
// `Plain` to match: a bare `dns("host")` resolves without `--enable-probes`.
#[cfg(feature = "probes")]
fn bi_dns(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::List(dns(&as_str(&args[0], "dns", 1)?)))
}

#[cfg(feature = "probes")]
fn bi_rev_dns(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::List(rev_dns(&as_str(&args[0], "rev_dns", 1)?)))
}

#[cfg(feature = "probes")]
fn bi_ping(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    require_probes("ping", ctx.probes_enabled)?;
    Ok(ping(&as_str(&args[0], "ping", 1)?))
}

#[cfg(feature = "probes")]
fn bi_portping(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    require_probes("portping", ctx.probes_enabled)?;
    let ip = as_str(&args[0], "portping", 1)?;
    let port = as_int(&args[1], "portping", 2)?;
    let proto = opt_str(args, 2, "portping", "tcp")?;
    portping(&ip, port, &proto)
}

#[cfg(feature = "probes")]
fn bi_traceroute(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    require_probes("traceroute", ctx.probes_enabled)?;
    Ok(traceroute(&as_str(&args[0], "traceroute", 1)?))
}

#[cfg(feature = "probes")]
fn bi_socket_get(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    require_probes("socket_get", ctx.probes_enabled)?;
    let host = as_str(&args[0], "socket_get", 1)?;
    let port = as_int(&args[1], "socket_get", 2)?;
    let send = opt_str(args, 2, "socket_get", "")?;
    socket_get(&host, port, &send)
}

#[cfg(feature = "probes")]
fn bi_tls_handshake(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    require_probes("tls_handshake", ctx.probes_enabled)?;
    let host = as_str(&args[0], "tls_handshake", 1)?;
    let port = as_int(&args[1], "tls_handshake", 2)?;
    let sni = match args.get(2) {
        None => None,
        Some(v) => Some(as_str(v, "tls_handshake", 3)?),
    };
    let ca = ctx.ca_bundle.clone();
    Ok(tls_handshake(&host, port, sni.as_deref(), ca.as_deref()))
}

/// One `url_<method>` builtin.
#[cfg(feature = "probes")]
fn url_method(
    method: &'static str,
    args: &[Value],
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let name = match method {
        "GET" => "url_get",
        "HEAD" => "url_head",
        "OPTIONS" => "url_options",
        "POST" => "url_post",
        _ => "url",
    };
    require_probes(name, ctx.probes_enabled)?;
    let url = as_str(&args[0], name, 1)?;
    let mut body: Option<String> = None;
    let mut headers: Vec<(String, String)> = Vec::new();
    if method == "POST" {
        if let Some(v) = args.get(1) {
            body = Some(as_str(v, "url_post", 2)?);
        }
        if let Some(Value::Object(m)) = args.get(2) {
            headers = headers_from_object(m);
        }
    } else if let Some(Value::Object(m)) = args.get(1) {
        headers = headers_from_object(m);
    }
    let ca = ctx.ca_bundle.clone();
    Ok(url_request(
        method,
        &url,
        body.as_deref(),
        &headers,
        ca.as_deref(),
    ))
}

/// Coerce an object of header values to `(name, value)` string pairs,
/// stringifying both key and value.
#[cfg(feature = "probes")]
fn headers_from_object(m: &IndexMap<String, Value>) -> Vec<(String, String)> {
    m.iter().map(|(k, v)| (k.clone(), scalar_str(v))).collect()
}

#[cfg(feature = "probes")]
fn bi_url_get(a: &[Value], c: &mut EvalContext) -> Result<Value, QueryError> {
    url_method("GET", a, c)
}
#[cfg(feature = "probes")]
fn bi_url_head(a: &[Value], c: &mut EvalContext) -> Result<Value, QueryError> {
    url_method("HEAD", a, c)
}
#[cfg(feature = "probes")]
fn bi_url_options(a: &[Value], c: &mut EvalContext) -> Result<Value, QueryError> {
    url_method("OPTIONS", a, c)
}
#[cfg(feature = "probes")]
fn bi_url_post(a: &[Value], c: &mut EvalContext) -> Result<Value, QueryError> {
    url_method("POST", a, c)
}

// --- ungated pure x509 surface (plain; not probe-gated) ---

#[cfg(feature = "probes")]
fn bi_x509_parse(args: &[Value]) -> Result<Value, QueryError> {
    x509_parse(&as_str(&args[0], "x509_parse", 1)?)
}

#[cfg(feature = "probes")]
fn bi_x509_from_config(args: &[Value]) -> Result<Value, QueryError> {
    x509_from_config(&args[0])
}

#[cfg(feature = "probes")]
fn bi_x509_eq(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Bool(x509_eq(&args[0], &args[1])))
}

#[cfg(feature = "probes")]
fn bi_cert_load(args: &[Value]) -> Result<Value, QueryError> {
    let path = as_str(&args[0], "cert_load", 1)?;
    let password = match args.get(1) {
        None => None,
        Some(v) => Some(as_str(v, "cert_load", 2)?),
    };
    cert_load(&path, password.as_deref())
}

#[cfg(feature = "probes")]
fn bi_ucs_cert(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let Value::ObjectRef(obj) = &args[0] else {
        return Err(QueryError::builtin(
            "ucs_cert: expects a sys file ssl-cert object (pipe through \
             .sys[\"file-ssl-cert\"][ref] first)",
        ));
    };
    let full_path = obj.full_path.clone();
    let label = if full_path.is_empty() {
        "object".to_owned()
    } else {
        full_path.clone()
    };
    let config_uri = obj.config_uri.clone();
    if config_uri.is_empty() {
        return Err(QueryError::builtin(format!(
            "ucs_cert: {label} has no source — it was not loaded from a UCS / file"
        )));
    }
    let mut cache_path = String::new();
    for key in ["cache-path", "source-path"] {
        if let Some(raw) = obj.fields.get(key).filter(|v| !matches!(v, Value::Null)) {
            let cp0 = scalar_str(raw).trim().trim_matches('"').to_owned();
            let cp = if key == "source-path" {
                cp0.strip_prefix("file://").unwrap_or(&cp0).to_owned()
            } else {
                cp0
            };
            if !cp.is_empty() {
                cache_path = cp;
                break;
            }
        }
    }
    let cert_label = if full_path.is_empty() {
        "cert".to_owned()
    } else {
        full_path
    };
    if cache_path.is_empty() {
        return Err(QueryError::builtin(format!(
            "ucs_cert: {cert_label} has no cache-path/source-path; \
             cannot locate the cert file in the archive"
        )));
    }
    match &ctx.ucs_cert_reader {
        Some(reader) => reader(&config_uri, &cache_path),
        None => Err(QueryError::builtin(
            "ucs_cert: no UCS reader is wired in this context (run it through the f5 CLI)",
        )),
    }
}

/// Probe builtin registrations — folded into the global registry by
/// [`crate::builtins`].
#[cfg(feature = "probes")]
pub(crate) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        // DNS lookups are ungated (benign name resolution).
        plain("dns", "net", 1, Some(1), false, bi_dns),
        plain("rev_dns", "net", 1, Some(1), false, bi_rev_dns),
        // Network probes — gated on `--enable-probes`; `Ctx` so they can read
        // `probes_enabled` / `ca_bundle` from the evaluator context.
        ctx("ping", "net", 1, Some(1), bi_ping),
        ctx("portping", "net", 2, Some(3), bi_portping),
        ctx("traceroute", "net", 1, Some(1), bi_traceroute),
        ctx("socket_get", "net", 2, Some(3), bi_socket_get),
        ctx("tls_handshake", "net", 2, Some(3), bi_tls_handshake),
        ctx("url_get", "net", 1, Some(2), bi_url_get),
        ctx("url_head", "net", 1, Some(2), bi_url_head),
        ctx("url_options", "net", 1, Some(2), bi_url_options),
        ctx("url_post", "net", 1, Some(3), bi_url_post),
        ctx("ucs_cert", "net", 1, Some(1), bi_ucs_cert),
        // Pure x509 surface — ungated (plain, deterministic here).
        plain("x509_parse", "net", 1, Some(1), false, bi_x509_parse),
        plain(
            "x509_from_config",
            "net",
            1,
            Some(1),
            false,
            bi_x509_from_config,
        ),
        plain("x509_eq", "net", 2, Some(2), false, bi_x509_eq),
        plain("cert_load", "value", 1, Some(2), false, bi_cert_load),
    ]
}
