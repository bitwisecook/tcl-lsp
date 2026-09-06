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

//! Which settings only make sense read together.
//!
//! A `CommandSpec` field is rarely a standalone switch. `arity` is read
//! against `arity_windows`; a `taint_source` colour means nothing without the
//! sinks that check it; setting `pure` while `side_effects` says otherwise is
//! a contradiction the studio should let an author *see* rather than discover
//! from a failing gate. The studio's help surfaces link a field to the rest of
//! its cluster so following the interaction is one click.
//!
//! Clusters, not pairs. A pair table has to state both directions and gets one
//! of them wrong the first time a member is added; membership of a named group
//! is symmetric by construction, and the group's name is the sentence the UI
//! wants anyway ("Taint sinks — 8 settings").
//!
//! A field may belong to several clusters, and [`STANDALONE`] names the ones
//! that belong to none, each with the reason — so a new field is filed by a
//! decision rather than by omission ([`tests`] fails until it is).

/// One named group of settings that interact, in the order the studio lists
/// them.
pub const CLUSTERS: &[Cluster] = &[
    Cluster {
        name: "Argument roles",
        why: "What each argument word *is* — the roles, the resolver that \
              overrides them, and the layouts that repeat them.",
        members: &[
            "arg_roles",
            "arg_role_resolver",
            "arg_role_resolver_roles",
            "repeated_args",
            "arg_presentation",
            "assigns_variable_at",
            "arg_types",
        ],
    },
    Cluster {
        name: "Arity",
        why: "How many words the command takes, and how that count changed \
              across releases.",
        members: &[
            "arity",
            "arity_windows",
            "reserved_trailing_words",
            "max_leading_option_words",
        ],
    },
    Cluster {
        name: "Result typing",
        why: "What the call produces, and what it does to the representation \
              of what it touches.",
        members: &[
            "return_type",
            "return_type_hook",
            "return_elements",
            "var_write_typing",
            "variable_write_min_args",
            "var_elements_effect",
            "representation_effect",
            "inferred_storage_type",
            "returns_path",
        ],
    },
    Cluster {
        name: "Subcommands",
        why: "The ensemble's own words: which exist, how a prefix resolves to \
              one, and what an unlisted one means.",
        members: &[
            "subcommands",
            "sub_subcommands",
            "allow_unknown_subcommands",
            "prefix_matching",
            "min_abbrev",
            "default_form_first_word",
        ],
    },
    Cluster {
        name: "Options",
        why: "The option table and every rule about how its rows may be \
              combined, placed, and abbreviated.",
        members: &[
            "options",
            "option_relations",
            "option_placement",
            "constraints",
            "setter_constraints",
            "literal_argument_validator",
        ],
    },
    Cluster {
        name: "Closed value sets",
        why: "Argument positions that accept only a listed vocabulary, and how \
              that vocabulary changed across releases.",
        members: &[
            "arg_values",
            "versioned_arg_values",
            "arg_values_accept_prefix",
            "closed_value_args",
        ],
    },
    Cluster {
        name: "Callbacks",
        why: "Words invoked as a command prefix later: which they are, when \
              they run, and what they are handed.",
        members: &[
            "command_prefixes",
            "command_prefix_resolver",
            "script_timing",
            "script_timing_resolver",
            "callback_taint_inputs",
        ],
    },
    Cluster {
        name: "Taint sources",
        why: "Where untrusted data enters, and what colour it carries onward.",
        members: &[
            "taint_source",
            "taint_transform",
            "taint_double_encode_colour",
            "taints_var_write",
            "is_unescape",
        ],
    },
    Cluster {
        name: "Taint sinks",
        why: "Where tainted data must not arrive, and what makes an arrival \
              safe.",
        members: &[
            "taint_output_sink",
            "taint_output_sink_subcommands",
            "taint_log_sink",
            "taint_network_sink_args",
            "taint_code_sink_args",
            "taint_interp_eval_subcommands",
            "taint_sink_safe_colour",
            "taint_sink_gate",
        ],
    },
    Cluster {
        name: "Secrets",
        why: "Argument and option positions that carry credentials, and the \
              headers that must not be logged.",
        members: &["credential_options", "credential_arg", "sensitive_headers"],
    },
    Cluster {
        name: "Effects and purity",
        why: "What the call changes in the world — and therefore whether the \
              optimiser may move, reuse, or fold it away.",
        members: &[
            "traits",
            "pure",
            "mutator",
            "destructive",
            "side_effects",
            "world_effects",
            "state_transitions",
            "result_stability",
            "const_fold",
            "const_fold_versioned",
        ],
    },
    Cluster {
        name: "Availability",
        why: "Which language, package, and release actually offer the command.",
        members: &[
            "surface",
            "required_package",
            "tcllib_package",
            "warn_missing_import",
            "is_namespace_exported",
        ],
    },
    Cluster {
        name: "Lifecycle",
        why: "When the command arrived, when it was deprecated, when it went \
              away, and what to write instead.",
        members: &[
            "introduced_version",
            "deprecated_version",
            "retired_version",
            "deprecation_fix",
            "deprecated_replacement",
            "deprecated_replacement_drop_in",
            "xc_translatable",
        ],
    },
    Cluster {
        name: "Bodies and frames",
        why: "Script arguments: whose frame they run in, what names they see, \
              and whether they run at all.",
        members: &[
            "body_kind",
            "body_interpreter",
            "body_arg_implicit_args",
            "body_scope",
            "frame_effect",
            "creates_scope_alias",
            "variable_scope",
            "loop_list_header",
        ],
    },
    Cluster {
        name: "Definitions and classes",
        why: "Commands that define other commands: the grammar of the body, \
              the members it may declare, and what the result is called.",
        members: &[
            "definition_body",
            "manufacturer_methods",
            "object_class",
            "oo_context_facts",
            "self_receiver_words",
            "method_prefix_matching",
            "creates_instance_at",
            "defines_command_at",
            "defines_symbol",
            "implementation_namespace",
        ],
    },
    Cluster {
        name: "Compiler hooks",
        why: "The named engine code a spec dispatches to, at each stage of the \
              pipeline.",
        members: &[
            "semantic_operation",
            "lowering_hook",
            "codegen_hook",
            "inline_codegen_hook",
            "native_lowering",
            "analyser_hook",
            "bpf_op",
            "cfg_rewrite_name",
        ],
    },
    Cluster {
        name: "Documentation",
        why: "What an author reads on hover, and the call shapes that back it.",
        members: &[
            "hover",
            "synopsis",
            "detail",
            "forms",
            "command_forms",
            "subcommand_forms",
        ],
    },
    Cluster {
        name: "Clause grammars",
        why: "Commands whose arguments are pattern/body clauses rather than a \
              fixed list.",
        members: &[
            "case_list",
            "clause_shape_check",
            "pattern_type",
            "pattern_arg_resolver",
            "format_string_type",
        ],
    },
    Cluster {
        name: "iRules events",
        why: "What an event handler requires of the connection, and what it \
              does to the rest of the rule.",
        members: &[
            "event_requires",
            "event_requirement_forms",
            "data_collection",
            "side_switch_target",
            "event_handler_priority",
            "irules_top_level_effect",
            "excluded_events",
        ],
    },
    Cluster {
        name: "Binary payloads",
        why: "Commands that read or produce byte arrays rather than strings.",
        members: &["byte_array_payload", "byte_array_effect"],
    },
    Cluster {
        name: "Handles and context",
        why: "Values that name a live resource, and where a command using one \
              is legal.",
        members: &["binds_handle", "remote_method", "context_gate"],
    },
    Cluster {
        name: "Safety",
        why: "What a safe interpreter hides, and what survives an uninitialised \
              target.",
        members: &["unsafe_command", "safe_on_uninit"],
    },
    Cluster {
        name: "Command table",
        why: "Commands that add, move, or remove entries in the interpreter's \
              own command table.",
        members: &["command_table_effect", "dispatch_dependencies"],
    },
    Cluster {
        name: "Tk geometry",
        why: "How a geometry manager claims a container, reconfigures a slave, \
              and lets it go.",
        members: &[
            "tk_geometry",
            "container_policy",
            "container_option",
            "direct_form",
            "placement_subcommand",
            "release_subcommands",
        ],
    },
    Cluster {
        name: "Completion",
        why: "How the call finishes, and what the editor offers after it.",
        members: &["completion"],
    },
];

