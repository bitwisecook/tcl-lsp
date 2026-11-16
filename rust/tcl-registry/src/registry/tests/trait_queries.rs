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

//! Trait and metadata queries: membership lists, frame sensitivity, unit linkage and audited allow lists.

use super::*;

// Cross-language RPC roles.

/// The remote-method relation is registry data on the iRules surface, so
/// it exists there and nowhere else. A consumer asking the registry —
/// rather than naming `ILX::call` — is what makes the dialect gate
/// automatic.
#[test]
fn remote_method_commands_are_an_irules_only_surface() {
    let plain = CommandRegistry::build_default();
    assert!(
        plain.remote_method_commands().is_empty(),
        "stock Tcl has no cross-language RPC commands"
    );
    assert!(plain.remote_method("ILX::call").is_none());

    let mut irules = CommandRegistry::build_default();
    irules.load_irules();
    assert_eq!(
        irules.remote_method_commands(),
        vec!["ILX::call", "ILX::init", "ILX::notify"]
    );
    // The explicitly-global spelling is the same command.
    assert!(irules.remote_method("::ILX::call").is_some());
    assert!(
        irules
            .remote_method("ILX::init")
            .is_some_and(|role| role.opens_handle().is_some())
    );
    assert!(
        irules
            .remote_method("ILX::notify")
            .is_some_and(|role| role.calls_method().is_some())
    );
}

/// `SAFE_INTERP_HIDDEN` and `TRANSFERS_CONTROL` were the same bit.
///
/// Both were spelled `1 << 61`, so the 65th trait silently aliased the
/// 61st. Because `FRAME_SENSITIVE_TRAITS` unions in `TRANSFERS_CONTROL`,
/// every safe-interp-hidden command — `file`, `source`, `encoding`,
/// `open`, … — read as frame-sensitive and had the inline-proc code
/// action suppressed, while `break`/`continue`/`yield`/`yieldto`/
/// `tailcall` read as safe-interp-hidden.
///
/// The two are now separate enum variants, so the aliasing is
/// unrepresentable rather than merely fixed. This pins the *behaviour*
/// that was wrong, which a type-level guarantee alone does not cover.
#[test]
fn safe_interp_hidden_is_not_control_transfer() {
    let reg = CommandRegistry::build_default();
    assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);

    for name in ["break", "continue", "yield", "yieldto", "tailcall"] {
        let spec = reg.get(name).expect("control-transfer command");
        assert!(
            spec.traits.contains(Traits::TRANSFERS_CONTROL),
            "{name} must transfer control"
        );
        assert!(
            !spec.traits.contains(Traits::SAFE_INTERP_HIDDEN),
            "{name} is not hidden in a safe interpreter"
        );
        assert!(reg.is_frame_sensitive(name), "{name} is frame-sensitive");
    }

    // Safe-hidden commands carrying no *other* frame-sensitive trait.
    // These are the ones the alias was wrongly marking: each had the
    // inline-proc code action suppressed on it.
    for name in [
        "file", "encoding", "open", "socket", "exec", "cd", "pwd", "glob", "load", "unload",
    ] {
        let spec = reg.get(name).expect("safe-hidden command");
        assert!(
            spec.traits.contains(Traits::SAFE_INTERP_HIDDEN),
            "{name} is hidden in a safe interpreter"
        );
        assert!(
            !spec.traits.contains(Traits::TRANSFERS_CONTROL),
            "{name} does not transfer control"
        );
        assert!(
            !reg.is_frame_sensitive(name),
            "{name} must not be frame-sensitive — the alias suppressed the \
                 inline-proc code action on it"
        );
    }

    // The correction must not swing too far: `source` and `exit` are
    // frame-sensitive on their own merits (a barrier and a block
    // terminator respectively), and stay so.
    for name in ["source", "exit"] {
        assert!(
            reg.is_frame_sensitive(name),
            "{name} is frame-sensitive independently of the trait alias"
        );
    }
}

