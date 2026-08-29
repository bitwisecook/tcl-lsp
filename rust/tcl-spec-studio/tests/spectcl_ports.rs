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

//! The equivalence gate for the eleven `.tclspec` ports.
//!
//! Every file in `docs/design/spec-dsl-examples/` names the `.rs` it was
//! ported from. This test loads each pack through [`spectcl::evaluate_pack`],
//! seeds a draft from the result with the ordinary
//! [`draft::from_command_spec`](tcl_spec_studio::draft::from_command_spec),
//! and compares it **field by field** against the draft
//! [`tcl_spec_studio::load_command`] seeds from the shipped spec — the same
//! comparison the studio's own round-trip makes.
//!
//! ## What "unequal" is allowed to mean
//!
//! A field may differ only when the port itself says so, and every such field
//! is listed in [`PORTS`] below with its reason. There are exactly three kinds:
//!
//! 1. **Native hooks the DSL replaces with a Tcl body or a derivation.** The
//!    draft model records a function-pointer field as "the defining Rust
//!    expression could not be recovered", so a *set* field compares equal
//!    whatever set it — but a field the pack **removes** (`oo::class`'s
//!    `arg_role_resolver`, derived `from-manufacturers`; `if`'s two hooks,
//!    derived from `clause_grammar`) changes the `__unrenderable` list, and
//!    that difference is the port's whole point.
//! 2. **Deliberately partial ports.** `string.tclspec` declares four of the
//!    shipped ensemble's twenty-four subcommands and says so in its header.
//! 3. **Data the shipped spec carries and the port does not transcribe**, or
//!    vice versa. Each is named individually — an unexplained difference
//!    fails.
//!
//! Anything not in the table is a real divergence and fails the test.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

use serde_json::Value;
use tcl_spec_studio::spectcl;
use tcl_spec_studio::{draft, load_command, schema};

/// One command inside a ported pack.
struct PortedCommand {
    /// The command name, as both the pack and the registry spell it.
    name: &'static str,
    /// A dialect profile the shipped spec is visible in.
    dialect: &'static str,
    /// Top-level draft keys allowed to differ, each with its reason.
    unequal: &'static [(&'static str, &'static str)],
    /// Subcommand-level draft keys allowed to differ, as
    /// `(subcommand, key, reason)`.
    unequal_subcommand: &'static [(&'static str, &'static str, &'static str)],
    /// When set, only these subcommands are compared — the port is
    /// deliberately partial and the `subcommands` key is checked per named
    /// subcommand instead of as a whole.
    subcommand_subset: &'static [&'static str],
}

const NONE: &[(&str, &str)] = &[];
const NO_SUBS: &[(&str, &str, &str)] = &[];
const ALL_SUBS: &[&str] = &[];

struct Port {
    file: &'static str,
    commands: &'static [PortedCommand],
}

