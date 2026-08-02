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

//! Profile traffic-order builtin.
//!
//! A BIG-IP virtual server processes its attached profiles bottom-up — the
//! transport profile (TCP/UDP/FASTL4) nearest the wire, then TLS, then the
//! application profile (HTTP/DNS/…), with the security / acceleration facets on
//! top. A config lists profiles in an arbitrary order (often alphabetical, so
//! `http` precedes `tcp`); the `profile_order` builtin re-orders them into that
//! processing ("traffic") order so a listener reads TCP → … → HTTP.
//!
//! Each profile's `layer` comes from the shared profile registry
//! ([`tcl_registry::profiles`]), so this is the single source of truth for
//! profile ordering across `f5 query`, the BIG-IP report and the LSP.

use std::collections::HashMap;
use std::sync::OnceLock;

use indexmap::IndexMap;
use tcl_registry::profile_defaults::{BigipVersion, profile_field_default, profile_field_defaults};
use tcl_registry::profiles::ProfileRegistry;

use crate::builtins::{BuiltinSpec, as_str, plain};
use crate::errors::QueryError;
use crate::value::Value;

fn registry() -> &'static ProfileRegistry {
    static R: OnceLock<ProfileRegistry> = OnceLock::new();
    R.get_or_init(ProfileRegistry::build)
}

/// Canonical traffic-order rank of a profile *type* — lowest is nearest the
/// wire. Accepts the type spellings the query engine and model use
/// (`"HTTP"`, `"CLIENT_SSL"`, `"ProfileType.CLIENT_SSL"`); the engine keys some
/// multi-word types with underscores where the registry keys them without
/// (`CLIENT_SSL` vs `CLIENTSSL`), so it retries the underscore-stripped form.
/// The layer itself always comes from [`ProfileRegistry`].
#[must_use]
pub fn profile_traffic_rank(profile_type: &str) -> u8 {
    let t = profile_type.rsplit('.').next().unwrap_or(profile_type);
    let reg = registry();
    if reg.get_profile(t).is_some() {
        reg.layer_rank(t)
    } else {
        reg.layer_rank(&t.replace('_', ""))
    }
}

/// The registry profile *type* of a TMOS built-in default profile, inferred
/// from its well-known name (`/Common/tcp` → `"TCP"`). Used only as a fallback
/// for base profiles a config never re-declares (so they are absent from the
/// `.ltm.profile` inventory and have no authoritative type). The resulting
/// type is still ranked by the registry `layer`.
#[must_use]
pub fn default_profile_type(profile_ref: &str) -> Option<&'static str> {
    let name = profile_ref
        .rsplit('/')
        .next()
        .unwrap_or(profile_ref)
        .to_ascii_lowercase();
    if name.contains("clientssl") || name.contains("client-ssl") {
        Some("CLIENTSSL")
    } else if name.contains("serverssl") || name.contains("server-ssl") {
        Some("SERVERSSL")
    } else if name == "http" || name == "http2" {
        Some("HTTP")
    } else if name == "tcp" {
        Some("TCP")
    } else if name == "udp" {
        Some("UDP")
    } else if name == "fastl4" {
        Some("FASTL4")
    } else if name == "sctp" {
        Some("SCTP")
    } else if name == "oneconnect" {
        Some("ONECONNECT")
    } else if name.contains("fasthttp") {
        Some("FASTHTTP")
    } else if name.contains("dns") {
        Some("DNS")
    } else {
        None
    }
}

/// Order profile references into traffic order. `resolve_type` maps a reference
/// to its authoritative profile-type name (e.g. from a config's profile
/// inventory); references it can't type fall back to well-known
/// default-profile-name inference ([`default_profile_type`]), then rank last —
/// keeping their original relative order (the sort is stable on input index).
pub fn order_profiles_by_traffic<F>(refs: &[String], resolve_type: F) -> Vec<String>
where
    F: Fn(&str) -> Option<String>,
{
    let mut items: Vec<(usize, u8, String)> = refs
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let rank = resolve_type(r)
                .or_else(|| default_profile_type(r).map(str::to_owned))
                .map_or(u8::MAX, |t| profile_traffic_rank(&t));
            (i, rank, r.clone())
        })
        .collect();
    items.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    items.into_iter().map(|(_, _, r)| r).collect()
}

