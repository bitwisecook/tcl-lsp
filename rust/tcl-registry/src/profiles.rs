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

//! F5 profile and protocol namespace metadata.
//!
//! Static data tables describing the 66 profile types, 113 protocol
//! command namespaces, and stack modification commands.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::lifecycle::Lifecycle;
use crate::relation::{
    Relation, RelationFactSource, RelationMode, RelationTermKind, TermHolds, closure_over,
};

mod profile_table;
mod protocol_namespace_table;

use profile_table::profile_specs;
use protocol_namespace_table::protocol_namespace_specs;

/// One operand of a [`ProfileRelation`] — a `BIG-IP` profile *type* name.
///
/// Always the registry's own spelling, which is upper case; [`ProfileFacts`]
/// normalises the configuration side to match, so the comparison here is a
/// plain string equality rather than a case fold per term per relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProfileTerm(pub &'static str);

impl RelationTermKind for ProfileTerm {
    fn collective_noun() -> &'static str {
        "Profiles"
    }

    fn describe(self) -> String {
        self.0.to_owned()
    }

    fn spelling(self) -> String {
        self.0.to_owned()
    }
}

/// A declared relation between `BIG-IP` profile types — the profile domain's
/// instance of the one relation mechanism (R12).
pub type ProfileRelation = Relation<ProfileTerm>;

/// One virtual server's attached profile stack, as the relation checker reads
/// it.
///
/// `complete` is `true` for a stack read from a parsed configuration: unlike an
/// invocation, which a `{*}` expansion can hide words from, a virtual server
/// names every profile it attaches, so absence is provable and a `Requires`
/// edge over one never has to abstain.
#[derive(Debug, Clone, Copy)]
pub struct ProfileFacts<'a> {
    /// The active profile type names, upper-cased, parents already inferred.
    pub active: &'a [String],
    /// Whether the stack was read in full. Only this licenses proving a
    /// profile *absent*.
    pub complete: bool,
}

impl RelationFactSource<ProfileTerm> for ProfileFacts<'_> {
    fn holds(&self, term: ProfileTerm) -> TermHolds {
        if self.active.iter().any(|name| name == term.0) {
            TermHolds::Yes
        } else if self.complete {
            TermHolds::No
        } else {
            TermHolds::Unknown
        }
    }
}

/// The relation list of a profile the registry does not know — no edges, so a
/// closure over it terminates immediately.
const NO_PROFILE_RELATIONS: &[ProfileRelation] = &[];

/// Metadata for an F5 profile type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSpec {
    /// Profile type name (e.g. `"HTTP"`, `"CLIENTSSL"`, `"DNS"`).
    pub name: &'static str,
    /// Protocol stack layer.
    pub layer: &'static str,
    /// Connection side: `"client"`, `"server"`, `"both"`, `"global"`.
    pub side: &'static str,
    /// This profile's relations to other profile types — its inference
    /// edges to the parents `BIG-IP` attaches for it, and its assertion edges
    /// to the profiles it cannot be stacked with.
    ///
    /// R12: one mechanism for both inference and conflict edges. The
    /// direction is on the edge
    /// ([`RelationMode`](crate::relation::RelationMode)) because the two read
    /// opposite ways: `HTTP` ⇒ `TCP` adds the parent, `FASTL4` ⊥ `HTTP`
    /// reports a defect.
    pub relations: &'static [ProfileRelation],
    /// Introduction / deprecation / retirement releases of this profile type
    /// on the `BIG-IP` release axis. An absent introducing release inherits
    /// the axis baseline (BIG-IP 15.0).
    pub lifecycle: Lifecycle,
}

impl ProfileSpec {
    /// Base for the table literals: no explicit version knowledge, so
    /// both bounds inherit the axis baseline. Identity fields are
    /// placeholders — every literal overrides them.
    const DEFAULT: Self = Self {
        name: "",
        layer: "",
        side: "",
        relations: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    };

