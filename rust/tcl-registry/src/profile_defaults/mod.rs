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

//! Version-ranged TMOS profile field defaults.
//!
//! A single-configuration file (SCF) — and `tmsh list … one-line` — omit any
//! profile field that is still at its TMOS default, because those values are
//! part of the read-only base config a BIG-IP loads before `bigip.conf`.
//! Reconstructing a profile's *effective* configuration from an SCF therefore
//! requires that default set. Because defaults drift across releases (a field
//! is added, a default is retuned, an option flips), each default is keyed by
//! the BIG-IP **version** it was captured at.
//!
//! This module is that table. It is the registry-side source of truth for
//! "what does field X of an unmodified `ltm profile <type>` resolve to on TMOS
//! version V", consumed by `f5 query` (the `profile_defaults` builtin), the
//! BIG-IP report (to show effective values for base profiles an SCF never
//! re-declares), and the LSP.
//!
//! **These are fallbacks only.** A value present in a parsed source
//! (UCS/qkview/SCF/`bigip.conf`) always wins: consult this table solely for a
//! field the parsed config does not itself carry.
//!
//! ## Version resolution
//!
//! Each entry carries the snapshot version it was captured at (as a half-open
//! range starting there). Resolving field X for a report at version V
//! floor-matches: the snapshot whose range covers V, else — when V predates
//! every snapshot we hold — the oldest snapshot we have. With a single snapshot
//! that means every report resolves to it; as more snapshots are added, a report
//! picks the nearest not-newer one. Refresh or extend by re-running the
//! generator against another base-config snapshot.

mod generated;

/// The full profile default table (generated). See the
/// module docs for provenance and the version-range policy.
pub use generated::PROFILE_DEFAULTS_GENERATED as PROFILE_DEFAULTS;

/// A parsed BIG-IP / TMOS version, ordered for range comparison.
///
/// TMOS versions are `major.minor.maintenance.point` (e.g. `16.1.3.2`); a
/// missing trailing component is `0`, so `15.1` compares equal to `15.1.0.0`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct BigipVersion {
    /// Major (e.g. `16`).
    pub major: u16,
    /// Minor (e.g. `1`).
    pub minor: u16,
    /// Maintenance / patch (e.g. `3`).
    pub maintenance: u16,
    /// Point / hotfix ordinal (e.g. `2`).
    pub point: u16,
}

impl BigipVersion {
    /// Construct a version from its four ordered components.
    #[must_use]
    pub const fn new(major: u16, minor: u16, maintenance: u16, point: u16) -> Self {
        Self {
            major,
            minor,
            maintenance,
            point,
        }
    }

    /// Parse a dotted version string (`"16.1.3.2"`, `"15.1"`, `"13.1.0.8"`).
    ///
    /// Up to four numeric components are read; missing components default to
    /// `0` and any component beyond the fourth is ignored. Returns `None` when
    /// the leading component is absent or non-numeric.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.trim().split('.');
        let major: u16 = parts.next()?.trim().parse().ok()?;
        let mut next = || {
            parts
                .next()
                .and_then(|p| p.trim().parse().ok())
                .unwrap_or(0)
        };
        Some(Self::new(major, next(), next(), next()))
    }
}

/// A half-open BIG-IP version range `[min, max)` — `min` inclusive, `max`
/// exclusive. `None` on either side means unbounded in that direction.
///
/// Half-open ranges tile the version line without gaps or overlaps: a default
/// that applied `[13.0, 14.0)` and its successor `[14.0, …)` meet exactly at
/// `14.0` with no ambiguity.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VersionRange {
    /// Inclusive lower bound; `None` = "since the beginning".
    pub min: Option<BigipVersion>,
    /// Exclusive upper bound; `None` = "current and later".
    pub max: Option<BigipVersion>,
}

impl VersionRange {
    /// The all-versions range.
    pub const UNBOUNDED: Self = Self {
        min: None,
        max: None,
    };

    /// `[min, ∞)` — the value applies from `min` onward (current and later).
    #[must_use]
    pub const fn from(min: BigipVersion) -> Self {
        Self {
            min: Some(min),
            max: None,
        }
    }

    /// `[−∞, max)` — the value applied before `max`.
    #[must_use]
    pub const fn until(max: BigipVersion) -> Self {
        Self {
            min: None,
            max: Some(max),
        }
    }