/// Order profile references into traffic order given a `full_path → type` map
/// (e.g. a config's typed profile inventory).
///
/// Each reference is matched against `types` by exact key first, then by leaf
/// name (last `/`-separated segment) — so a virtual's partition-relative
/// reference like `my_http` resolves against the inventory key
/// `/Common/my_http`. References that resolve to neither fall back to
/// well-known default-profile-name inference inside
/// [`order_profiles_by_traffic`]. Leaf collisions across partitions keep the
/// first-seen type (same-partition attachment is the norm).
#[must_use]
pub fn order_profiles_with_types<S: std::hash::BuildHasher>(
    refs: &[String],
    types: &HashMap<String, String, S>,
) -> Vec<String> {
    let mut by_leaf: HashMap<&str, &str> = HashMap::new();
    for (path, ty) in types {
        let leaf = path.rsplit('/').next().unwrap_or(path);
        by_leaf.entry(leaf).or_insert(ty.as_str());
    }
    order_profiles_by_traffic(refs, |r| {
        types.get(r).cloned().or_else(|| {
            by_leaf
                .get(r.rsplit('/').next().unwrap_or(r))
                .map(|t| (*t).to_owned())
        })
    })
}

pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        plain("profile_order", "value", 1, Some(1), true, bi_profile_order),
        plain(
            "profile_defaults",
            "bigip",
            1,
            Some(2),
            false,
            bi_profile_defaults,
        ),
        plain(
            "profile_default",
            "bigip",
            2,
            Some(3),
            false,
            bi_profile_default,
        ),
    ]
}

/// Parse an optional trailing version argument into a [`BigipVersion`].
///
/// `null` / empty string / absent → `None` (resolve the current default);
/// a malformed version string is an error rather than a silent "current".
fn version_arg(
    args: &[Value],
    idx: usize,
    name: &'static str,
) -> Result<Option<BigipVersion>, QueryError> {
    match args.get(idx) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => {
            let s = as_str(v, name, idx + 1)?;
            if s.trim().is_empty() {
                return Ok(None);
            }
            BigipVersion::parse(&s)
                .map(Some)
                .ok_or_else(|| QueryError::builtin(format!("{name}: invalid BIG-IP version '{s}'")))
        }
    }
}

/// `profile_defaults(type ?version?)` — the TMOS default field values of an
/// unmodified `ltm profile <type>`, as an object of `field → default`.
///
/// SCF/`one-line` exports omit fields left at their default (they live in
/// `/config/profile_base.conf`); this recovers them. The optional second
/// argument is a BIG-IP version (`"15.1"`, `"16.1.3.2"`); omitted resolves the
/// current default. Unknown types yield an empty object.
fn bi_profile_defaults(args: &[Value]) -> Result<Value, QueryError> {
    let ty = as_str(&args[0], "profile_defaults", 1)?;
    let version = version_arg(args, 1, "profile_defaults")?;
    let mut m: IndexMap<String, Value> = IndexMap::new();
    for (field, value) in profile_field_defaults(&ty, version) {
        m.insert(field.to_owned(), Value::Str(value.to_owned()));
    }
    Ok(Value::Object(m))
}

/// `profile_default(type field ?version?)` — the TMOS default value of one
/// field of `ltm profile <type>`, or `null` when the type/field is unknown.
/// The optional version selects the release (see [`bi_profile_defaults`]).
fn bi_profile_default(args: &[Value]) -> Result<Value, QueryError> {
    let ty = as_str(&args[0], "profile_default", 1)?;
    let field = as_str(&args[1], "profile_default", 2)?;
    let version = version_arg(args, 2, "profile_default")?;
    Ok(profile_field_default(&ty, &field, version)
        .map_or(Value::Null, |v| Value::Str(v.to_owned())))
}