    /// The profile type's lifecycle with the `BIG-IP` axis baseline filled in
    /// for an absent introducing release.
    #[must_use]
    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle
            .with_baseline(tcl_dialect::VersionKey::BigipVersion.baseline_version())
    }

    /// The profile types this one implies — the terms of its inference edges.
    #[must_use]
    pub fn inferred_parents(&self) -> Vec<&'static str> {
        self.relation_terms(RelationMode::Infer)
    }

    /// The profile types this one cannot be stacked with — the terms of its
    /// assertion edges.
    #[must_use]
    pub fn forbidden_peers(&self) -> Vec<&'static str> {
        self.relation_terms(RelationMode::Assert)
    }

    /// The named terms of every relation in `mode`, the subject excluded: the
    /// subject of a row-owned edge is the row itself.
    fn relation_terms(&self, mode: RelationMode) -> Vec<&'static str> {
        self.relations
            .iter()
            .filter(|relation| relation.mode == mode)
            .flat_map(|relation| relation.terms.iter().map(|term| term.0))
            .collect()
    }
}

/// iRules protocol command namespace availability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolNamespaceSpec {
    /// Namespace prefix (e.g. `"HTTP"`, `"SSL"`, `"TCP"`).
    pub prefix: &'static str,
    /// Profiles that provide this namespace.
    pub profiles: &'static [&'static str],
    /// Protocol layer.
    pub layer: &'static str,
    /// Default connection side.
    pub side: &'static str,
    /// Whether `clientside`/`serverside` qualifiers are supported.
    pub side_selectable: bool,
}

/// A command that changes the active profile stack at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackModification {
    /// Command name (e.g. `"SSL::disable"`).
    pub command: &'static str,
    /// Connection side affected.
    pub side: Option<&'static str>,
    /// Profile removed by this command.
    pub removes_profile: Option<&'static str>,
    /// Profile added by this command.
    pub adds_profile: Option<&'static str>,
}

/// Profile registry providing lookup over static profile tables.
pub struct ProfileRegistry {
    profiles: FxHashMap<&'static str, ProfileSpec>,
    namespaces: FxHashMap<&'static str, ProtocolNamespaceSpec>,
    modifications: Vec<StackModification>,
}

impl ProfileRegistry {
    /// The lifecycle of profile type `name` on the BIG-IP release axis
    /// (explicit data, absent introducing release inheriting the BIG-IP 15.0
    /// baseline). `None` for an unknown profile type.
    #[must_use]
    pub fn profile_lifecycle(&self, name: &str) -> Option<Lifecycle> {
        Some(self.get_profile(name)?.lifecycle())
    }

    /// Whether profile type `name` exists at BIG-IP `version` per the
    /// declared data (baseline semantics).
    #[must_use]
    pub fn profile_available_at(&self, name: &str, version: &str) -> bool {
        self.profile_lifecycle(name)
            .is_some_and(|life| life.available_at(Some(version)))
    }

    /// Build the profile registry from static data.
    #[must_use]
    pub fn build() -> Self {
        let mut profiles = FxHashMap::default();
        for spec in profile_specs() {
            profiles.insert(spec.name, spec);
        }
        let mut namespaces = FxHashMap::default();
        for spec in protocol_namespace_specs() {
            namespaces.insert(spec.prefix, spec);
        }
        Self {
            profiles,
            namespaces,
            modifications: modification_specs(),
        }
    }

    /// Look up a profile spec by name.
    #[must_use]
    pub fn get_profile(&self, name: &str) -> Option<&ProfileSpec> {
        self.profiles.get(name)
    }

    /// Look up a protocol namespace by prefix.
    #[must_use]
    pub fn get_namespace(&self, prefix: &str) -> Option<&ProtocolNamespaceSpec> {
        self.namespaces.get(prefix)
    }