/// Fields that interact with nothing else, and why.
///
/// Listed rather than omitted: a field with no cluster is a decision, and the
/// coverage test below fails by name until one is made.
pub const STANDALONE: &[(&str, &str)] = &[(
    "name",
    "the command word itself — every other field is about it",
)];

/// A named group of settings that are read together.
#[derive(Debug, Clone, Copy)]
pub struct Cluster {
    /// Heading the studio shows above the links.
    pub name: &'static str,
    /// One sentence saying what the group is about.
    pub why: &'static str,
    /// Field keys in the group, in reading order.
    pub members: &'static [&'static str],
}

/// The clusters `key` belongs to, in declaration order.
#[must_use]
pub fn clusters_for(key: &str) -> Vec<&'static Cluster> {
    CLUSTERS
        .iter()
        .filter(|cluster| cluster.members.contains(&key))
        .collect()
}

/// The other settings `key` is read with, grouped by cluster.
///
/// The shape the studio's help dock renders: a heading, why the group hangs
/// together, and the sibling keys to link to.
#[must_use]
pub fn related(key: &str) -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    clusters_for(key)
        .into_iter()
        .map(|cluster| {
            let siblings = cluster
                .members
                .iter()
                .copied()
                .filter(|member| *member != key)
                .collect();
            (cluster.name, cluster.why, siblings)
        })
        .collect()
}