const PORTS: &[Port] = &[
    Port {
        file: "lsort.tclspec",
        commands: &[PortedCommand {
            name: "lsort",
            dialect: "tcl9.1",
            unequal: NONE,
            unequal_subcommand: NO_SUBS,
            subcommand_subset: ALL_SUBS,
        }],
    },
    Port {
        file: "foreach.tclspec",
        commands: &[PortedCommand {
            name: "foreach",
            dialect: "tcl9.1",
            unequal: NONE,
            unequal_subcommand: NO_SUBS,
            subcommand_subset: ALL_SUBS,
        }],
    },
    Port {
        file: "switch.tclspec",
        commands: &[PortedCommand {
            name: "switch",
            dialect: "tcl9.1",
            unequal: NONE,
            unequal_subcommand: NO_SUBS,
            subcommand_subset: ALL_SUBS,
        }],
    },
    Port {
        file: "if.tclspec",
        commands: &[PortedCommand {
            name: "if",
            dialect: "tcl9.1",
            unequal: NONE,
            unequal_subcommand: NO_SUBS,
            subcommand_subset: ALL_SUBS,
        }],
    },
    Port {
        file: "upvar.tclspec",
        commands: &[PortedCommand {
            name: "upvar",
            dialect: "tcl9.1",
            unequal: NONE,
            unequal_subcommand: NO_SUBS,
            subcommand_subset: ALL_SUBS,
        }],
    },
    Port {
        file: "geturl.tclspec",
        commands: &[
            PortedCommand {
                name: "uri::geturl",
                dialect: "tcl9.1",
                unequal: &[("surface", "the port is faithful to the `.rs` (`surface: None`), but the shipped registry states a tcllib command's surface at REGISTRATION time — `tcllib_command_specs` gives every tcllib command the Tcl core rows its modules need, since a tcllib module is a `package require` away and the F5 embedded surfaces have none. A pack loader installing into a registry would apply the same blanket rule; the spec field itself matches")],
                unequal_subcommand: NO_SUBS,
                subcommand_subset: ALL_SUBS,
            },
            PortedCommand {
                name: "http::geturl",
                dialect: "tcl9.1",
                unequal: NONE,
                unequal_subcommand: NO_SUBS,
                subcommand_subset: ALL_SUBS,
            },
        ],
    },
    Port {
        file: "irules-http-header.tclspec",
        commands: &[PortedCommand {
            name: "HTTP::header",
            dialect: "f5-irules",
            unequal: &[(
                "traits",
                "the shipped registry adds REQUIRES_HTTP_CONTEXT at registration \
                 (irules/mod.rs's http_context() wrapper over IRULES_SPECS — \
                 pack-level curation, the IRULE1201 source of truth), not in the \
                 spec literal. The port is faithful to the .rs; a pack loader \
                 installing into the iRules registry applies the same wrapper \
                 policy, exactly like the tcllib dialect fill above",
            )],
            unequal_subcommand: NO_SUBS,
            subcommand_subset: ALL_SUBS,
        }],
    },
    Port {
        file: "return.tclspec",
        commands: &[PortedCommand {
            name: "return",
            dialect: "tcl9.1",
            unequal: NONE,
            unequal_subcommand: NO_SUBS,
            subcommand_subset: ALL_SUBS,
        }],
    },
    Port {
        file: "snit-type.tclspec",
        commands: &[PortedCommand {
            name: "snit::type",
            dialect: "tcl9.1",
            unequal: &[("surface", "the port is faithful to the `.rs` (`surface: None`), but the shipped registry states a tcllib command's surface at REGISTRATION time — `tcllib_command_specs` gives every tcllib command the Tcl core rows its modules need, since a tcllib module is a `package require` away and the F5 embedded surfaces have none. A pack loader installing into a registry would apply the same blanket rule; the spec field itself matches")],
            unequal_subcommand: NO_SUBS,
            subcommand_subset: ALL_SUBS,
        }],
    },
    Port {
        file: "oo-class.tclspec",
        commands: &[PortedCommand {
            name: "oo::class",
            dialect: "tcl9.1",
            unequal: NONE,
            unequal_subcommand: NO_SUBS,
            subcommand_subset: ALL_SUBS,
        }],
    },
    Port {
        file: "string.tclspec",
        commands: &[PortedCommand {
            name: "string",
            dialect: "tcl9.1",
            unequal: &[(
                "__unrenderable",
                "the shipped `string` carries `completion`, `world_effects`, \
                 `state_transitions`, `dispatch_dependencies` and \
                 `result_stability` descriptors that arrived with the \
                 completion-code and stable-call-CSE work. SpecTcl has no \
                 surface for any of them yet — the loader already reports \
                 `world_effects` / `state_transitions` rows as \"not yet \
                 loadable; dropped\" — so they stay on the unrenderable list \
                 and the port does not transcribe them. Every field the port \
                 does carry matches",
            )],
            unequal_subcommand: &[(
                "length",
                "__unrenderable",
                "the same five descriptors, declared per-subcommand on \
                 `string length`; unrenderable for the same reason",
            )],
            subcommand_subset: &["length", "is", "map", "range"],
        }],
    },
];

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/design/spec-dsl-examples")
        .canonicalize()
        .expect("the spec-dsl-examples directory")
}

