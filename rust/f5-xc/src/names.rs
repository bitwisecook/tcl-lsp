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

//! XC object names.
//!
//! `ves.io.schema.ObjectCreateMetaType.name` is required and must follow
//! DNS-1035: a leading letter, then lowercase letters, digits and hyphens,
//! 63 characters at most. A BIG-IP object path satisfies none of that —
//! `/Common/web-pool` leads with a separator and carries two — so every
//! rendering derives the XC name through here rather than passing the path
//! straight out.

use std::fmt::Write as _;

/// Longest DNS-1035 label.
const MAX_LEN: usize = 63;

fn is_dns1035(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && name.len() <= MAX_LEN
        && !name.ends_with('-')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// FNV-1a 32-bit hash — deterministic, dependency-free, used only to keep
/// two different paths that normalise alike from becoming one object.
fn fnv1a_32(s: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for b in s.as_bytes() {
        hash ^= u32::from(*b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Derive a DNS-1035 XC object name from a BIG-IP object path.
///
/// A name that already satisfies DNS-1035 is returned unchanged, so an
/// unqualified `web-pool` stays itself. Anything else is folded to lowercase
/// with every other character becoming a hyphen, which keeps the partition in
/// the name — `/Common/web-pool` becomes `common-web-pool-…` rather than
/// dropping to `web-pool` and colliding with the same pool name in another
/// partition. Because that folding is lossy on its own (`web_pool` and
/// `web-pool` fold alike), a hash of the original path is appended whenever
/// the name had to change, exactly as `hcl_ident` disambiguates Terraform
/// labels. The original path is not lost: each rendering carries it in the
/// object's description.
#[must_use]
pub fn xc_object_name(bigip_path: &str) -> String {
    if is_dns1035(bigip_path) {
        return bigip_path.to_owned();
    }

    let folded: String = bigip_path
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                c
            } else {
                '-'
            }
        })
        .collect();

    // Collapse runs of hyphens and trim them from both ends.
    let mut out = String::with_capacity(folded.len());
    for part in folded.split('-').filter(|p| !p.is_empty()) {
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(part);
    }

    // DNS-1035 wants a letter first; a path that folded to digits or to
    // nothing at all still has to produce a usable name.
    if !out.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        out.insert_str(0, "xc-");
    }

    let suffix = format!("-{:08x}", fnv1a_32(bigip_path));
    out.truncate(MAX_LEN - suffix.len());
    while out.ends_with('-') {
        out.pop();
    }
    let _ = write!(out, "{suffix}");
    out
}

/// The description for an object that stands for a named BIG-IP object,
/// naming the path it came from so the original survives the rename.
#[must_use]
pub fn derived_from(bigip_path: &str) -> String {
    format!("Translated from BIG-IP {bigip_path}")
}

/// The description for an object the translation invents rather than
/// carries over — the load balancer and its service policy answer to no
/// BIG-IP object, so naming one would be a lie.
pub const GENERATED: &str = "Generated from an iRule by tcl-lsp";

#[cfg(test)]
mod tests {
    use super::*;

    fn valid(name: &str) -> bool {
        is_dns1035(name)
    }

    #[test]
    fn an_already_valid_name_is_untouched() {
        assert_eq!(xc_object_name("web-pool"), "web-pool");
        assert_eq!(xc_object_name("api2"), "api2");
    }

    #[test]
    fn a_partition_qualified_path_keeps_its_partition() {
        let name = xc_object_name("/Common/web-pool");
        assert!(name.starts_with("common-web-pool-"), "{name}");
        assert!(valid(&name), "{name}");
    }

    #[test]
    fn the_same_pool_in_two_partitions_stays_two_objects() {
        let common = xc_object_name("/Common/web-pool");
        let other = xc_object_name("/Tenant-A/web-pool");
        assert_ne!(common, other);
        assert!(valid(&common) && valid(&other));
    }

    #[test]
    fn names_that_fold_alike_stay_distinct() {
        // The fold maps `_` and `-` to the same character, so only the hash
        // keeps these apart.
        assert_ne!(
            xc_object_name("/Common/web_pool"),
            xc_object_name("/Common/web-pool")
        );
    }

    #[test]
    fn a_name_without_a_leading_letter_still_becomes_valid() {
        for path in ["/9lives", "/Common/2fast", "////", "/Common/ünïcode"] {
            let name = xc_object_name(path);
            assert!(valid(&name), "{path} -> {name}");
        }
    }

    #[test]
    fn a_very_long_path_is_truncated_to_a_valid_label() {
        let name = xc_object_name(&format!("/Common/{}", "a".repeat(200)));
        assert!(valid(&name), "{name}");
        assert!(name.len() <= MAX_LEN, "{} chars: {name}", name.len());
    }

    #[test]
    fn the_description_names_the_source_object() {
        assert_eq!(
            derived_from("/Common/web-pool"),
            "Translated from BIG-IP /Common/web-pool"
        );
    }
}
