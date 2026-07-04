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

use std::sync::OnceLock;

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

pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![plain(
        "profile_order",
        "value",
        1,
        Some(1),
        true,
        bi_profile_order,
    )]
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
        assert_eq!(out, vec!["/Common/tcp".to_owned(), "/Common/http".to_owned()]);
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
}