/// One field that did not match, rendered for the report.
struct Diff {
    key: String,
    ported: String,
    shipped: String,
}

fn short(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 220 {
        let head: String = text.chars().take(200).collect();
        format!("{head}… ({} bytes)", text.len())
    } else {
        text
    }
}

fn compare_keys(
    ported: &Value,
    shipped: &Value,
    keys: impl Iterator<Item = &'static str>,
    prefix: &str,
    out: &mut Vec<Diff>,
) {
    for key in keys {
        let a = ported.get(key).unwrap_or(&Value::Null);
        let b = shipped.get(key).unwrap_or(&Value::Null);
        if a != b {
            out.push(Diff {
                key: format!("{prefix}{key}"),
                ported: short(a),
                shipped: short(b),
            });
        }
    }
}

fn subcommand<'a>(draft: &'a Value, name: &str) -> Option<&'a Value> {
    draft
        .get("subcommands")?
        .as_array()?
        .iter()
        .find(|sub| sub.get("name").and_then(Value::as_str) == Some(name))
}

/// Load every port and report, per command, which draft fields differ.
#[test]
#[allow(clippy::too_many_lines)]
fn every_port_loads_and_matches_its_shipped_spec() {
    let dir = examples_dir();
    let mut report = String::new();
    let mut failures = 0usize;

    for port in PORTS {
        let source = std::fs::read_to_string(dir.join(port.file))
            .unwrap_or_else(|e| panic!("reading {}: {e}", port.file));
        let pack = spectcl::evaluate_pack(&source);

        let _ = writeln!(
            report,
            "\n=== {} — pack `{}` v{} ({} command(s), {} notice(s))",
            port.file,
            pack.name,
            pack.dsl_version,
            pack.commands.len(),
            pack.notices.len()
        );
        for notice in &pack.notices {
            let _ = writeln!(report, "    notice: {notice}");
        }

        assert_eq!(
            pack.commands.len(),
            port.commands.len(),
            "{} declared {} commands, expected {}",
            port.file,
            pack.commands.len(),
            port.commands.len()
        );

        for expected in port.commands {
            let loaded = pack
                .command(expected.name)
                .unwrap_or_else(|| panic!("{} declares no `{}`", port.file, expected.name));
            // Seeded through the *same* `from_command_spec` the browser uses
            // for a shipped spec, which is what makes the comparison below
            // meaningful — both sides are drafts built by one code path.
            let ported = Value::Object(draft::from_command_spec(loaded.spec));
            let shipped = load_command(expected.name, expected.dialect)
                .unwrap_or_else(|| panic!("no shipped `{}`", expected.name));

            let mut diffs: Vec<Diff> = Vec::new();
            let skip_subcommands = !expected.subcommand_subset.is_empty();
            compare_keys(
                &ported,
                &shipped,
                schema::COMMAND_FIELDS
                    .iter()
                    .map(|field| field.key)
                    .filter(|key| !(skip_subcommands && *key == "subcommands")),
                "",
                &mut diffs,
            );
            compare_keys(
                &ported,
                &shipped,
                std::iter::once(tcl_spec_studio::draft::UNRENDERABLE_KEY),
                "",
                &mut diffs,
            );

            for name in expected.subcommand_subset {
                let a = subcommand(&ported, name)
                    .unwrap_or_else(|| panic!("`{}` ports no subcommand `{name}`", expected.name));
                let b = subcommand(&shipped, name)
                    .unwrap_or_else(|| panic!("shipped `{}` has no `{name}`", expected.name));
                compare_keys(
                    a,
                    b,
                    schema::SUBCOMMAND_FIELDS.iter().map(|field| field.key),
                    &format!("subcommands[{name}]."),
                    &mut diffs,
                );
                compare_keys(
                    a,
                    b,
                    std::iter::once(tcl_spec_studio::draft::UNRENDERABLE_KEY),
                    &format!("subcommands[{name}]."),
                    &mut diffs,
                );
            }

            let allowed: BTreeSet<String> = expected
                .unequal
                .iter()
                .map(|(key, _)| (*key).to_owned())
                .chain(
                    expected
                        .unequal_subcommand
                        .iter()
                        .map(|(sub, key, _)| format!("subcommands[{sub}].{key}")),
                )
                .collect();

            let _ = writeln!(
                report,
                "  {} @ {}: {} differing field(s)",
                expected.name,
                expected.dialect,
                diffs.len()
            );
            for diff in &diffs {
                let documented = allowed.contains(&diff.key);
                let reason = expected
                    .unequal
                    .iter()
                    .find(|(key, _)| *key == diff.key)
                    .map(|(_, why)| *why)
                    .or_else(|| {
                        expected
                            .unequal_subcommand
                            .iter()
                            .find(|(sub, key, _)| format!("subcommands[{sub}].{key}") == diff.key)
                            .map(|(_, _, why)| *why)
                    })
                    .unwrap_or("UNDOCUMENTED");
                if !documented {
                    failures += 1;
                }
                let _ = writeln!(
                    report,
                    "    {} {}\n        ported : {}\n        shipped: {}\n        reason : {reason}",
                    if documented { "[documented]" } else { "[FAIL]" },
                    diff.key,
                    diff.ported,
                    diff.shipped,
                );
            }

            for (key, why) in expected.unequal {
                if !diffs.iter().any(|d| d.key == *key) {
                    let _ = writeln!(
                        report,
                        "    [stale] `{key}` is documented as unequal ({why}) but now matches"
                    );
                }
            }
        }
    }

    println!("{report}");
    assert_eq!(failures, 0, "undocumented draft differences:\n{report}");
}