    /// Protocol namespace metadata for an already selected iRules command row.
    /// The caller retains its actual context/argument-form admission. A reported
    /// or source name cannot substitute for this row, and this projection grants
    /// no Native command, namespace or event ownership.
    #[must_use]
    pub fn namespace_for_command_spec(
        &self,
        spec: &crate::CommandSpec,
    ) -> Option<&ProtocolNamespaceSpec> {
        use tcl_dialect::model::{Family, SpecProvider};
        spec.surface?
            .iter()
            .any(|row| row.provider == SpecProvider::Core(Family::F5Irules))
            .then_some(())?;
        let (holder, _) = tcl_syntax::naming::key_holder_and_tail(spec.name);
        let namespace = tcl_syntax::naming::unroot_rooted_key(holder).unwrap_or(holder);
        self.get_namespace(namespace)
    }

    /// Whether `profile` is connection *infrastructure* — a transport
    /// (`TCP`/`UDP`/`FASTL4`/`SCTP`) or shared-TLS/persistence
    /// (`SSL_PERSISTENCE`/`PERSIST`) profile that the stack implies rather
    /// than the operator selecting. The `# Profiles:` header code action
    /// filters these out. Derived from each profile's registered `layer`
    /// (the `transport` and `tls_shared` layers), so the list cannot drift
    /// from the profile data itself.
    #[must_use]
    pub fn is_infrastructure_profile(&self, profile: &str) -> bool {
        self.get_profile(profile)
            .is_some_and(|p| matches!(p.layer, "transport" | "tls_shared"))
    }

    /// All registered profile names.
    #[must_use]
    pub fn all_profile_names(&self) -> Vec<&str> {
        self.profiles.keys().copied().collect()
    }

    /// All registered namespace prefixes.
    #[must_use]
    pub fn all_namespace_prefixes(&self) -> Vec<&str> {
        self.namespaces.keys().copied().collect()
    }

    /// Stack modification commands.
    #[must_use]
    pub fn modifications(&self) -> &[StackModification] {
        &self.modifications
    }

    /// Number of registered profiles.
    #[must_use]
    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    /// Number of registered namespaces.
    #[must_use]
    pub fn namespace_count(&self) -> usize {
        self.namespaces.len()
    }

    /// `profiles` plus every profile reachable from them by an inference
    /// edge, uppercased.
    ///
    /// The transitive walk is [`closure_over`], shared with every other
    /// relation domain; the only thing this adds is the case fold and the
    /// pass-through of names the registry does not know (which carry no edges
    /// but must still appear in the expanded stack, because a configuration
    /// naming a profile the registry has not caught up with has still attached
    /// it).
    #[must_use]
    pub fn expand_profile_stack(&self, profiles: &[&str]) -> FxHashSet<String> {
        let mut expanded: FxHashSet<String> = profiles.iter().map(|p| p.to_uppercase()).collect();
        let seeds: Vec<ProfileTerm> = expanded
            .iter()
            .filter_map(|name| self.get_profile(name).map(|spec| ProfileTerm(spec.name)))
            .collect();
        for term in closure_over(seeds, |term| {
            self.get_profile(term.0)
                .map_or(NO_PROFILE_RELATIONS, |spec| spec.relations)
        }) {
            expanded.insert(term.0.to_uppercase());
        }
        expanded
    }

