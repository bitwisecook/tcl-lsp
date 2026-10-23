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

//! Authored BIG-IP configuration object-name matching.
//!
//! Paths remain literal F5 slash-domain keys. Policies describe local tooling
//! contracts; they do not establish appliance or Tcl interpreter resolution.

/// Read the first component of an absolute authored F5 object path.
/// Empty components remain empty; this neither validates nor resolves a path.
#[must_use]
pub fn object_path_partition(path: &str) -> Option<&str> {
    path.strip_prefix('/')
        .and_then(|rest| rest.split('/').next())
}

/// Matching precedence within a caller-supplied object domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectNamePolicy<'a> {
    /// Exact literal keys only.
    Exact,
    /// Exact, the attached object's partition, then matching reference leaves.
    ExactThenOwnerPartitionOrReferenceLeaf {
        /// Actual authored path of the referring object.
        owner: &'a str,
    },
    /// Exact, default partition, Common, then slash-delimited suffix.
    Configuration {
        /// Authored default partition; an empty or slash-only value means Common.
        default_partition: &'a str,
    },
    /// Exact, a Common-prefixed key, then slash-delimited suffix.
    CommonThenSuffix,
    /// Exact, then candidates with the same leaf as the reference.
    ExactThenReferenceLeaf,
    /// Exact, then candidates whose leaf equals the complete reference.
    ExactThenShortName,
    /// Exact, then short-name or Common-prefixed candidates in input order.
    ExactThenShortNameOrCommon,
    /// Exact or suffix candidates together in input order, including absolute suffixes.
    ExactOrSuffix,
}

impl ObjectNamePolicy<'_> {
    fn priority(self, reference: &str, path: &str) -> Option<u8> {
        if path == reference {
            return Some(0);
        }
        let suffix_matches = || {
            path.strip_suffix(reference)
                .is_some_and(|prefix| prefix.ends_with('/'))
        };
        match self {
            Self::Exact => None,
            Self::ExactThenOwnerPartitionOrReferenceLeaf { owner } => {
                if !reference.starts_with('/')
                    && let Some(partition) = object_path_partition(owner)
                    && qualified_path_equals(path, partition, reference)
                {
                    return Some(1);
                }
                (leaf(path) == leaf(reference)).then_some(2)
            }
            Self::Configuration { default_partition } => {
                if !reference.starts_with('/') {
                    let trimmed = default_partition.trim_matches('/');
                    let partition = if trimmed.is_empty() {
                        "Common"
                    } else {
                        trimmed
                    };
                    if qualified_path_equals(path, partition, reference) {
                        return Some(1);
                    }
                    if partition != "Common" && path.strip_prefix("/Common/") == Some(reference) {
                        return Some(2);
                    }
                }
                suffix_matches().then_some(3)
            }
            Self::CommonThenSuffix => {
                if path.strip_prefix("/Common/") == Some(reference) {
                    Some(1)
                } else {
                    suffix_matches().then_some(2)
                }
            }
            Self::ExactThenReferenceLeaf => (leaf(path) == leaf(reference)).then_some(1),
            Self::ExactThenShortName => (leaf(path) == reference).then_some(1),
            Self::ExactThenShortNameOrCommon => (leaf(path) == reference
                || path.strip_prefix("/Common/") == Some(reference))
            .then_some(1),
            Self::ExactOrSuffix => (if reference.starts_with('/') {
                path.ends_with(reference)
            } else {
                suffix_matches()
            })
            .then_some(0),
        }
    }
}

fn qualified_path_equals(path: &str, partition: &str, reference: &str) -> bool {
    path.strip_prefix('/')
        .and_then(|path| path.strip_prefix(partition))
        .and_then(|path| path.strip_prefix('/'))
        == Some(reference)
}