/// A pack whose vocabulary this server does not know still loads: the unknown
/// words drop with a notice and everything around them survives.
#[test]
fn unknown_vocabulary_drops_with_a_notice_rather_than_failing() {
    let pack = spectcl::evaluate_pack(
        r"
        speclib probe 1.0 {
            pragma something
            command demo {
                arity 1..2
                traits {BYTE_COMPILED NOT_A_REAL_TRAIT}
                dialects {all-tcl not-a-dialect}
                unheard_of_property 3
                option -flag -detail {A flag.} -not-a-flag 1
            }
        }
        ",
    );
    let demo = pack.command("demo").expect("the command still loads");
    assert_eq!(demo.spec.arity.min, 1);
    assert_eq!(demo.spec.arity.max, 2);
    assert_eq!(demo.spec.options.len(), 1);
    assert!(
        demo.spec
            .traits
            .contains(tcl_registry::traits::Traits::BYTE_COMPILED)
    );
    let messages: Vec<String> = pack.notices.iter().map(ToString::to_string).collect();
    let joined = messages.join("\n");
    for expected in [
        "pragma",
        "NOT_A_REAL_TRAIT",
        "not-a-dialect",
        "unheard_of_property",
        "-not-a-flag",
    ] {
        assert!(
            joined.contains(expected),
            "expected a notice mentioning `{expected}`, got:\n{joined}"
        );
    }
}