/// `related` as the JSON the schema carries to the browser.
#[must_use]
pub fn to_json(key: &str) -> serde_json::Value {
    serde_json::Value::Array(
        related(key)
            .into_iter()
            .map(|(name, why, siblings)| {
                serde_json::json!({ "name": name, "why": why, "keys": siblings })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema;

    fn schema_keys() -> Vec<&'static str> {
        let mut keys: Vec<&'static str> = schema::COMMAND_FIELDS
            .iter()
            .map(|field| field.key)
            .chain(schema::SUBCOMMAND_FIELDS.iter().map(|field| field.key))
            .chain(schema::NESTED_FIELDS.iter().map(|field| field.key))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        keys
    }

    /// The gate: a new field is filed by a decision, not by omission.
    #[test]
    fn every_field_is_clustered_or_declared_standalone() {
        let mut unfiled = Vec::new();
        for key in schema_keys() {
            let clustered = !clusters_for(key).is_empty();
            let standalone = STANDALONE.iter().any(|(name, _)| *name == key);
            if !clustered && !standalone {
                unfiled.push(key);
            }
            assert!(
                !(clustered && standalone),
                "{key} is both clustered and declared standalone"
            );
        }
        assert!(
            unfiled.is_empty(),
            "these fields are in no cluster and are not declared standalone — \
             add each to a CLUSTERS row, or to STANDALONE with the reason: {unfiled:?}"
        );
    }

    /// A link that goes nowhere is worse than no link.
    #[test]
    fn every_named_key_is_a_live_field() {
        let keys = schema_keys();
        for cluster in CLUSTERS {
            for member in cluster.members {
                assert!(
                    keys.contains(member),
                    "cluster {} names {member}, which is not a schema field",
                    cluster.name
                );
            }
        }
        for (key, _) in STANDALONE {
            assert!(
                keys.contains(key),
                "STANDALONE names {key}, which is not a schema field"
            );
        }
    }

    #[test]
    fn clusters_are_named_once_and_say_what_they_are_for() {
        let mut seen: Vec<&str> = Vec::new();
        for cluster in CLUSTERS {
            assert!(
                !seen.contains(&cluster.name),
                "{} is declared twice",
                cluster.name
            );
            assert!(
                cluster.why.len() > 30,
                "{}'s reason is too short to explain the grouping",
                cluster.name
            );
            let mut members = cluster.members.to_vec();
            members.sort_unstable();
            let before = members.len();
            members.dedup();
            assert_eq!(
                before,
                members.len(),
                "{} lists a member twice",
                cluster.name
            );
            seen.push(cluster.name);
        }
    }

    /// Following a link and coming back must land where you started.
    #[test]
    fn membership_is_symmetric() {
        for cluster in CLUSTERS {
            for member in cluster.members {
                let back: Vec<&str> = related(member)
                    .into_iter()
                    .filter(|(name, _, _)| *name == cluster.name)
                    .flat_map(|(_, _, siblings)| siblings)
                    .collect();
                for other in cluster.members.iter().filter(|other| *other != member) {
                    assert!(
                        back.contains(other),
                        "{member} does not link back to {other} in {}",
                        cluster.name
                    );
                }
            }
        }
    }
}