fn leaf(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// All candidates in the highest-precedence matching tier, in input order.
#[derive(Debug)]
pub struct ObjectNameCandidates<T> {
    candidates: Vec<T>,
    exact_tier: bool,
}

impl<T> ObjectNameCandidates<T> {
    /// Inspect every candidate without treating ambiguity as identity.
    #[must_use]
    pub fn candidates(&self) -> &[T] {
        &self.candidates
    }

    /// Select the first candidate under an explicitly ordered tooling contract.
    #[must_use]
    pub fn first_in_order(self) -> Option<T> {
        self.candidates.into_iter().next()
    }

    /// Select the first exact-key candidate, otherwise require a unique fallback.
    /// Duplicate exact keys retain their input order and do not become unique identity.
    #[must_use]
    pub fn first_exact_in_order_or_unique(self) -> Option<T> {
        if self.exact_tier {
            self.first_in_order()
        } else {
            self.unique()
        }
    }

    /// Select only a single candidate, refusing absent or ambiguous matches.
    #[must_use]
    pub fn unique(self) -> Option<T> {
        if self.candidates.len() == 1 {
            self.candidates.into_iter().next()
        } else {
            None
        }
    }
}

/// Match a literal object reference within the caller's already-filtered domain.
///
/// Callers supply module, object-type and configuration ownership constraints;
/// the matcher preserves their order and exposes ambiguity before selection.
#[must_use]
pub fn object_name_candidates<'a, T>(
    reference: &str,
    entries: impl IntoIterator<Item = (&'a str, T)>,
    policy: ObjectNamePolicy<'_>,
) -> ObjectNameCandidates<T> {
    let mut candidates = Vec::new();
    let mut selected_priority = None;
    for (path, value) in entries {
        let Some(priority) = policy.priority(reference, path) else {
            continue;
        };
        if selected_priority.is_none_or(|selected| priority < selected) {
            candidates.clear();
            selected_priority = Some(priority);
        }
        if selected_priority == Some(priority) {
            candidates.push(value);
        }
    }
    ObjectNameCandidates {
        candidates,
        exact_tier: selected_priority == Some(0) && policy != ObjectNamePolicy::ExactOrSuffix,
    }
}

/// Treatment of an empty service suffix in an authored object reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmptyServiceSuffix {
    /// Leave a trailing colon in the object key.
    Retain,
    /// Remove a trailing colon as well as a numeric service suffix.
    Strip,
}

/// Remove one numeric colon suffix while preserving IPv6 and nonnumeric keys.
#[must_use]
pub fn reference_without_numeric_service(reference: &str, empty: EmptyServiceSuffix) -> &str {
    if reference.matches(':').count() == 1
        && let Some((path, service)) = reference.rsplit_once(':')
        && (empty == EmptyServiceSuffix::Strip || !service.is_empty())
        && service.bytes().all(|byte| byte.is_ascii_digit())
    {
        path
    } else {
        reference
    }
}