    /// Every profile this one names as a parent — the inference edges of
    /// [`ProfileSpec::relations`], flattened.
    ///
    /// Reconstructed for the consumers that publish a flat parents list
    /// (the registry snapshot) rather than walk the relations themselves.
    #[must_use]
    pub fn profile_parents(&self, profile: &str) -> Vec<&'static str> {
        self.get_profile(profile)
            .map_or_default(ProfileSpec::inferred_parents)
    }

    /// True when `active`'s expanded profile stack satisfies any one of the
    /// `required` profiles (OR semantics) — a `RequiresOneOf` over the
    /// stack's facts, read through the same [`ProfileFacts`] the conflict
    /// checker uses.
    ///
    /// The candidate's *own* expansion does not have to be computed. The
    /// active expansion is a fixed point over the inference edges, so it is
    /// already closed under them: if a candidate is in it, everything the
    /// candidate implies is in it too, and `expand(candidate) ⊆ expanded`
    /// reduces to `candidate ∈ expanded`. That is one closure per call rather
    /// than one per call plus one per candidate, on a path the hover and
    /// completion surfaces take
    /// (`CommandRegistry::valid_irules_commands_for_event`);
    /// `stack_satisfies_agrees_with_the_subset_formulation` holds the
    /// equivalence over the whole shipped table.
    #[must_use]
    pub fn stack_satisfies(&self, required: &[&str], active: &[&str]) -> bool {
        if required.is_empty() {
            return true;
        }
        let expanded: Vec<String> = self.expand_profile_stack(active).into_iter().collect();
        let facts = ProfileFacts {
            active: &expanded,
            complete: true,
        };
        required.iter().any(|candidate| {
            self.get_profile(candidate).map_or_else(
                // A profile the registry does not know carries no edges, so
                // it can only be satisfied by appearing in the stack verbatim.
                || expanded.contains(&candidate.to_uppercase()),
                |spec| facts.holds(ProfileTerm(spec.name)) == TermHolds::Yes,
            )
        })
    }

    /// Canonical protocol-stack rank for a profile *type* (e.g. `"HTTP"`),
    /// lowest = closest to the wire.  Looks the type up in the registry and
    /// ranks it by its [`ProfileSpec::layer`] via [`layer_rank`].  Unknown
    /// types rank last so nothing is dropped when ordering.
    ///
    /// Use this to order a virtual server's profile stack (transport → TLS →
    /// application → …) instead of alphabetically, which would list `HTTP`
    /// ahead of `TCP` and invert the real processing order.
    #[must_use]
    pub fn layer_rank(&self, profile_type: &str) -> u8 {
        layer_rank(self.get_profile(profile_type).map_or("", |p| p.layer))
    }
}

/// Rank of a protocol-stack [`ProfileSpec::layer`] name — lowest is nearest
/// the wire.  A BIG-IP virtual processes its profile stack bottom-up:
/// transport → TLS → application, with the security / acceleration / utility
/// facets layered on top.  Unknown layers rank last so no profile is dropped.
#[must_use]
pub fn layer_rank(layer: &str) -> u8 {
    match layer {
        "transport" => 0,
        "tls_shared" => 1,
        "tls" => 2,
        "application" => 3,
        "load_balance" => 4,
        "security" => 5,
        "acceleration" => 6,
        "utility" => 7,
        _ => 8,
    }
}

/// Profiles attached to a file.
///
/// Combines an explicit `# profiles: …` directive (scanned from the leading
/// comment block) with the profiles implied by every `when EVENT` handler
/// present, then expands the transitive profile stack.  Returns the sorted,
/// uppercased, fully-expanded profile set.
#[must_use]
pub fn compute_file_profiles(
    source: &str,
    events: &crate::events::EventRegistry,
    profiles: &ProfileRegistry,
) -> Vec<String> {
    let mut seed: FxHashSet<String> = parse_profile_directive(source);
    for event in scan_file_events(source) {
        if let Some(props) = events.get_props(&event) {
            seed.extend(props.implied_profiles.iter().map(|p| p.to_uppercase()));
        }
    }
    let seed_refs: Vec<&str> = seed.iter().map(String::as_str).collect();
    let mut expanded: Vec<String> = profiles
        .expand_profile_stack(&seed_refs)
        .into_iter()
        .collect();
    expanded.sort_unstable();
    expanded
}

