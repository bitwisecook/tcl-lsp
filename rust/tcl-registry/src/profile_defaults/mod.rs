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
//! profile field that is still at its TMOS default, because those values live
//! in `/config/profile_base.conf`, the read-only base config every BIG-IP ships
//! with. Reconstructing a profile's *effective* configuration from an SCF
//! therefore requires that default set. Worse, the defaults themselves drift
//! across releases (a field is added, a default is retuned, an option flips), so
//! a single value per field is not enough — each default is keyed by the BIG-IP
//! **version range** over which it applies.
//!
//! This module is that table. It is the registry-side source of truth for
//! "what does field X of an unmodified `ltm profile <type>` resolve to on TMOS
//! version V", consumed by `f5 query` (the `profile_defaults` builtin), the
//! BIG-IP report (to show effective values for base profiles an SCF never
//! re-declares), and the LSP.
//!
//! ## Data + import
//!
//! [`PROFILE_DEFAULTS`] is generated (`generated.rs`) by
//! `scripts/registry-audit/gen_profile_defaults.py` from a real
//! `profile_base.conf` — the read-only default-profile base every BIG-IP ships
//! (sourced from f5-corkscrew's Apache-2.0 archive fixtures). One snapshot
//! yields version-unbounded entries; known cross-release changes a single
//! snapshot can't express (e.g. the client/server-ssl `options` gaining
//! `no-tlsv1.3` at 14.0) are recorded as an `OVERRIDES` table in the generator
//! and emitted as adjacent half-open ranges. Refresh by re-running the
//! generator against a newer `profile_base.conf`; add range boundaries to
//! `OVERRIDES` as releases retune a default. The resolver treats an open upper
//! bound as the latest known value.

mod generated;

/// The full profile default table (generated from `profile_base.conf`). See the
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
        let mut next = || parts.next().and_then(|p| p.trim().parse().ok()).unwrap_or(0);
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
    /// The default value as it appears in `profile_base.conf` for this range.
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
/// `version = None` selects the current value — the entry whose range has an
/// open upper bound ([`VersionRange::is_current`]); when every entry is bounded
/// the highest-versioned one is used. Returns `None` when the profile/field is
/// unknown or no recorded range covers `version`.
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
fn resolve_field(
    spec: &ProfileDefaults,
    field: &str,
    version: Option<BigipVersion>,
) -> Option<&'static str> {
    let mut candidates = spec.fields.iter().filter(|f| f.field == field);
    match version {
        Some(v) => candidates.find(|f| f.range.contains(v)).map(|f| f.value),
        // Current value: rank an open upper bound above any bounded range, then
        // by highest lower bound (newest recorded range). `Option<BigipVersion>`
        // and `bool` both order the way we want (`false < true`, `None < Some`).
        None => candidates
            .max_by_key(|f| (f.range.is_current(), f.range.min))
            .map(|f| f.value),
    }
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
            Some("DEFAULT")
        );
        assert_eq!(
            profile_field_default("client-ssl", "renegotiation", None),
            Some("enabled")
        );
    }

    #[test]
    fn version_split_selects_the_right_side() {
        // Before 14.0: no TLS 1.3 flag in the base options.
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", Some(v("13.1.0.8"))),
            Some("dont-insert-empty-fragments")
        );
        // 14.0 and later: TLS 1.3 disabled by default.
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", Some(v("14.0"))),
            Some("dont-insert-empty-fragments no-tlsv1.3")
        );
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", Some(v("16.1.3"))),
            Some("dont-insert-empty-fragments no-tlsv1.3")
        );
        // No version → current value (open upper bound).
        assert_eq!(
            profile_field_default("CLIENTSSL", "options", None),
            Some("dont-insert-empty-fragments no-tlsv1.3")
        );
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
            "expected the full profile_base.conf import, got {}",
            profiles_with_defaults().len()
        );
        // Real values from profile_base.conf (not guessed): the base tcp proxy
        // buffer is 65535, and the profile carries dozens of fields.
        assert_eq!(
            profile_field_default("tcp", "proxy-buffer-high", None),
            Some("65535")
        );
        assert!(profile_field_defaults("tcp", None).len() >= 40);
        // A nested block is flattened to a faithful tmsh string.
        assert_eq!(
            profile_field_default("clientssl", "cert-key-chain", None),
            Some("{ default { cert /Common/default.crt chain none key /Common/default.key passphrase none } }")
        );
        // Application + transport + TLS + niche types all present.
        for ty in ["TCP", "UDP", "FASTL4", "HTTP", "HTTP2", "CLIENTSSL", "SERVERSSL", "ONECONNECT", "SCTP", "REWRITE"] {
            assert!(
                !profile_field_defaults(ty, None).is_empty()
                    || profile_tmsh_kind(ty).is_some(),
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
        // `options` appears exactly once despite two ranged entries
        assert_eq!(d.iter().filter(|(f, _)| *f == "options").count(), 1);
        assert_eq!(
            d.iter().find(|(f, _)| *f == "options").map(|(_, v)| *v),
            Some("dont-insert-empty-fragments no-tlsv1.3")
        );
    }

    #[test]
    fn tmsh_kind_and_inventory() {
        assert_eq!(profile_tmsh_kind("HTTP"), Some("ltm profile http"));
        assert_eq!(profile_tmsh_kind("ONECONNECT"), Some("ltm profile one-connect"));
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
                        let overlap = a.range.min.is_none_or(|m| b.range.max.is_none_or(|x| m < x))
                            && b.range.min.is_none_or(|m| a.range.max.is_none_or(|x| m < x));
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