/// Construct exact registry-reference paths in the owner's partition and Common.
///
/// The input is an already extracted configuration reference. This domain does
/// not search arbitrary partitions or canonicalise Tcl namespace separators.
#[must_use]
pub fn owner_partition_reference_paths(
    reference: &str,
    owner: &str,
    default_partition: &str,
) -> Vec<String> {
    if reference.starts_with('/') {
        return vec![reference.to_owned()];
    }
    let partition = object_path_partition(owner)
        .filter(|partition| !partition.is_empty())
        .unwrap_or(default_partition.trim_matches('/'));
    let mut candidates = vec![format!("/{partition}/{reference}")];
    if partition != "Common" {
        candidates.push(format!("/Common/{reference}"));
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::{ObjectNamePolicy, object_name_candidates};

    fn matches<'a>(
        reference: &str,
        paths: &'a [&'a str],
        policy: ObjectNamePolicy<'_>,
    ) -> Vec<&'a str> {
        object_name_candidates(reference, paths.iter().map(|path| (*path, *path)), policy)
            .candidates
    }

    #[test]
    fn configuration_precedence_is_independent_of_source_order() {
        let paths = ["/Other/p", "/Common/p", "/Tenant/p", "p"];
        let policy = ObjectNamePolicy::Configuration {
            default_partition: "/Tenant/",
        };
        assert_eq!(matches("p", &paths, policy), ["p"]);
        assert_eq!(matches("p", &paths[..3], policy), ["/Tenant/p"]);
        assert_eq!(matches("p", &paths[..2], policy), ["/Common/p"]);
        assert_eq!(matches("p", &paths[..1], policy), ["/Other/p"]);
        assert_eq!(matches("missing", &paths, policy), [] as [&str; 0]);
        assert_eq!(
            matches(
                "p",
                &paths[..3],
                ObjectNamePolicy::Configuration {
                    default_partition: "///"
                }
            ),
            ["/Common/p"]
        );
    }

    #[test]
    fn ambiguity_requires_an_explicit_selection_contract() {
        let paths = ["/Z/p", "/A/p"];
        let select = || {
            object_name_candidates(
                "p",
                paths.iter().map(|path| (*path, *path)),
                ObjectNamePolicy::Configuration {
                    default_partition: "Tenant",
                },
            )
        };
        assert_eq!(select().candidates(), paths);
        assert_eq!(select().first_in_order(), Some("/Z/p"));
        assert_eq!(select().unique(), None);
        assert_eq!(
            object_name_candidates(
                "/A/p",
                paths.iter().map(|path| (*path, *path)),
                ObjectNamePolicy::ExactThenReferenceLeaf
            )
            .unique(),
            Some("/A/p")
        );
    }

    #[test]
    fn short_and_reference_leaf_policies_remain_distinct() {
        let paths = ["/Other/p", "/Common/p"];
        assert_eq!(
            matches(
                "/Absent/p",
                &paths,
                ObjectNamePolicy::ExactThenReferenceLeaf
            ),
            paths
        );
        assert_eq!(
            matches("/Absent/p", &paths, ObjectNamePolicy::ExactThenShortName),
            [] as [&str; 0]
        );
        assert_eq!(
            matches("p", &paths, ObjectNamePolicy::ExactThenShortNameOrCommon),
            paths
        );
        assert_eq!(
            matches("p", &paths, ObjectNamePolicy::CommonThenSuffix),
            ["/Common/p"]
        );
    }

    #[test]
    fn literal_paths_are_not_tcl_or_filesystem_canonicalised() {
        let paths = [
            "/Tenant/a::b",
            "/Tenant/a//b",
            "/Tenant/a/../b",
            "/Tenant/b",
            "/Tenant/ap",
        ];
        let policy = ObjectNamePolicy::Configuration {
            default_partition: "Tenant",
        };
        for reference in ["a::b", "a//b", "a/../b"] {
            assert_eq!(
                matches(reference, &paths, policy),
                [format!("/Tenant/{reference}")]
            );
        }
        assert_eq!(matches("p", &paths, policy), [] as [&str; 0]);
        assert_eq!(matches("/Other/b", &paths, policy), [] as [&str; 0]);
    }

    #[test]
    fn attachment_partition_precedes_unique_leaf_fallback() {
        let paths = ["/Z/p", "/Tenant/p", "/Common/p"];
        let policy = ObjectNamePolicy::ExactThenOwnerPartitionOrReferenceLeaf {
            owner: "/Tenant/vs",
        };
        assert_eq!(matches("p", &paths, policy), ["/Tenant/p"]);
        assert_eq!(matches("/Z/p", &paths, policy), ["/Z/p"]);
        assert_eq!(
            matches(
                "p",
                &paths,
                ObjectNamePolicy::ExactThenOwnerPartitionOrReferenceLeaf {
                    owner: "/Absent/vs"
                }
            ),
            paths
        );
        assert_eq!(
            super::owner_partition_reference_paths("p", "/Tenant/vs", "Other"),
            ["/Tenant/p", "/Common/p"]
        );
        assert_eq!(
            super::owner_partition_reference_paths("/Z/p", "/Tenant/vs", "Other"),
            ["/Z/p"]
        );
        assert_eq!(
            super::owner_partition_reference_paths("p", "vs", ""),
            ["//p", "/Common/p"]
        );
        assert_eq!(
            matches("p", &paths, ObjectNamePolicy::Exact),
            [] as [&str; 0]
        );
    }

    #[test]
    fn service_suffix_policy_preserves_colon_domain_boundaries() {
        use super::{EmptyServiceSuffix, reference_without_numeric_service};
        assert_eq!(
            reference_without_numeric_service("/Tenant/n:80", EmptyServiceSuffix::Retain),
            "/Tenant/n"
        );
        assert_eq!(
            reference_without_numeric_service("/Tenant/n:", EmptyServiceSuffix::Retain),
            "/Tenant/n:"
        );
        assert_eq!(
            reference_without_numeric_service("/Tenant/n:", EmptyServiceSuffix::Strip),
            "/Tenant/n"
        );
        for reference in ["/Tenant/n:https", "/Tenant/2001:db8::1", "/Tenant/n::80"] {
            assert_eq!(
                reference_without_numeric_service(reference, EmptyServiceSuffix::Strip),
                reference
            );
        }
    }

    #[test]
    fn duplicate_exact_keys_preserve_order_without_unique_identity() {
        let entries = [
            ("/Tenant/p", "FIRST"),
            ("/Tenant/p", "SECOND"),
            ("/Other/p", "OTHER"),
        ];
        let select = |reference| {
            super::object_name_candidates(reference, entries, ObjectNamePolicy::ExactThenShortName)
        };
        assert_eq!(select("/Tenant/p").candidates(), ["FIRST", "SECOND"]);
        assert_eq!(select("/Tenant/p").unique(), None);
        assert_eq!(
            select("/Tenant/p").first_exact_in_order_or_unique(),
            Some("FIRST")
        );
        assert_eq!(select("p").first_exact_in_order_or_unique(), None);
        assert_eq!(
            super::object_name_candidates("p", entries, ObjectNamePolicy::ExactOrSuffix)
                .first_exact_in_order_or_unique(),
            None
        );
    }

    #[test]
    fn generic_suffix_policy_retains_source_order_even_before_exact() {
        let paths = ["/Tenant/p", "p"];
        assert_eq!(matches("p", &paths, ObjectNamePolicy::ExactOrSuffix), paths);
        assert_eq!(
            matches("/p", &paths, ObjectNamePolicy::ExactOrSuffix),
            ["/Tenant/p"]
        );
    }
}