/// [`compute_file_profiles`] with resolved command heads.
///
/// The profile inventory is an event-handler consumer, so it must make the
/// same offset-sensitive alias/rename decision as diagnostics, outlines, and
/// graphs. The resolver is supplied by the compiler/LSP boundary; this crate
/// deliberately does not infer command-table mutations itself.
#[must_use]
pub fn compute_file_profiles_with_registry_and_head_resolver(
    source: &str,
    events: &crate::events::EventRegistry,
    profiles: &ProfileRegistry,
    registry: &crate::CommandRegistry,
    resolver: &dyn crate::events::CommandHeadResolver,
) -> Vec<String> {
    let mut seed: FxHashSet<String> = parse_profile_directive(source);
    for event in scan_file_events_with_registry_and_head_resolver(source, registry, resolver) {
        if let Some(props) = events.get_props(&event) {
            seed.extend(props.implied_profiles.iter().map(|p| p.to_uppercase()));
        }
    }
    let seed_refs: Vec<&str> = seed.iter().map(String::as_str).collect();
    let mut expanded: Vec<String> = profiles
        .expand_profile_stack(&seed_refs)
        .into_iter()
        .collect();
    expanded.sort_unstable();
    expanded
}

/// Parse a leading `# profiles: HTTP, CLIENTSSL` directive.  Scans at most
/// the first 20 lines and stops at
/// the first non-comment, non-blank line.  Names are uppercased and split on
/// commas/whitespace.
#[must_use]
pub fn parse_profile_directive(source: &str) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for line in source.split('\n').take(20) {
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        if let Some(payload) = profile_directive_payload(stripped) {
            for token in payload.split(|c: char| c == ',' || c.is_whitespace()) {
                if !token.is_empty() {
                    out.insert(token.to_uppercase());
                }
            }
        }
    }
    out
}

/// Match `# profiles? :` (case-insensitive) at the head of an already-trimmed
/// comment line and return the payload after the colon.
fn profile_directive_payload(stripped_line: &str) -> Option<&str> {
    let after_hash = stripped_line.strip_prefix('#')?.trim_start();
    let lower = after_hash.to_ascii_lowercase();
    // `profiles?` — the optional trailing `s`; longest match first.
    let kw_len = if lower.starts_with("profiles") {
        "profiles".len()
    } else if lower.starts_with("profile") {
        "profile".len()
    } else {
        return None;
    };
    // The keyword is ASCII, so the byte length is the same on `after_hash`.
    let payload = after_hash[kw_len..].trim_start().strip_prefix(':')?.trim();
    (!payload.is_empty()).then_some(payload)
}

/// Event names from every live handler, through the shared syntax owner.
#[must_use]
pub fn scan_file_events(source: &str) -> FxHashSet<String> {
    crate::events::scan_when_events(source)
        .into_iter()
        .collect()
}

/// [`scan_file_events`] with resolved command heads.
#[must_use]
pub fn scan_file_events_with_registry_and_head_resolver(
    source: &str,
    registry: &crate::CommandRegistry,
    resolver: &dyn crate::events::CommandHeadResolver,
) -> FxHashSet<String> {
    crate::events::top_level_when_handlers_with_registry_and_head_resolver(
        source, registry, resolver,
    )
    .into_iter()
    .map(|handler| handler.event)
    .collect()
}

// Full static data — auto-generated.

fn modification_specs() -> Vec<StackModification> {
    vec![
        StackModification {
            command: "SSL::disable",
            side: None,
            removes_profile: None,
            adds_profile: None,
        },
        StackModification {
            command: "SSL::enable",
            side: None,
            removes_profile: None,
            adds_profile: None,
        },
        StackModification {
            command: "HTTP::disable",
            side: None,
            removes_profile: Some("HTTP"),
            adds_profile: None,
        },
        StackModification {
            command: "HTTP::enable",
            side: None,
            removes_profile: None,
            adds_profile: Some("HTTP"),
        },
    ]
}

#[cfg(test)]
mod tests;