/// Every hook a pack declares is carried as text with its family's declared
/// metadata, and is installed as that family's abstention until the sandbox
/// lands.
#[test]
fn hook_bodies_are_carried_as_text_with_their_family_metadata() {
    let source = std::fs::read_to_string(examples_dir().join("return.tclspec")).expect("return");
    let pack = spectcl::evaluate_pack(&source);
    let command = pack.command("return").expect("return");

    let fields: Vec<&str> = command.hooks.iter().map(|hook| hook.field).collect();
    assert!(fields.contains(&"arg_role_resolver"));
    assert!(fields.contains(&"context_gate"));
    assert!(fields.contains(&"options.arity_hook"));

    let arity_hook = command
        .hooks
        .iter()
        .find(|hook| hook.field == "options.arity_hook")
        .expect("the option-arity hook");
    assert_eq!(arity_hook.family, spectcl::HookFamily::OptionArity);
    assert_eq!(arity_hook.family.verbs(), &["consume"]);
    assert_eq!(arity_hook.family.silence(), "consume one word");
    // Normative: the option-arity hook and the two fold families run only on
    // an all-literal call.
    assert!(arity_hook.family.requires_all_literal());
    match &arity_hook.source {
        // `..`: this assertion is about the *body* the row carried. A hook
        // source grows fields (declared inputs, and whatever the hook host
        // needs next); naming them here would make every such addition a
        // failing test with nothing to say.
        spectcl::HookSource::Body { params, body, .. } => {
            assert_eq!(params, &["words".to_owned(), "ctx".to_owned()]);
            assert!(body.contains("option-value-start"));
            assert!(body.contains("consume 0 -invalid {missing value for -errorstack}"));
        }
        other => panic!("expected a Tcl body, got {other:?}"),
    }

    // Silence keeps a security finding alive; that is the one family whose
    // abstention is not "no opinion".
    assert_eq!(
        spectcl::HookFamily::TaintSinkGate.silence(),
        "the sink applies"
    );
}

/// A derivation keyword is an explicit opt-in, never an implicit consequence
/// of declaring the data it reads.
#[test]
fn derivations_are_recorded_as_derivations() {
    for (file, command, field, keyword) in [
        (
            "oo-class.tclspec",
            "oo::class",
            "arg_role_resolver",
            "from-manufacturers",
        ),
        ("if.tclspec", "if", "clause_shape_check", "clause_grammar"),
    ] {
        let source = std::fs::read_to_string(examples_dir().join(file)).expect(file);
        let pack = spectcl::evaluate_pack(&source);
        let loaded = pack.command(command).expect(command);
        let hook = loaded
            .hooks
            .iter()
            .find(|hook| hook.field == field)
            .unwrap_or_else(|| panic!("{command} declares no {field}"));
        assert_eq!(
            hook.source,
            spectcl::HookSource::Derived {
                keyword: keyword.to_owned()
            }
        );
    }
}

/// Brace-quoted prose is every byte between the braces: the loader never
/// reflows, dedents, or joins, because the gate compares the loaded field
/// with the compiled `&'static str` byte for byte.
#[test]
fn braced_prose_survives_byte_for_byte() {
    let source = std::fs::read_to_string(examples_dir().join("return.tclspec")).expect("return");
    let pack = spectcl::evaluate_pack(&source);
    let hover = pack
        .command("return")
        .expect("return")
        .spec
        .hover
        .expect("a hover");
    // One `example` block, not three rows — the shipped `examples` string
    // separates its examples with a *blank* line, and repeated `example` rows
    // join with a single newline.
    assert!(hover.examples.contains("\n\n# Re-throw a caught error"));
    assert!(hover.examples.starts_with("proc safeDiv {a b} {"));
    assert!(!hover.examples.ends_with('\n'));
}

/// Every call shape `if_.rs`'s own tests and the design memo's matrix name,
/// plus the two subtle rows the memo says prove where keywords match.
const IF_CORPUS: &[&str] = &[
    "",
    "1",
    "1 a",
    "1 then",
    "1 then a",
    "1 a else b",
    "1 a else",
    "a x elseif b y else z",
    "1 a b c",
    "else a",
    "else {a}",
    "1 a elseif",
    "1 a elseif b",
    "1 a elseif b then",
    "1 a elseif else b",
    "1 a elseif b y",
    "1 a {b} {c}",
    "1 then a elseif 2 then b else c",
];