    /// `[min, max)`.
    #[must_use]
    pub const fn between(min: BigipVersion, max: BigipVersion) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
        }
    }

    /// Whether `v` falls in this range.
    #[must_use]
    pub fn contains(&self, v: BigipVersion) -> bool {
        self.min.is_none_or(|m| v >= m) && self.max.is_none_or(|m| v < m)
    }

    /// Whether this range has no upper bound — i.e. it is the "current" value.
    #[must_use]
    pub const fn is_current(&self) -> bool {
        self.max.is_none()
    }
}

/// One field default valid across a version range.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FieldDefault {
    /// tmsh field name (e.g. `"idle-timeout"`, `"insert-xforwarded-for"`).
    pub field: &'static str,
    /// The default value captured for this version.
    pub value: &'static str,
    /// BIG-IP versions this default applies to.
    pub range: VersionRange,
}

/// Default field values for one profile type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProfileDefaults {
    /// Registry profile *type* name, uppercase (matches
    /// [`crate::profiles::ProfileSpec::name`]): `"TCP"`, `"HTTP"`, `"CLIENTSSL"`.
    pub profile: &'static str,
    /// tmsh object kind these defaults belong to (`"ltm profile tcp"`).
    pub tmsh_kind: &'static str,
    /// Field defaults, one or more entries per field (disjoint version ranges).
    pub fields: &'static [FieldDefault],
}

/// Resolve the effective default value of `field` on profile type
/// `profile_type` (case-insensitive, underscores ignored) at `version`.
///
/// `version = None` selects the current (newest) snapshot. `Some(v)` floor-
/// matches: the snapshot covering `v`, or the oldest snapshot when `v` predates
/// all of them. Returns `None` only when the profile or field is unknown.
#[must_use]
pub fn profile_field_default(
    profile_type: &str,
    field: &str,
    version: Option<BigipVersion>,
) -> Option<&'static str> {
    let key = normalise_type(profile_type);
    let spec = PROFILE_DEFAULTS
        .iter()
        .find(|p| normalise_type(p.profile) == key)?;
    resolve_field(spec, field, version)
}

/// All effective field defaults for `profile_type` at `version`, one value per
/// field, sorted by field name. See [`profile_field_default`] for the
/// version-selection rule.
#[must_use]
pub fn profile_field_defaults(
    profile_type: &str,
    version: Option<BigipVersion>,
) -> Vec<(&'static str, &'static str)> {
    let key = normalise_type(profile_type);
    let Some(spec) = PROFILE_DEFAULTS
        .iter()
        .find(|p| normalise_type(p.profile) == key)
    else {
        return Vec::new();
    };
    let mut fields: Vec<&'static str> = spec.fields.iter().map(|f| f.field).collect();
    fields.sort_unstable();
    fields.dedup();
    fields
        .into_iter()
        .filter_map(|f| resolve_field(spec, f, version).map(|v| (f, v)))
        .collect()
}

/// The tmsh object kind (`"ltm profile tcp"`) recorded for a profile type, or
/// `None` when the type has no default set.
#[must_use]
pub fn profile_tmsh_kind(profile_type: &str) -> Option<&'static str> {
    let key = normalise_type(profile_type);
    PROFILE_DEFAULTS
        .iter()
        .find(|p| normalise_type(p.profile) == key)
        .map(|p| p.tmsh_kind)
}

/// All profile types that carry a default set, sorted.
#[must_use]
pub fn profiles_with_defaults() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = PROFILE_DEFAULTS.iter().map(|p| p.profile).collect();
    names.sort_unstable();
    names
}

/// Pick the default for one field of `spec` at `version`.
///
/// `version = None` selects the current (newest) snapshot. `Some(v)` floor-
/// matches: the snapshot whose range contains `v`, or — when `v` predates every
/// snapshot we have — the oldest snapshot (clamp up to the lowest recorded
/// version rather than returning nothing).
fn resolve_field(
    spec: &ProfileDefaults,
    field: &str,
    version: Option<BigipVersion>,
) -> Option<&'static str> {
    // A hand-authored cross-version split (FIELD_OVERRIDES) takes precedence
    // over the single-snapshot generated entry for a (profile, field); the
    // generated field's own entries apply otherwise. The generated table is one
    // snapshot per field, so this is where pre-snapshot version history lives.
    let candidates: Vec<&FieldDefault> = match field_override(spec.profile, field) {
        Some(over) => over.iter().collect(),
        None => spec.fields.iter().filter(|f| f.field == field).collect(),
    };
    match version {
        Some(v) => candidates
            .iter()
            .find(|f| f.range.contains(v))
            // Floor-clamp: report older than any snapshot → oldest snapshot
            // (smallest lower bound; an unbounded-below range sorts first).
            .or_else(|| candidates.iter().min_by_key(|f| f.range.min))
            .map(|f| f.value),
        // Current value: rank an open upper bound above any bounded range, then
        // by highest lower bound (newest recorded range). `Option<BigipVersion>`
        // and `bool` both order the way we want (`false < true`, `None < Some`).
        None => candidates
            .iter()
            .max_by_key(|f| (f.range.is_current(), f.range.min))
            .map(|f| f.value),
    }
}

