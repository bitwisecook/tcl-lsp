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

//! Base tmsh objects a BIG-IP ships with — the single-configuration-file
//! fallback.
//!
//! A single-configuration file (SCF) omits every object that lives in the
//! read-only base configuration a BIG-IP loads before `bigip.conf`, because
//! those objects (default profiles, ciphers, system iRules, protocol
//! signatures, app-classification categories, …) are assumed present. A
//! qkview or UCS, by contrast, carries the objects verbatim. Reconstructing an
//! effective configuration from an SCF therefore needs the base set embedded so
//! that references resolve and base objects are never mistaken for user
//! objects.
//!
//! This module is that embedded set, split by what it can offer:
//!
//! * [`BASE_OBJECTS`] — base objects with their default field values, for the
//!   handful of kinds whose fields an SCF actually omits (profiles are excluded
//!   here; their version-ranged defaults live in [`crate::profile_defaults`]).
//! * [`BASE_OBJECT_NAMES`] — the identities `(module, object_type, name)` of
//!   base objects that carry no embeddable field defaults but must still be
//!   known to exist (e.g. app-classification categories, IPS signatures).
//!
//! When a qkview/UCS provides an object, its real values take precedence; these
//! tables are consulted only to fill what an SCF leaves out.

use crate::profile_defaults::BigipVersion;

mod generated_names;
mod generated_objects;

pub use generated_names::BASE_OBJECT_NAMES;
pub use generated_objects::BASE_OBJECTS;

/// The BIG-IP version these base objects were captured at. Field values reflect
/// this release; a report for an older release uses them as its best-available
/// fallback (there is only this one snapshot). See [`crate::profile_defaults`]
/// for the version floor-matching policy this mirrors.
pub const BASE_OBJECTS_VERSION: BigipVersion = BigipVersion::new(17, 1, 0, 0);

/// One base tmsh object and its default field values.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BaseObject {
    /// tmsh module word (`ltm`, `security`, `sys`, …).
    pub module: &'static str,
    /// tmsh object-type word(s) after the module (`profile tcp`, `rule`,
    /// `cipher rule`, …).
    pub object_type: &'static str,
    /// Object name as it appears in the base config (`clientssl`,
    /// `/Common/_sys_https_redirect`, …).
    pub name: &'static str,
    /// Flattened default fields `(field, value)`, in base-config order. Nested
    /// blocks and lists are rendered as their tmsh string form.
    pub fields: &'static [(&'static str, &'static str)],
}

impl BaseObject {
    /// The default value of `field`, or `None` when this object does not carry
    /// it. The first matching entry wins (base-config order).
    #[must_use]
    pub fn field(&self, field: &str) -> Option<&'static str> {
        self.fields
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, v)| *v)
    }
}

/// Look up a field-bearing base object by its full identity.
#[must_use]
pub fn base_object(module: &str, object_type: &str, name: &str) -> Option<&'static BaseObject> {
    BASE_OBJECTS
        .iter()
        .find(|o| o.module == module && o.object_type == object_type && o.name == name)
}

/// The default value of `field` on the base object identified by
/// `(module, object_type, name)`, or `None` when the object or field is
/// unknown.
#[must_use]
pub fn base_object_field(
    module: &str,
    object_type: &str,
    name: &str,
    field: &str,
) -> Option<&'static str> {
    base_object(module, object_type, name).and_then(|o| o.field(field))
}

/// Whether `(module, object_type, name)` names a base object — one shipped in
/// the read-only base config, whether or not it carries embedded field
/// defaults. Consults both [`BASE_OBJECTS`] and [`BASE_OBJECT_NAMES`].
#[must_use]
pub fn is_base_object(module: &str, object_type: &str, name: &str) -> bool {
    BASE_OBJECTS
        .iter()
        .any(|o| o.module == module && o.object_type == object_type && o.name == name)
        || BASE_OBJECT_NAMES
            .iter()
            .any(|&(m, t, n)| m == module && t == object_type && n == name)
}

/// All base-object identities `(module, object_type, name)` — the field-bearing
/// ones followed by the name-only ones.
pub fn base_object_identities() -> impl Iterator<Item = (&'static str, &'static str, &'static str)>
{
    BASE_OBJECTS
        .iter()
        .map(|o| (o.module, o.object_type, o.name))
        .chain(BASE_OBJECT_NAMES.iter().copied())
}

/// Total number of tracked base objects (field-bearing plus name-only).
#[must_use]
pub fn base_object_count() -> usize {
    BASE_OBJECTS.len() + BASE_OBJECT_NAMES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_broad() {
        // The embedded base set is large: thousands of objects across both
        // tables (profiles excluded — see profile_defaults).
        assert!(
            base_object_count() > 5000,
            "expected a broad base set, got {}",
            base_object_count()
        );
        assert!(!BASE_OBJECTS.is_empty());
        assert!(!BASE_OBJECT_NAMES.is_empty());
    }

    #[test]
    fn field_bearing_lookup_resolves() {
        // Every field-bearing object round-trips through the identity lookup and
        // carries scalar/list default values.
        let sample = BASE_OBJECTS[0];
        let got = base_object(sample.module, sample.object_type, sample.name)
            .expect("field-bearing object resolves");
        assert_eq!(got, &sample);
        assert!(!got.fields.is_empty(), "field-bearing objects carry fields");
        // A field looks up by name.
        let (f, v) = sample.fields[0];
        assert_eq!(got.field(f), Some(v));
    }

    #[test]
    fn system_irules_keep_events_with_empty_bodies() {
        // A base system iRule keeps its `when EVENT` handlers so flow charts can
        // render its events — but each body is emptied, never embedding the
        // verbatim Tcl.
        let rule = base_object("ltm", "rule", "_sys_https_redirect")
            .expect("the base https-redirect system rule is tracked");
        let handlers: Vec<&str> = rule
            .fields
            .iter()
            .filter(|(f, _)| *f == "when")
            .map(|(_, v)| *v)
            .collect();
        assert!(!handlers.is_empty(), "the rule declares event handlers");
        // Every handler body is empty: `EVENT { }`, no verbatim logic.
        for h in &handlers {
            assert!(
                h.ends_with("{ }"),
                "handler body must be emptied, got {h:?}"
            );
        }
        assert!(handlers.iter().any(|h| h.starts_with("HTTP_REQUEST")));
    }

    #[test]
    fn name_only_identities_are_known_but_carry_no_fields() {
        // App-classification categories are tracked name-only.
        let (m, t, n) = BASE_OBJECT_NAMES[0];
        assert!(is_base_object(m, t, n));
        // Name-only entries are absent from the field-bearing table.
        assert!(base_object(m, t, n).is_none() || base_object(m, t, n).unwrap().fields.is_empty());
    }

    #[test]
    fn unknown_object_is_not_base() {
        assert!(!is_base_object("ltm", "pool", "/Common/__no_such_pool__"));
        assert!(base_object("ltm", "pool", "/Common/__no_such_pool__").is_none());
        assert!(base_object_field("ltm", "rule", "/Common/__nope__", "when").is_none());
    }

    #[test]
    fn identities_iterator_covers_both_tables() {
        let n = base_object_identities().count();
        assert_eq!(n, base_object_count());
    }
}