/// **Derivation is a proof obligation, not a shorthand.**
///
/// `if.tclspec` replaces two hook functions with a three-line
/// `clause_grammar`. This walks the derived grammar against the shipped
/// `if`'s own hooks over its whole test matrix and asserts they agree case
/// for case — roles *and* the reported defect.
#[test]
fn the_clause_grammar_derivation_agrees_with_the_shipped_walk() {
    let source = std::fs::read_to_string(examples_dir().join("if.tclspec")).expect("if");
    let pack = spectcl::evaluate_pack(&source);
    let ported = pack.command("if").expect("if");
    let grammar = ported
        .clause_grammar
        .as_ref()
        .expect("if.tclspec declares a clause_grammar");

    let shipped = tcl_spec_studio::environment::store_for_dialect("tcl9.1")
        .get("if")
        .expect("shipped if");
    let shipped_roles = shipped.arg_role_resolver.expect("if's role resolver");
    let shipped_shape = shipped.clause_shape_check.expect("if's shape check");

    for call in IF_CORPUS {
        let words: Vec<&str> = call.split_whitespace().collect();
        let derived = grammar.walk(&words);
        assert_eq!(
            derived.roles,
            shipped_roles(&words),
            "roles disagree for `if {call}`"
        );
        assert_eq!(
            derived.error,
            shipped_shape(&words),
            "shape disagrees for `if {call}`"
        );
    }
}

/// The sharpest derivation claim in the design: `oo::class`'s resolver reads
/// `TCLOO_GRAMMAR.manufacturers`, while the port declares
/// `TCLOO_ROOT_CLASS_MANUFACTURERS`-shaped rows. The two are *different*
/// tables that agree on every keyword and body index today, which makes the
/// derivation correct now and not correct by construction.
///
/// So the loader must **diff them**, and a future divergence must fail here
/// rather than pass silently.
#[test]
fn the_manufacturer_derivation_agrees_with_the_shipped_resolver() {
    let source = std::fs::read_to_string(examples_dir().join("oo-class.tclspec")).expect("oo");
    let pack = spectcl::evaluate_pack(&source);
    let ported = pack.command("oo::class").expect("oo::class");

    // 1. The two tables agree on every keyword and body index. Visibility is
    //    deliberately *not* compared: that is exactly where they differ.
    let shipped_grammar = &tcl_registry::definer::TCLOO_GRAMMAR;
    for method in ported.spec.manufacturer_methods {
        let counterpart = shipped_grammar
            .manufacturer(method.keyword)
            .unwrap_or_else(|| panic!("TCLOO_GRAMMAR has no `{}`", method.keyword));
        assert_eq!(
            method.definition_body_at, counterpart.definition_body_at,
            "`{}` disagrees on where the definition body is",
            method.keyword
        );
    }
    assert_eq!(
        ported.spec.manufacturer_methods.len(),
        shipped_grammar.manufacturers.len(),
        "the pack's rows and TCLOO_GRAMMAR no longer cover the same keywords"
    );

    // 2. The derivation reproduces the shipped resolver's answer, call for
    //    call — including the bounds-checked cases that emit nothing.
    let shipped = tcl_spec_studio::environment::store_for_dialect("tcl9.1")
        .get("oo::class")
        .expect("shipped oo::class");
    let shipped_roles = shipped
        .arg_role_resolver
        .expect("oo::class's role resolver");
    for call in [
        "",
        "create",
        "create Fruit",
        "create Fruit {method eat {} {}}",
        "new",
        "new {method eat {} {}}",
        "createWithNamespace",
        "createWithNamespace Fruit ::ns",
        "createWithNamespace Fruit ::ns {method eat {} {}}",
        "destroy",
        "notAManufacturer Fruit body",
    ] {
        let words: Vec<&str> = call.split_whitespace().collect();
        assert_eq!(
            spectcl::roles_from_manufacturers(ported.spec, &words),
            shipped_roles(&words),
            "`oo::class {call}` disagrees"
        );
    }
}