/// A hand-authored cross-version split for one `(profile, field)`.
///
/// The generated table records a single snapshot value per field (see
/// [`generated`]); it cannot express a default that changed across TMOS
/// releases. An override lists the full version history for that field, and
/// [`resolve_field`] uses it *instead of* the generated entry.
///
/// Invariant (guarded by `overrides_agree_with_generated_snapshot`): the
/// newest band's value must equal the generated snapshot value, so an override
/// only *adds* older history and never silently diverges from regeneration.
struct FieldOverride {
    /// Profile type, matched via [`normalise_type`].
    profile: &'static str,
    /// The field whose generated snapshot entry these bands replace.
    field: &'static str,
    /// Version-ranged values, newest band last.
    values: &'static [FieldDefault],
}

/// Cross-version splits the single-snapshot generated table can't express.
///
/// `client-ssl` / `server-ssl` `options` gained TLS/DTLS opt-outs over time.
/// BIG-IP 21.1 then made the canonical profiles TLS 1.2/1.3-only while moving
/// the older behaviour to `clientssl-legacy` / `serverssl-legacy`.
static FIELD_OVERRIDES: &[FieldOverride] = &[
    FieldOverride {
        profile: "CLIENTSSL",
        field: "options",
        values: SSL_OPTIONS_BANDS,
    },
    FieldOverride {
        profile: "SERVERSSL",
        field: "options",
        values: SSL_OPTIONS_BANDS,
    },
];

/// Shared `options` history for the client/server SSL profiles (identical).
static SSL_OPTIONS_BANDS: &[FieldDefault] = &[
    FieldDefault {
        field: "options",
        value: "dont-insert-empty-fragments",
        range: VersionRange::until(BigipVersion::new(14, 0, 0, 0)),
    },
    FieldDefault {
        field: "options",
        value: "dont-insert-empty-fragments no-tlsv1.3",
        range: VersionRange::between(
            BigipVersion::new(14, 0, 0, 0),
            BigipVersion::new(17, 1, 0, 0),
        ),
    },
    FieldDefault {
        field: "options",
        value: "dont-insert-empty-fragments no-tlsv1.3 no-dtlsv1.2",
        range: VersionRange::between(
            BigipVersion::new(17, 1, 0, 0),
            BigipVersion::new(21, 1, 0, 0),
        ),
    },
    FieldDefault {
        field: "options",
        value: "dont-insert-empty-fragments no-tlsv1.1 no-tlsv1 no-ssl",
        range: VersionRange::from(BigipVersion::new(21, 1, 0, 0)),
    },
];

/// The version bands for `(profile, field)`, or `None` when no split is
/// hand-authored (the generated snapshot then applies as-is).
fn field_override(profile: &str, field: &str) -> Option<&'static [FieldDefault]> {
    let key = normalise_type(profile);
    FIELD_OVERRIDES
        .iter()
        .find(|o| o.field == field && normalise_type(o.profile) == key)
        .map(|o| o.values)
}