#[test]
fn writes_first_arg_variable_membership() {
    let reg = CommandRegistry::build_default();
    // TP: the five first-arg writers.
    for cmd in ["set", "append", "lappend", "incr", "lset"] {
        assert!(reg.writes_first_arg_variable(cmd), "{cmd} writes arg 0");
    }
    // FP guards: `unset` destroys (not a write); value-taking and
    // unknown commands are out.
    for cmd in ["unset", "puts", "llength", "foreach", "nosuchcmd"] {
        assert!(!reg.writes_first_arg_variable(cmd), "{cmd} must not match");
    }
}

#[test]
fn rmw_first_arg_variable_membership() {
    let reg = CommandRegistry::build_default();
    // TP: read-modify-write commands fold the current value in.
    for cmd in ["append", "lappend", "incr", "lset"] {
        assert!(reg.rmw_first_arg_variable(cmd), "{cmd} is RMW");
    }
    // FP guards: a whole-value `set` is rename-safe; `unset` destroys.
    for cmd in ["set", "unset", "puts", "nosuchcmd"] {
        assert!(!reg.rmw_first_arg_variable(cmd), "{cmd} must not match");
    }
}

#[test]
fn commands_with_trait_query() {
    let reg = CommandRegistry::build_default();
    let control_flow = reg.commands_with_trait(Traits::CONTROL_FLOW);
    assert!(control_flow.contains(&"for"));
    assert!(control_flow.contains(&"if"));
    assert!(control_flow.contains(&"while"));
    assert!(!control_flow.contains(&"puts"));
}

#[test]
fn byte_compiled_covers_the_core_builtins() {
    let reg = CommandRegistry::build_default();
    // Every registered command the minifier must never alias.
    // The former built-in skip list (minus the
    // non-command keywords `else` / `elseif` and the unregistered
    // `pwd`).
    let expected = [
        "set",
        "unset",
        "proc",
        "if",
        "while",
        "for",
        "foreach",
        "switch",
        "return",
        "break",
        "continue",
        "expr",
        "catch",
        "try",
        "throw",
        "package",
        "namespace",
        "upvar",
        "uplevel",
        "variable",
        "global",
        "append",
        "lappend",
        "incr",
        "info",
        "string",
        "list",
        "llength",
        "lindex",
        "lrange",
        "lsort",
        "lsearch",
        "lreplace",
        "linsert",
        "dict",
        "array",
        "regexp",
        "regsub",
        "scan",
        "format",
        "open",
        "close",
        "read",
        "gets",
        "eof",
        "flush",
        "seek",
        "tell",
        "fconfigure",
        "fcopy",
        "fileevent",
        "socket",
        "after",
        "update",
        "vwait",
        "rename",
        "source",
        "eval",
        "apply",
        "tailcall",
        "error",
        "cd",
        "file",
        "glob",
        "clock",
        "binary",
        "encoding",
        "interp",
        "load",
        "exit",
        "pid",
        "exec",
        "chan",
        "puts",
    ];
    for name in expected {
        assert!(
            reg.is_byte_compiled(name),
            "{name} should carry Traits::BYTE_COMPILED"
        );
    }
    // A user-proc-like name and a command outside the curated
    // skip set must not carry the trait.
    assert!(!reg.is_byte_compiled("my_helper_proc"));
    assert!(!reg.is_byte_compiled("split"));
}

#[test]
fn not_proc_factory_covers_registered_skip_heads() {
    let reg = CommandRegistry::build_default();
    // Registered heads from the former `_FACTORY_SKIP_HEADS` list
    // (the four non-command heads method / classmethod /
    // itcl::class / ::itcl::class are handled by the scanner's
    // residual set, not the registry).
    let expected = [
        "proc",
        "namespace",
        "if",
        "switch",
        "while",
        "for",
        "foreach",
        "try",
        "catch",
        "eval",
        "apply",
        "expr",
        "uplevel",
        "upvar",
        "variable",
        "set",
        "lappend",
        "dict",
        "array",
        "string",
        "list",
        "lindex",
        "package",
        "source",
        "interp",
        "oo::class",
        "oo::define",
        "oo::objdefine",
    ];
    for name in expected {
        assert!(
            reg.is_not_proc_factory(name),
            "{name} should carry Traits::NOT_PROC_FACTORY"
        );
    }
    // A real factory-wrapper head must not be skipped.
    assert!(!reg.is_not_proc_factory("my_factory"));
}

