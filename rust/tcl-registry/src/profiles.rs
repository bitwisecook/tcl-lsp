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
//! Static data tables describing the 57 profile types, 87 protocol
//! command namespaces, and stack modification commands.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::lifecycle::Lifecycle;
use crate::relation::{
    Relation, RelationFactSource, RelationMode, RelationTermKind, TermHolds, closure_over,
};

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
/// instance of the one relation mechanism (R11).
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
    /// R11: one mechanism, not the two bare `requires` / `conflicts` slices
    /// this replaced. The direction is on the edge
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

    /// Whether `profile` is connection *infrastructure* — a transport
    /// (`TCP`/`UDP`/`FASTL4`/`SCTP`) or shared-TLS/persistence
    /// (`SSL_PERSISTENCE`/`PERSIST`) profile that the stack implies rather
    /// than the operator selecting. The `# Profiles:` header code action
    /// filters these out. Derives the former hardcoded `INFRA_PROFILES`
    /// list from each profile's registered `layer` (the `transport` and
    /// `tls_shared` layers).
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
    /// The `requires` slice this replaced, reconstructed for the consumers
    /// that publish it (the registry snapshot) rather than walk it.
    #[must_use]
    pub fn profile_parents(&self, profile: &str) -> Vec<&'static str> {
        self.get_profile(profile)
            .map(ProfileSpec::inferred_parents)
            .unwrap_or_default()
    }

    /// True when `active`'s expanded profile stack satisfies any one of the
    /// `required` profiles (OR semantics).
    #[must_use]
    pub fn stack_satisfies(&self, required: &[&str], active: &[&str]) -> bool {
        if required.is_empty() {
            return true;
        }
        let active_expanded = self.expand_profile_stack(active);
        required.iter().any(|candidate| {
            self.expand_profile_stack(std::slice::from_ref(candidate))
                .is_subset(&active_expanded)
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

// AUTO-GENERATED — do not edit manually

fn profile_specs() -> Vec<ProfileSpec> {
    let mut out = profile_specs_0();
    out.extend(profile_specs_1());
    out.extend(profile_specs_2());
    out.extend(profile_specs_3());
    out.extend(profile_specs_4());
    out.extend(profile_specs_5());
    out.extend(profile_specs_6());
    out
}

fn profile_specs_0() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "ACCESS",
            layer: "security",
            side: "client",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("ACCESS"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "AIMCP",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("AIMCP"), &[ProfileTerm("HTTP")]),
                ]
            },
            lifecycle: Lifecycle::introduced_in("21.1.0"),
        },
        ProfileSpec {
            name: "ANTIFRAUD",
            layer: "security",
            side: "client",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("ANTIFRAUD"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "ASM",
            layer: "security",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("ASM"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "AUTH",
            layer: "security",
            side: "client",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("AUTH"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "AVR",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("AVR"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "BOTDEFENSE",
            layer: "security",
            side: "client",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("BOTDEFENSE"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "CACHE",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("CACHE"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "CATEGORY",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("CATEGORY"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "CLASSIFICATION",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("CLASSIFICATION"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_1() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "CLIENTSSL",
            layer: "tls",
            side: "client",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("CLIENTSSL"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "CONNECTOR",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("CONNECTOR"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DATAGRAM",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("DATAGRAM"), &[ProfileTerm("UDP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DIAMETER",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("DIAMETER"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DIAMETERSESSION",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("DIAMETERSESSION"), &[ProfileTerm("DIAMETER")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DIAMETER_ENDPOINT",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("DIAMETER_ENDPOINT"), &[ProfileTerm("DIAMETER")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DNS",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DOSL7",
            layer: "security",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("DOSL7"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "ECA",
            layer: "security",
            side: "client",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("ECA"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_2() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "FASTHTTP",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("FASTHTTP"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "FASTL4",
            layer: "transport",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "FIX",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("FIX"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "FLOW",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "GENERICMSG",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("GENERICMSG"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "GTP",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "HTML",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("HTML"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "HTTP",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("HTTP"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "HTTP2",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("HTTP2"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "HTTP_PROXY_CONNECT",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("HTTP_PROXY_CONNECT"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_3() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "ICAP",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("ICAP"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "IPS",
            layer: "security",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("IPS"), &[ProfileTerm("PROTOCOL_INSPECTION")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "IVS_ENTRY",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("IVS_ENTRY"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "JSON",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("JSON"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "L7CHECK",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("L7CHECK"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "LSN",
            layer: "application",
            side: "client",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "MQTT",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("MQTT"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "MR",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("MR"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "MSSQL",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("MSSQL"), &[ProfileTerm("TDS")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "NAME",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("NAME"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_4() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "PCP",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "PEM",
            layer: "security",
            side: "client",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("PEM"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "PERSIST",
            // Side-independent (shared) TLS/persistence layer. Stack
            // infrastructure, not an operator-selected profile.
            layer: "tls_shared",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("PERSIST"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "PROTOCOL_INSPECTION",
            layer: "security",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("PROTOCOL_INSPECTION"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "QOE",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("QOE"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "RADIUS",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "RADIUS_AAA",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("RADIUS_AAA"), &[ProfileTerm("RADIUS")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "REQUESTADAPT",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("REQUESTADAPT"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "RESPONSEADAPT",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("RESPONSEADAPT"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "REWRITE",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("REWRITE"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_5() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "RTSP",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("RTSP"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SCTP",
            layer: "transport",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SERVERSSL",
            layer: "tls",
            side: "server",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("SERVERSSL"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SIP",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SIPROUTER",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("SIPROUTER"), &[ProfileTerm("SIP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SIPSESSION",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("SIPSESSION"), &[ProfileTerm("SIP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SOCKS",
            layer: "application",
            side: "client",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("SOCKS"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SSE",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("SSE"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SSL_PERSISTENCE",
            // Side-independent (shared) TLS layer. Stack
            // infrastructure, not an operator-selected profile.
            layer: "tls_shared",
            side: "client",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("SSL_PERSISTENCE"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_6() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "STREAM",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("STREAM"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "TAP",
            layer: "security",
            side: "client",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("TAP"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "TCP",
            layer: "transport",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "TDS",
            layer: "application",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("TDS"), &[ProfileTerm("TCP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "UDP",
            layer: "transport",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "WEBACCELERATION",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("WEBACCELERATION"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "WS",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("WS"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "XML",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[
                    ProfileRelation::implies(ProfileTerm("XML"), &[ProfileTerm("HTTP")]),
                ]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn protocol_namespace_specs() -> Vec<ProtocolNamespaceSpec> {
    let mut out = protocol_namespace_specs_0();
    out.extend(protocol_namespace_specs_1());
    out.extend(protocol_namespace_specs_2());
    out.extend(protocol_namespace_specs_3());
    out.extend(protocol_namespace_specs_4());
    out.extend(protocol_namespace_specs_5());
    out.extend(protocol_namespace_specs_6());
    out.extend(protocol_namespace_specs_7());
    out.extend(protocol_namespace_specs_8());
    out.extend(protocol_namespace_specs_9());
    out
}

fn protocol_namespace_specs_0() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "AAA",
            profiles: &[],
            layer: "security",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ACCESS",
            profiles: &["ACCESS"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ACCESS2",
            profiles: &["ACCESS"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ACL",
            profiles: &[],
            layer: "security",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ADAPT",
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AES",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AM",
            profiles: &[],
            layer: "acceleration",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ANTIFRAUD",
            profiles: &["ANTIFRAUD"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ASM",
            profiles: &["ASM"],
            layer: "security",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ASN1",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AUTH",
            profiles: &["AUTH"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AVR",
            profiles: &["AVR"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_1() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "BIGPROTO",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "BIGTCP",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "BOTDEFENSE",
            profiles: &["BOTDEFENSE"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "BWC",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CACHE",
            profiles: &["CACHE", "WEBACCELERATION"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CATEGORY",
            profiles: &["CATEGORY"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CLASSIFICATION",
            profiles: &["CLASSIFICATION"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CLASSIFY",
            profiles: &["FASTHTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "COMPRESS",
            profiles: &["FASTHTTP", "HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CONNECTOR",
            profiles: &["CONNECTOR"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CRYPTO",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DATAGRAM",
            profiles: &["DATAGRAM"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_2() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "DECOMPRESS",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DEMANGLE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DHCP",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DHCPv4",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DHCPv6",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DIAG",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DIAMETER",
            profiles: &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DNS",
            profiles: &["DNS"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DNSMSG",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DOSL7",
            profiles: &["DOSL7"],
            layer: "security",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DSLITE",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ECA",
            profiles: &["ECA"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_3() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "FIX",
            profiles: &["FIX"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "FLOW",
            profiles: &["FLOW"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "FLOWTABLE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "FTP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "GENERICMESSAGE",
            profiles: &["GENERICMSG"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "GTP",
            profiles: &["GTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HA",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HSL",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTML",
            profiles: &["HTML"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTTP",
            profiles: &["FASTHTTP", "HTTP", "HTTP_PROXY_CONNECT"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTTP2",
            profiles: &["HTTP2"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTTPLOG",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_4() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "ICAP",
            profiles: &["ICAP"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IKE",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ILX",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IMAP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IP",
            profiles: &[],
            layer: "transport",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "IPFIX",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ISESSION",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ISTATS",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IVS_ENTRY",
            profiles: &["IVS_ENTRY"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "JSON",
            profiles: &["JSON"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "L7CHECK",
            profiles: &["L7CHECK"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LB",
            profiles: &[],
            layer: "load_balance",
            side: "global",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_5() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "LDAP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LINE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LINK",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LSN",
            profiles: &["LSN"],
            layer: "application",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "MESSAGE",
            profiles: &["MR"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "MQTT",
            profiles: &["MQTT"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "MR",
            profiles: &["MR"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "NAME",
            profiles: &["NAME"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "NSH",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "NTLM",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "OFFBOX",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ONECONNECT",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_6() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "PCP",
            profiles: &["PCP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PEM",
            profiles: &["PEM"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PLUGIN",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "POLICY",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "POP3",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PROFILE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PROTOCOL_INSPECTION",
            profiles: &["IPS", "PROTOCOL_INSPECTION"],
            layer: "security",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PSC",
            profiles: &[],
            layer: "security",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PSM",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "QOE",
            profiles: &["QOE"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "RADIUS",
            profiles: &["RADIUS", "RADIUS_AAA"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "RESOLV",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_7() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "RESOLVER",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "REST",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "REWRITE",
            profiles: &["REWRITE"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ROUTE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "RTSP",
            profiles: &["RTSP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SCTP",
            profiles: &["SCTP"],
            layer: "transport",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SDP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SIP",
            profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SIPALG",
            profiles: &["MR", "SIP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SMTPS",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SOCKS",
            profiles: &["SOCKS"],
            layer: "application",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SSE",
            profiles: &["SSE"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_8() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "SSL",
            profiles: &["CLIENTSSL", "PERSIST", "SERVERSSL", "SSL_PERSISTENCE"],
            layer: "tls",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "STATS",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "STREAM",
            profiles: &["STREAM"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "TAP",
            profiles: &["TAP"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "TCP",
            profiles: &["TCP"],
            layer: "transport",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "TDS",
            profiles: &["MSSQL", "TDS"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "TMM",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "UDP",
            profiles: &["UDP"],
            layer: "transport",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "URI",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "VALIDATE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "VDI",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "WAM",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_9() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "WEBSSO",
            profiles: &["ACCESS", "HTTP"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "WS",
            profiles: &["WS"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "X509",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "XLAT",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "XML",
            profiles: &["XML"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
    ]
}

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
mod tests {
    use super::*;

    #[test]
    fn build_profile_registry() {
        let reg = ProfileRegistry::build();
        assert!(reg.profile_count() > 10);
        assert!(reg.namespace_count() > 10);
    }

    #[test]
    fn profile_lookup() {
        let reg = ProfileRegistry::build();
        let http = reg.get_profile("HTTP").unwrap();
        assert_eq!(http.layer, "application");
        assert!(http.inferred_parents().contains(&"TCP"));
    }

    #[test]
    fn aimcp_is_gated_to_bigip_21_1() {
        let reg = ProfileRegistry::build();
        let aimcp = reg.get_profile("AIMCP").expect("AIMCP profile registered");
        assert_eq!(aimcp.layer, "application");
        assert_eq!(aimcp.inferred_parents(), vec!["HTTP"]);
        assert!(!reg.profile_available_at("AIMCP", "21.0.0"));
        assert!(reg.profile_available_at("AIMCP", "21.1.0"));
    }

    #[test]
    fn namespace_lookup() {
        let reg = ProfileRegistry::build();
        let http_ns = reg.get_namespace("HTTP").unwrap();
        assert!(http_ns.profiles.contains(&"HTTP"));
        assert_eq!(http_ns.layer, "application");
    }

    #[test]
    fn parse_profile_directive_comma_and_space() {
        let got = parse_profile_directive("# profiles: HTTP, clientssl serverssl\nset x 1\n");
        let mut v: Vec<&str> = got.iter().map(String::as_str).collect();
        v.sort_unstable();
        assert_eq!(v, ["CLIENTSSL", "HTTP", "SERVERSSL"]);
    }

    #[test]
    fn parse_profile_directive_singular_and_stops_at_code() {
        // Singular `profile:` is accepted; scanning stops at the first
        // non-comment line, so a later directive is ignored.
        let got = parse_profile_directive("# profile : TCP\nset x 1\n# profiles: HTTP\n");
        let v: Vec<&str> = got.iter().map(String::as_str).collect();
        assert_eq!(v, ["TCP"]);
    }

    #[test]
    fn parse_profile_directive_rejects_non_directive_comments() {
        assert!(parse_profile_directive("# just a comment\nwhen HTTP_REQUEST {}\n").is_empty());
    }

    #[test]
    fn scan_file_events_finds_uppercase_events() {
        let evs = scan_file_events("when HTTP_REQUEST {\n}\nwhen CLIENT_ACCEPTED { }\n");
        let mut v: Vec<&str> = evs.iter().map(String::as_str).collect();
        v.sort_unstable();
        assert_eq!(v, ["CLIENT_ACCEPTED", "HTTP_REQUEST"]);
    }

    #[test]
    fn scan_file_events_respects_command_identity_and_normalises_case() {
        assert!(scan_file_events("awhen HTTP_REQUEST {}").is_empty());
        assert_eq!(
            scan_file_events("::when http_request { if {1} { :::when client_data {} } }"),
            FxHashSet::from_iter(["HTTP_REQUEST".to_owned()])
        );
    }

    #[test]
    fn compute_file_profiles_unions_directive_and_inferred() {
        let events = crate::events::EventRegistry::build();
        let profiles = ProfileRegistry::build();
        // CLIENTSSL_HANDSHAKE infers CLIENTSSL; the directive adds HTTP.  Both
        // expand transitively to include TCP (the parent of each).
        let got = compute_file_profiles(
            "# profiles: HTTP\nwhen CLIENTSSL_HANDSHAKE { }\n",
            &events,
            &profiles,
        );
        assert!(got.contains(&"HTTP".to_string()));
        assert!(got.contains(&"CLIENTSSL".to_string()));
        assert!(
            got.contains(&"TCP".to_string()),
            "transitive parent: {got:?}"
        );
        // Sorted output.
        assert!(got.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn ssl_namespace_side_selectable() {
        let reg = ProfileRegistry::build();
        let ssl = reg.get_namespace("SSL").unwrap();
        assert!(ssl.side_selectable);
    }

    #[test]
    fn modification_specs_exist() {
        let reg = ProfileRegistry::build();
        assert_eq!(reg.modifications().len(), 4);
    }
}