/// The pack-declared `state_transitions … resolver from-frame-effect` claim
/// rests on `upvar`'s own `frame_effect`, so the port must at least carry that
/// descriptor across unchanged.
#[test]
fn upvar_carries_its_frame_effect_verbatim() {
    let source = std::fs::read_to_string(examples_dir().join("upvar.tclspec")).expect("upvar");
    let pack = spectcl::evaluate_pack(&source);
    let ported = pack.command("upvar").expect("upvar");
    let shipped = tcl_spec_studio::environment::store_for_dialect("tcl9.1")
        .get("upvar")
        .expect("shipped upvar");
    assert_eq!(ported.spec.frame_effect, shipped.frame_effect);
}

/// `snit-type.tclspec` spells `SNIT_GRAMMAR` out inline rather than naming it,
/// which is the whole reason `definition_body` stopped being reference-only.
/// The draft records the field as un-round-trippable either way, so this
/// compares the **descriptor the loader built** against the shipped constant,
/// field for field — the only way the port's "complete" claim is checkable.
#[test]
fn the_inline_snit_grammar_matches_the_shipped_constant() {
    let source = std::fs::read_to_string(examples_dir().join("snit-type.tclspec")).expect("snit");
    let pack = spectcl::evaluate_pack(&source);
    let built = pack
        .command("snit::type")
        .expect("snit::type")
        .spec
        .definition_body
        .expect("the inline definition_body");
    let shipped = &tcl_registry::definer::SNIT_GRAMMAR;

    // The port's own claim, spelled out so a silently-empty grammar cannot
    // pass the field comparisons below by matching nothing against nothing.
    assert_eq!(built.members.len(), 15, "member rows");
    assert_eq!(
        built.builtin_object_methods.len(),
        6,
        "built-in object methods"
    );
    assert_eq!(built.member_body_commands.len(), 1, "member-body commands");
    assert_eq!(built.manufacturers.len(), 1, "manufacturer rows");

    assert_eq!(
        format!("{:?}", built.family),
        format!("{:?}", shipped.family)
    );
    assert_eq!(built.implicit_vars, shipped.implicit_vars);
    assert_eq!(
        built.member_body_namespace_path,
        shipped.member_body_namespace_path
    );
    assert_eq!(built.builtin_type_methods, shipped.builtin_type_methods);
    assert_eq!(
        built.builtin_terminating_methods,
        shipped.builtin_terminating_methods
    );
    assert_eq!(built.bare_word_construction, shipped.bare_word_construction);
    assert_eq!(
        built.dynamic_method_dispatch,
        shipped.dynamic_method_dispatch
    );
    assert_eq!(
        built.unknown_dispatch_method,
        shipped.unknown_dispatch_method
    );
    assert_eq!(
        built.property_accessor_methods,
        shipped.property_accessor_methods
    );
    assert_eq!(
        format!("{:?}", built.members),
        format!("{:?}", shipped.members),
        "member rows"
    );
    assert_eq!(
        format!("{:?}", built.builtin_object_methods),
        format!("{:?}", shipped.builtin_object_methods),
        "built-in object methods"
    );
    assert_eq!(
        format!("{:?}", built.member_body_commands),
        format!("{:?}", shipped.member_body_commands),
        "member-body commands"
    );
    assert_eq!(
        format!("{:?}", built.manufacturers),
        format!("{:?}", shipped.manufacturers),
        "manufacturer rows"
    );

    // The one function pointer in the grammar, read as data: an exact-word set
    // plus a prefix set. Compared by behaviour, since a pointer has none.
    let built_hint = built.bare_word_construction_hint.expect("a hint");
    let shipped_hint = shipped.bare_word_construction_hint.expect("a hint");
    for word in ["%AUTO%", ".w", ".", "auto", "%auto%", "x.y", ""] {
        assert_eq!(
            built_hint(word),
            shipped_hint(word),
            "the bare-word construction hint disagrees on {word:?}"
        );
    }
}