#[test]
fn frameless_runtime_covers_the_audited_allow_list() {
    let reg = CommandRegistry::build_default();
    let got: std::collections::HashSet<&str> = reg
        .commands_with_trait(Traits::FRAMELESS_RUNTIME)
        .into_iter()
        .collect();
    let expected: std::collections::HashSet<&str> = [
        "list",
        "lindex",
        "lrange",
        "linsert",
        "llength",
        "lsort",
        "lsearch",
        "lappend",
        "lreverse",
        "lreplace",
        // Tcl 9.0+: shares `lreplace`'s core (see
        // `tcl-vm/src/cmd_list.rs::cmd_ledit`); a flat single-variable
        // read-modify-write with no eval fallback and no sublist-index
        // descent (unlike `lset`, which deliberately stays off this
        // list) — see `commands/tcl/ledit.rs`.
        "ledit",
        "lrepeat",
        "lassign",
        // Tcl 9.0+: a flat native dispatch over `tcl-cmd-core::lseq`
        // (`tcl-vm/src/cmd_lseq.rs::cmd_lseq`) that evaluates nothing —
        // see `commands/tcl/lseq.rs`.
        "lseq",
        "concat",
        "split",
        "join",
        "string",
        "format",
        "expr",
        "global",
        "variable",
        "upvar",
        "namespace",
        "set",
        "incr",
        "append",
        "unset",
        "puts",
        "return",
        "error",
        "continue",
        "break",
        // `throw type message` (Tcl 8.6+) — `cmd_throw`
        // (`tcl-vm/src/cmd_try.rs`) takes its two already-substituted
        // arguments, validates `type` as a non-empty Tcl list, and
        // builds its return-options dict directly with no eval
        // fallback of any kind (it never touches `vm`), structurally
        // identical to `cmd_error` in this respect. See
        // `commands/tcl/throw_.rs`.
        "throw",
        // `::tcl::unsupported::corotype coroName` (and its
        // namespace-relative `tcl::unsupported::corotype` spelling) — a
        // flat lookup into the coroutine table
        // (`tcl-vm/src/cmd_coro.rs::cmd_corotype`): no eval fallback, no
        // `Frame` of its own. Both spellings share the same `make_spec`
        // and so the same traits. See
        // `commands/tcl/tcl_unsupported_corotype.rs`.
        "::tcl::unsupported::corotype",
        "tcl::unsupported::corotype",
        // `::tcl::unsupported::representation value` — describes the
        // value it was handed and runs nothing. See
        // `commands/tcl/tcl_unsupported_representation.rs`.
        "::tcl::unsupported::representation",
        "tcl::unsupported::representation",
    ]
    .into_iter()
    .collect();
    assert_eq!(
        got, expected,
        "FRAMELESS_RUNTIME stamps drifted from the audited allow-list"
    );
}

/// `global` / `variable` / `upvar` carry
/// `CREATES_DYNAMIC_BARRIER` so SSA's barrier-def walk knows the
/// per-arg list belongs to `var_scoping`, not the role-driven
/// `VarWrite` query.
#[test]
fn creates_dynamic_barrier_trait_marks_scope_aliases() {
    let reg = CommandRegistry::build_default();
    for cmd in &["global", "variable", "upvar"] {
        assert!(
            reg.get(cmd)
                .unwrap()
                .traits
                .contains(Traits::CREATES_DYNAMIC_BARRIER),
            "{cmd} should carry CREATES_DYNAMIC_BARRIER",
        );
    }
    // `set` does NOT carry the trait — its VarWrite at arg 0 is
    // a single-target def, not a vararg list.
    assert!(
        !reg.get("set")
            .unwrap()
            .traits
            .contains(Traits::CREATES_DYNAMIC_BARRIER)
    );
}

#[test]
fn procedure_definition_words_place_the_three_words_of_tcl_proc() {
    assert_eq!(
        CommandRegistry::build_default()
            .procedure_definition_words("proc", &["f", "{a b}", "{ body }"]),
        None,
        "catalogue presence supplies no native formal-argument grammar"
    );
    let reg = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    assert_eq!(
        reg.procedure_definition_words("proc", &["f", "{a b}", "{ body }"]),
        Some(ProcedureWords {
            name: 0,
            params: 1,
            statics: None,
            body: 2,
        })
    );
}