/// `profile_order` — sort a list of profile references into traffic order.
///
/// Operates on a stream of profile names/paths (e.g. a virtual's `.profiles`).
/// Types are inferred from well-known profile names; callers that already know
/// each profile's type (like the report, which has the typed profile
/// inventory) use [`order_profiles_by_traffic`] directly for exact ordering.
fn bi_profile_order(args: &[Value]) -> Result<Value, QueryError> {
    let items = crate::builtins::as_sequence(&args[0], "profile_order", 1)?;
    let refs: Vec<String> = items
        .iter()
        .map(|v| as_str(v, "profile_order", 1).unwrap_or_default())
        .collect();
    let ordered = order_profiles_by_traffic(&refs, |_| None);
    Ok(Value::List(ordered.into_iter().map(Value::Str).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_transport_before_application() {
        assert!(profile_traffic_rank("TCP") < profile_traffic_rank("HTTP"));
        // Engine underscore spelling still resolves through the registry.
        assert!(profile_traffic_rank("CLIENT_SSL") < profile_traffic_rank("HTTP"));
        assert!(profile_traffic_rank("TCP") < profile_traffic_rank("CLIENT_SSL"));
        assert!(profile_traffic_rank("ProfileType.SERVER_SSL") < profile_traffic_rank("HTTP"));
    }

    #[test]
    fn orders_default_profiles_by_name() {
        let refs = vec!["/Common/http".to_owned(), "/Common/tcp".to_owned()];
        let out = order_profiles_by_traffic(&refs, |_| None);
        assert_eq!(
            out,
            vec!["/Common/tcp".to_owned(), "/Common/http".to_owned()]
        );
    }

    #[test]
    fn relative_refs_resolve_by_leaf_against_full_path_inventory() {
        // Virtual attaches partition-relative names; inventory is full-path
        // keyed. Leaf matching still types them, so a custom HTTP profile is
        // ordered after the transport profile.
        let mut types = HashMap::new();
        types.insert("/Common/my_http".to_owned(), "HTTP".to_owned());
        types.insert("/Common/my_tcp".to_owned(), "TCP".to_owned());
        let refs = vec!["my_http".to_owned(), "my_tcp".to_owned()];
        let out = order_profiles_with_types(&refs, &types);
        assert_eq!(out, vec!["my_tcp".to_owned(), "my_http".to_owned()]);
    }

    #[test]
    fn resolver_types_win_over_name_inference() {
        // A custom-named HTTP profile is ordered after TCP via the resolver.
        let refs = vec!["/Common/weird_name".to_owned(), "/Common/tcp".to_owned()];
        let out = order_profiles_by_traffic(&refs, |r| {
            (r == "/Common/weird_name").then(|| "HTTP".to_owned())
        });
        assert_eq!(
            out,
            vec!["/Common/tcp".to_owned(), "/Common/weird_name".to_owned()]
        );
    }

    fn as_string(v: &Value) -> Option<&str> {
        match v {
            Value::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    #[test]
    fn profile_defaults_builtin_returns_field_object() {
        let out = bi_profile_defaults(&[Value::Str("tcp".to_owned())]).unwrap();
        let Value::Object(m) = out else {
            panic!("expected object");
        };
        assert_eq!(m.get("idle-timeout").and_then(as_string), Some("300"));
        assert_eq!(m.get("nagle").and_then(as_string), Some("disabled"));
    }

    #[test]
    fn profile_default_builtin_resolves_by_version() {
        let call = |args: &[Value]| -> Value { bi_profile_default(args).unwrap() };
        let clientssl_options = |ver: Option<&str>| {
            let mut args = vec![
                Value::Str("clientssl".to_owned()),
                Value::Str("options".to_owned()),
            ];
            if let Some(v) = ver {
                args.push(Value::Str(v.to_owned()));
            }
            call(&args)
        };

        let before_21_1 = "dont-insert-empty-fragments no-tlsv1.3 no-dtlsv1.2";
        let current = "dont-insert-empty-fragments no-tlsv1.1 no-tlsv1 no-ssl";
        // Before 14.0 → the original single-flag value (the override band covers it).
        assert_eq!(
            as_string(&clientssl_options(Some("13.1"))),
            Some("dont-insert-empty-fragments")
        );
        // [14.0, 17.1) → no-tlsv1.3 added, not yet no-dtlsv1.2.
        assert_eq!(
            as_string(&clientssl_options(Some("15.1"))),
            Some("dont-insert-empty-fragments no-tlsv1.3")
        );
        // [17.1, 21.1) adds no-dtlsv1.2.
        assert_eq!(
            as_string(&clientssl_options(Some("17.5"))),
            Some(before_21_1)
        );
        // 21.1 adopts the secure TLS defaults extracted from profile_base.conf.
        assert_eq!(as_string(&clientssl_options(Some("21.1"))), Some(current));
        // No version → current (newest) band.
        assert_eq!(as_string(&clientssl_options(None)), Some(current));
    }

    #[test]
    fn profile_default_builtin_unknown_is_null_and_bad_version_errors() {
        let unknown = bi_profile_default(&[
            Value::Str("tcp".to_owned()),
            Value::Str("no-such-field".to_owned()),
        ])
        .unwrap();
        assert!(matches!(unknown, Value::Null));

        let bad = bi_profile_default(&[
            Value::Str("tcp".to_owned()),
            Value::Str("idle-timeout".to_owned()),
            Value::Str("not-a-version".to_owned()),
        ]);
        assert!(bad.is_err());
    }
}