/// Canonicalise a profile type spelling: uppercase, drop `_`/`-`/`.` and any
/// `ProfileType.` prefix (`"client_ssl"`, `"CLIENT-SSL"`,
/// `"ProfileType.CLIENT_SSL"` → `"CLIENTSSL"`).
fn normalise_type(t: &str) -> String {
    let t = t.rsplit('.').next().unwrap_or(t);
    t.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> BigipVersion {
        BigipVersion::parse(s).unwrap()
    }

    #[test]
    fn version_parses_and_orders() {
        assert_eq!(v("16.1.3.2"), BigipVersion::new(16, 1, 3, 2));
        assert_eq!(v("15.1"), BigipVersion::new(15, 1, 0, 0));
        assert_eq!(v("15.1"), v("15.1.0.0"));
        assert!(v("13.1.0.8") < v("14.0"));
        assert!(v("16.1") > v("16.0.1"));
        assert!(BigipVersion::parse("").is_none());
        assert!(BigipVersion::parse("x.y").is_none());
    }

    #[test]
    fn range_contains_is_half_open() {
        let r = VersionRange::between(v("13.0"), v("14.0"));
        assert!(r.contains(v("13.0"))); // min inclusive
        assert!(r.contains(v("13.1.5")));
        assert!(!r.contains(v("14.0"))); // max exclusive
        assert!(!r.contains(v("12.1")));
        assert!(VersionRange::UNBOUNDED.contains(v("1.0")));
        assert!(VersionRange::from(v("14.0")).contains(v("17.1")));
        assert!(!VersionRange::until(v("14.0")).contains(v("14.0")));
    }

    #[test]
    fn baseline_default_resolves_for_any_version() {
        assert_eq!(
            profile_field_default("TCP", "idle-timeout", None),
            Some("300")
        );
        assert_eq!(
            profile_field_default("tcp", "idle-timeout", Some(v("11.5"))),
            Some("300")
        );
        // case / separator / prefix insensitivity
        assert_eq!(
            profile_field_default("ProfileType.CLIENT_SSL", "ciphers", None),
            Some("none")
        );
        assert_eq!(
            profile_field_default("client-ssl", "renegotiation", None),
            Some("enabled")
        );
    }

    #[test]
    fn version_splits_and_floor_matching() {
        // client-ssl / server-ssl `options` gained opt-outs over releases; the
        // hand-authored override recovers the per-release value.
        let cur = "dont-insert-empty-fragments no-tlsv1.3 no-dtlsv1.2";
        let mid = "dont-insert-empty-fragments no-tlsv1.3";
        let orig = "dont-insert-empty-fragments";
        // 17.1 through 21.0 → the legacy-compatible value.
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", Some(v("17.1"))),
            Some(cur)
        );
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", Some(v("20.1"))),
            Some(cur)
        );
        // [14.0, 17.1) → no-tlsv1.3 but not yet no-dtlsv1.2.
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", Some(v("16.1"))),
            Some(mid)
        );
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", Some(v("14.0"))),
            Some(mid)
        );
        // Before 14.0 → the original value (the override covers it — no clamp).
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", Some(v("13.1.0.8"))),
            Some(orig)
        );
        // 21.1 and no-version current use the new secure canonical profile.
        let secure = "dont-insert-empty-fragments no-tlsv1.1 no-tlsv1 no-ssl";
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", Some(v("21.1"))),
            Some(secure)
        );
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", None),
            Some(secure)
        );
        // server-ssl shares the identical history.
        assert_eq!(
            profile_field_default("SERVERSSL", "options", Some(v("13.1"))),
            Some(orig)
        );

        // A field with only the 17.1 snapshot still floor-clamps up to it for
        // older reports rather than returning nothing.
        assert_eq!(
            profile_field_default("CLIENTSSL", "mode", Some(v("13.1"))),
            Some("enabled")
        );
    }

    /// The newest override band must equal the generated snapshot value, so an
    /// override only adds older history and can't silently diverge on regen.
    #[test]
    fn overrides_agree_with_generated_snapshot() {
        for over in FIELD_OVERRIDES {
            let newest = over
                .values
                .iter()
                .max_by_key(|f| (f.range.is_current(), f.range.min))
                .expect("override has at least one band");
            // The generated table has a single snapshot entry per field; read
            // it directly (bypassing the override) to check the newest band.
            let generated = PROFILE_DEFAULTS
                .iter()
                .find(|p| normalise_type(p.profile) == normalise_type(over.profile))
                .and_then(|p| {
                    p.fields
                        .iter()
                        .filter(|f| f.field == over.field)
                        .max_by_key(|f| (f.range.is_current(), f.range.min))
                })
                .map(|f| f.value);
            assert_eq!(
                Some(newest.value),
                generated,
                "{}::{} newest override band must match the generated snapshot",
                over.profile,
                over.field
            );
        }
    }

    #[test]
    fn unknown_profile_or_field_is_none() {
        assert!(profile_field_default("TCP", "no-such-field", None).is_none());
        assert!(profile_field_default("NOPE", "idle-timeout", None).is_none());
        assert!(profile_field_defaults("NOPE", None).is_empty());
    }

    #[test]
    fn generated_import_is_broad_and_faithful() {
        // The generated table covers the full base-profile set, not a handful.
        assert!(
            profiles_with_defaults().len() >= 50,
            "expected the full base-profile import, got {}",
            profiles_with_defaults().len()
        );
        // Real captured values (not guessed): the base tcp proxy
        // buffer is 65535, and the profile carries dozens of fields.
        assert_eq!(
            profile_field_default("tcp", "proxy-buffer-high", None),
            Some("65535")
        );
        assert!(profile_field_defaults("tcp", None).len() >= 40);
        assert_eq!(
            profile_field_default("clientssl", "cipher-group", Some(v("21.1"))),
            Some("/Common/f5-default")
        );
        assert_eq!(
            profile_field_default("clientssl", "ciphers", Some(v("20.1"))),
            Some("DEFAULT")
        );
        assert_eq!(
            profile_field_default("clientssl", "ciphers", Some(v("21.1"))),
            Some("none")
        );
        assert_eq!(
            profile_field_default("json", "maximum-entries", Some(v("21.1"))),
            Some("2048")
        );
        assert_eq!(profile_tmsh_kind("aimcp"), Some("ltm profile aimcp"));
        // A nested block is flattened to a faithful tmsh string.
        assert_eq!(
            profile_field_default("clientssl", "cert-key-chain", None),
            Some(
                "{ default { cert /Common/default.crt chain none key /Common/default.key passphrase none } }"
            )
        );
        // Application + transport + TLS + niche types all present.
        for ty in [
            "TCP",
            "UDP",
            "FASTL4",
            "HTTP",
            "HTTP2",
            "CLIENTSSL",
            "SERVERSSL",
            "ONECONNECT",
            "SCTP",
            "REWRITE",
        ] {
            assert!(
                !profile_field_defaults(ty, None).is_empty() || profile_tmsh_kind(ty).is_some(),
                "missing profile type {ty}"
            );
        }
    }

    #[test]
    fn defaults_are_sorted_deduped_and_single_valued() {
        let d = profile_field_defaults("CLIENTSSL", Some(v("16.1")));
        // sorted by field
        let names: Vec<&str> = d.iter().map(|(f, _)| *f).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        // `options` resolves to exactly one value — and at 16.1 that is the
        // [14.0, 17.1) band (no-tlsv1.3, not yet no-dtlsv1.2).
        assert_eq!(d.iter().filter(|(f, _)| *f == "options").count(), 1);
        assert_eq!(
            d.iter().find(|(f, _)| *f == "options").map(|(_, v)| *v),
            Some("dont-insert-empty-fragments no-tlsv1.3")
        );
    }

    #[test]
    fn tmsh_kind_and_inventory() {
        assert_eq!(profile_tmsh_kind("HTTP"), Some("ltm profile http"));
        assert_eq!(
            profile_tmsh_kind("ONECONNECT"),
            Some("ltm profile one-connect")
        );
        assert!(profile_tmsh_kind("NOPE").is_none());
        let all = profiles_with_defaults();
        assert!(all.contains(&"TCP"));
        assert!(all.contains(&"CLIENTSSL"));
        // sorted
        let mut s = all.clone();
        s.sort_unstable();
        assert_eq!(all, s);
    }

    #[test]
    fn every_field_resolves_at_current_and_ranges_are_disjoint() {
        for spec in PROFILE_DEFAULTS {
            // every field yields a current value
            for f in spec.fields {
                assert!(
                    profile_field_default(spec.profile, f.field, None).is_some(),
                    "{}::{} has no current default",
                    spec.profile,
                    f.field
                );
            }
            // per-field ranges must not overlap (half-open tiling)
            let mut by_field: std::collections::HashMap<&str, Vec<&FieldDefault>> =
                std::collections::HashMap::new();
            for f in spec.fields {
                by_field.entry(f.field).or_default().push(f);
            }
            for (field, entries) in by_field {
                for (i, a) in entries.iter().enumerate() {
                    for b in &entries[i + 1..] {
                        let overlap = a
                            .range
                            .min
                            .is_none_or(|m| b.range.max.is_none_or(|x| m < x))
                            && b.range
                                .min
                                .is_none_or(|m| a.range.max.is_none_or(|x| m < x));
                        assert!(
                            !overlap,
                            "{}::{field} has overlapping version ranges {:?} / {:?}",
                            spec.profile, a.range, b.range
                        );
                    }
                }
            }
        }
    }
}