#[test]
fn procedure_definition_words_answers_only_for_a_procedure_definer() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.procedure_definition_words("set", &["x", "1", "2"]),
        None
    );
    assert_eq!(reg.procedure_definition_words("proc", &["f"]), None);
    assert_eq!(reg.procedure_definition_words("no_such_head", &[]), None);
}

// -- ``is_canonical_list_command`` (W101 safe-idiom driver)

#[test]
fn is_canonical_list_command_includes_list_and_split_excludes_concat() {
    let reg = CommandRegistry::build_default();
    assert!(reg.is_canonical_list_command("list"));
    assert!(reg.is_canonical_list_command("linsert"));
    assert!(reg.is_canonical_list_command("split"));
    assert!(reg.is_canonical_list_command("lreverse"));
    // ``concat`` returns LIST but is the explicit non-canonical
    // exclusion.
    assert!(!reg.is_canonical_list_command("concat"));
    // Non-list commands (e.g. ``set``) are filtered out.
    assert!(!reg.is_canonical_list_command("set"));
    // Unknown commands return false.
    assert!(!reg.is_canonical_list_command("unknownthing"));
}

#[test]
fn is_canonical_list_command_handles_compound_subcommand_keys() {
    let reg = CommandRegistry::build_default();
    // ``dict keys`` returns LIST.
    assert!(reg.is_canonical_list_command("dict keys"));
    // ``dict get`` returns String (or unspecified) — not canonical.
    assert!(!reg.is_canonical_list_command("dict froob"));
}

/// `unit_linkage` composes `spec.traits | sub.traits` and filters to the
/// linkage union, so the answer is subcommand-precise: `package provide`
/// publishes an API surface, `package require` pulls another unit in, and
/// `package names` does neither.
#[test]
fn unit_linkage_is_subcommand_precise() {
    let reg = CommandRegistry::build_default();
    let empty = None;
    assert_eq!(
        reg.unit_linkage("package", &["provide", "mylib", "1.0"], empty),
        Traits::PROVIDES_PACKAGE
    );
    assert_eq!(
        reg.unit_linkage("package", &["ifneeded", "mylib", "1.0", "body"], empty),
        Traits::PROVIDES_PACKAGE
    );
    assert_eq!(
        reg.unit_linkage("package", &["require", "mylib"], empty),
        Traits::LOADS_EXTERNAL_UNIT
    );
    assert_eq!(
        reg.unit_linkage("package", &["names"], empty),
        Traits::empty()
    );
}

/// `invocation_traits` composes `spec.traits | sub.traits` for the
/// concrete call, which is the only way the eval-family bits on the
/// compound members are visible: `namespace eval` / `namespace inscope` /
/// `interp eval` carry `EVALUATES_CODE` on the **subcommand**, so a
/// parent-only `get(name).traits` test misses them.
#[test]
fn invocation_traits_compose_subcommand_traits() {
    let reg = CommandRegistry::build_default();
    let empty = None;
    // TP — the eval family, bare and compound.
    for (name, args) in [
        ("eval", &["{set x 1}"][..]),
        ("uplevel", &["1", "{set x 1}"][..]),
        ("namespace", &["eval", "ns", "{set x 1}"][..]),
        ("namespace", &["inscope", "ns", "{set x 1}"][..]),
        ("interp", &["eval", "slave", "{set x 1}"][..]),
    ] {
        let traits = reg.invocation_traits(name, args, empty);
        assert!(
            traits.contains(Traits::EVALUATES_CODE),
            "{name} {args:?} must carry EVALUATES_CODE"
        );
        assert!(
            traits.contains(Traits::SCRIPT_CONCATENATES_ARGS),
            "{name} {args:?} must carry SCRIPT_CONCATENATES_ARGS"
        );
    }
    // TN — the parent spec alone carries neither bit, which is exactly
    // why composing is required.
    let parent = reg.get("namespace").expect("namespace spec").traits;
    assert!(!parent.contains(Traits::EVALUATES_CODE));
    // TN — a sibling subcommand that evaluates nothing stays clean.
    assert!(
        !reg.invocation_traits("namespace", &["delete", "ns"], empty)
            .contains(Traits::EVALUATES_CODE)
    );
    // TN — an unknown command carries no traits at all.
    assert_eq!(
        reg.invocation_traits("no_such_command_xyz", &["a"], empty),
        Traits::empty()
    );
    // The parent's own traits still compose in — a subcommand's traits
    // are additive, not a replacement.
    assert!(
        reg.invocation_traits("namespace", &["eval", "ns", "{}"], empty)
            .contains(Traits::LANGUAGE_KEYWORD),
        "the parent `namespace` spec's own bits survive composition"
    );
}

/// The query answers per call, and only for a command that substitutes at
/// all — so a consumer never has to read a switch spelling itself.
#[test]
fn substitutions_performed_answers_per_call_and_only_for_substituting_commands() {
    let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
    assert_eq!(
        registry.substitutions_performed("set", &["x", "1"]),
        None,
        "a command that substitutes nothing has no answer to give"
    );
    let all = registry
        .substitutions_performed("subst", &["hello $name"])
        .expect("subst substitutes");
    assert!(all.variables && all.commands && all.backslashes);
    let off = registry
        .substitutions_performed("subst", &["-novariables", "hello $name"])
        .expect("subst substitutes");
    assert!(
        !off.variables && off.commands,
        "only the named kind is disabled"
    );
}

/// The whole registry-declared boundary surface, resolved by name: every
/// command that widens a file's caller set reports its kind, a
/// `::`-qualified spelling resolves the same, and a command that does
/// neither reports nothing.
#[test]
fn unit_linkage_covers_every_declared_boundary_command() {
    let reg = CommandRegistry::build_default();
    let empty = None;
    for (name, args, want) in [
        ("source", &["lib.tcl"][..], Traits::LOADS_EXTERNAL_UNIT),
        ("::source", &["lib.tcl"][..], Traits::LOADS_EXTERNAL_UNIT),
        ("load", &["libx.so"][..], Traits::LOADS_EXTERNAL_UNIT),
        ("auto_load", &["helper"][..], Traits::LOADS_EXTERNAL_UNIT),
        (
            "auto_import",
            &["::lib::*"][..],
            Traits::LOADS_EXTERNAL_UNIT,
        ),
        // `namespace import` is deliberately not a boundary — see its
        // own spec comment.
        (
            "namespace",
            &["import", "::lib::helper"][..],
            Traits::empty(),
        ),
        (
            "namespace",
            &["export", "helper"][..],
            Traits::EXPORTS_COMMAND,
        ),
        (
            "namespace",
            &["ensemble", "create"][..],
            Traits::EXPORTS_COMMAND,
        ),
        ("namespace", &["eval", "::ns", "{}"][..], Traits::empty()),
        ("set", &["x", "1"][..], Traits::empty()),
        ("no_such_command_xyz", &["source"][..], Traits::empty()),
    ] {
        assert_eq!(
            reg.unit_linkage(name, args, empty),
            want,
            "unit_linkage({name}, {args:?})"
        );
    }
}

/// `SAFE_INTERP_HIDDEN` has its own bit, distinct from `TRANSFERS_CONTROL`:
/// every `break`/`continue`/`tailcall`/`yield` reads as safe-interp-hidden
/// and every `cd`/`exec`/`glob`/… reads as control-transferring, which
/// `FRAME_SENSITIVE_TRAITS` consumes directly.
#[test]
fn safe_interp_hidden_no_longer_aliases_transfers_control() {
    let reg = CommandRegistry::build_default();
    assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);
    let cd = reg.get("cd").expect("cd is registered");
    assert!(cd.traits.contains(Traits::SAFE_INTERP_HIDDEN));
    assert!(!cd.traits.contains(Traits::TRANSFERS_CONTROL));
    assert!(!reg.is_frame_sensitive("cd"));
    let brk = reg.get("break").expect("break is registered");
    assert!(brk.traits.contains(Traits::TRANSFERS_CONTROL));
    assert!(!brk.traits.contains(Traits::SAFE_INTERP_HIDDEN));
}
