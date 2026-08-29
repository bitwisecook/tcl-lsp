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

//! The framework front-end: a `.bpftcl` source string → a priority-ordered
//! [`BpfModule`] of typed programs.
//!
//! F5-inspired `when <EVENT> priority N { body }` blocks define programs, but in
//! a *separate* event space: we load the profile-stamped `bpf` registry (not
//! iRules), so `when` flows through as a generic [`Statement::Call`] rather than
//! the F5 `::when::` lowering. We recognise that call, map the event via our own
//! table, and re-lower each handler body independently.

use tcl_compiler::cfg_builder::build_cfg_function;
use tcl_compiler::ir::CommandTokens;
use tcl_compiler::{Script, Statement};
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_registry::bpf_op::{BpfDeclKind, BpfOpKind};
use tcl_registry::model::ingress::static_context_for;

use crate::capability::{CapabilityPolicy, check_policy, collect_policy};
use crate::deploy::resolve_attach;
use crate::diag::{BpfDiag, BpfError};
use crate::event::{event_to_prog_type, known_event_names};
use crate::ir::{BpfModule, BpfProgramDecl, ProgType};
use crate::lower::{lower_function, parse_int};
use crate::profile::{BpfProfileSpec, collect_profile, expand_fields};
use crate::source::lower_bpf_source;
use crate::template::{TemplateDef, collect_templates, expand_uses};
use crate::unroll::unroll_loops;

/// Compile a `.bpftcl` translation unit into a bundle of typed programs.
///
/// # Errors
/// Returns the first [`BpfError`] encountered (bad event, out-of-subset
/// construct, type error, …).
pub fn compile_module(source: &str) -> Result<BpfModule, BpfError> {
    // The BPF profile provides the typed verbs + `when` *and* its exact Tcl 9.0
    // embedding facts. This is deliberately NOT the iRules dialect, and the BPF
    // `when` spec carries no lowering hook, so `when` stays a generic call we
    // re-lower ourselves — a separate event space from F5's `::when::`.
    let registry = bpf_registry();
    let module = lower_bpf_source(source, registry);

    // The (optional) active profile — the top-layer config selected for the file.
    let profile = collect_profile(&module.top_level, registry)?;
    // Reusable parameterised handlers a `use` site can splice in.
    let templates = collect_templates(&module.top_level, registry)?;
    // The capability policy that sandboxes each handler's operations.
    let policy = collect_policy(&module.top_level, registry)?;

    let mut programs = Vec::new();
    let mut saw_when = false;

    for stmt in &module.top_level.statements {
        if let Statement::Call {
            command,
            canonical_command,
            args,
            tokens,
            span,
            ..
        } = stmt
            && decl_kind(
                canonical_command.as_deref().unwrap_or(command.as_str()),
                registry,
            ) == Some(BpfDeclKind::When)
        {
            saw_when = true;
            programs.push(lower_when_decl(
                args,
                tokens.as_ref(),
                *span,
                registry,
                profile.as_ref(),
                &templates,
                &policy,
            )?);
        }
    }

    // When a `when` block is present, only `when` calls and recognised
    // declarations (`is_decl`) are consumed. Any other top-level statement —
    // a stray `drop`, `load16 …`, `set …`, or an unknown command — would be
    // silently dropped from the emitted program with no diagnostic, a silent
    // mishandle. Reject it instead.
    if saw_when {
        for stmt in &module.top_level.statements {
            let is_when = matches!(
                stmt,
                Statement::Call { command, canonical_command, .. }
                    if decl_kind(
                        canonical_command.as_deref().unwrap_or(command.as_str()),
                        registry,
                    ) == Some(BpfDeclKind::When)
            );
            if !is_when && !is_decl(stmt, registry) {
                let (name, span) = stray_stmt_info(stmt);
                return Err(BpfError::new(
                    BpfDiag::StrayTopLevel,
                    span,
                    format!(
                        "top-level `{name}` is not a `when` block or a declaration; \
                         it would be silently dropped — move it inside a `when` handler"
                    ),
                ));
            }
        }
    }

    // No framework envelope: treat the top level (minus profile/field decls) as a
    // single anonymous SOCKET_FILTER program (the raw-DSL path, handy for tests).
    if !saw_when {
        let body = strip_decls(&module.top_level, registry);
        if !body.statements.is_empty() {
            let used = expand_uses(&body, &templates, registry.numbers())?;
            let unrolled = unroll_loops(&used, registry)?;
            let expanded = match profile.as_ref() {
                Some(p) => expand_fields(&unrolled, p)?,
                None => unrolled,
            };
            check_policy(&expanded, &policy, registry)?;
            let cfg = build_cfg_function("main", &expanded, false);
            let program = lower_function(&cfg, ProgType::SocketFilter, registry)?;
            programs.push(BpfProgramDecl {
                event: "SOCKET_FILTER".to_owned(),
                priority: 500,
                program,
                attach: None,
                source_base: 0,
            });
        }
    }

    // Hang the (optional) deployment target off each program it governs.
    resolve_attach(&module.top_level, &mut programs)?;

    // Deterministic order: ascending priority (lower = runs first, F5-style),
    // then event name as a tiebreaker.
    programs.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.event.cmp(&b.event))
    });

    let module = BpfModule { programs };

    // Handler composition (issue #1204): two handlers of the *same* event at the
    // *same* priority have no deterministic order — the priority sort cannot
    // distinguish them and the event-name tiebreaker is identical. That is an
    // ambiguous composition; reject it rather than emit a non-deterministic
    // chain ("sorting independent object files is not sufficient").
    if let Some((event, priority)) = crate::compose::ambiguous_priorities(&module)
        .into_iter()
        .next()
    {
        return Err(BpfError::new(
            BpfDiag::AmbiguousComposition,
            Span::empty(0),
            format!(
                "two `when {event}` handlers share priority {priority}; give each a \
                 distinct priority so their composition order is deterministic"
            ),
        ));
    }

    Ok(module)
}

/// The canonical BPF registry, with the BPF profile and its Tcl 9.0 embedding
/// stamped by the registry owner.
fn bpf_registry() -> &'static CommandRegistry {
    static_context_for("bpf").commands()
}

/// The default handler priority when the `when` header names none
/// (F5-inspired: lower runs first).
const DEFAULT_PRIORITY: u32 = 500;

fn lower_when_decl(
    args: &[String],
    tokens: Option<&CommandTokens>,
    call_span: Span,
    registry: &CommandRegistry,
    profile: Option<&BpfProfileSpec>,
    templates: &[TemplateDef],
    policy: &CapabilityPolicy,
) -> Result<BpfProgramDecl, BpfError> {
    // The span of header word `i`, falling back to the whole call.
    let word_span = |i: usize| {
        tokens
            .and_then(|t| t.argv.get(i).copied())
            .unwrap_or(call_span)
    };

    // Accept exactly `when EVENT { body }` or `when EVENT priority N { body }`
    // — nothing else. A malformed header must never be silently normalised
    // (`when XDP priority nope { … }` must not become priority 500 with no
    // diagnostic).
    let priority = match args.len() {
        2 => DEFAULT_PRIORITY,
        4 => {
            if args[1] != "priority" {
                return Err(BpfError::new(
                    BpfDiag::BadArity,
                    word_span(1),
                    format!(
                        "unknown `when` header keyword `{}` — the only supported \
                         forms are `when EVENT {{ body }}` and \
                         `when EVENT priority N {{ body }}`",
                        args[1]
                    ),
                ));
            }
            if !is_literal_word(&args[2]) {
                return Err(BpfError::new(
                    BpfDiag::BadArity,
                    word_span(2),
                    "the `when` priority must be a literal (no substitutions)",
                ));
            }
            parse_int(&args[2], registry.numbers())
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| {
                    BpfError::new(
                        BpfDiag::BadArity,
                        word_span(2),
                        format!(
                            "invalid `when` priority `{}` — must be a non-negative \
                         integer literal",
                            args[2]
                        ),
                    )
                })?
        }
        _ => {
            return Err(BpfError::new(
                BpfDiag::BadArity,
                call_span,
                "`when` expects: when EVENT { body }  or  when EVENT priority N { body }",
            ));
        }
    };

    let event = args[0].clone();
    if !is_literal_word(&event) {
        return Err(BpfError::new(
            BpfDiag::BadEvent,
            word_span(0),
            "the `when` event must be a literal name (no substitutions)",
        ));
    }
    let prog_type = resolve_event_prog_type(&event, word_span(0))?;

    let body_idx = args.len() - 1;
    let body_text = &args[body_idx];
    // The body word's span starts at the opening brace; +1 skips it so
    // body-relative diagnostic offsets map back into the original file.
    let source_base = tokens
        .and_then(|t| t.argv.get(body_idx).copied())
        .map_or(0, |sp| sp.start() + 1);

    let body_module = lower_bpf_source(body_text, registry);
    let used = expand_uses(&body_module.top_level, templates, registry.numbers())
        .map_err(|e| e.offset(source_base))?;
    let unrolled = unroll_loops(&used, registry).map_err(|e| e.offset(source_base))?;
    let expanded = match profile {
        Some(p) => expand_fields(&unrolled, p).map_err(|e| e.offset(source_base))?,
        None => unrolled,
    };
    check_policy(&expanded, policy, registry).map_err(|e| e.offset(source_base))?;
    let cfg = build_cfg_function(&format!("::bpf::{event}"), &expanded, false);
    let program = lower_function(&cfg, prog_type, registry).map_err(|e| e.offset(source_base))?;

    Ok(BpfProgramDecl {
        event,
        priority,
        program,
        attach: None,
        source_base,
    })
}

/// Resolve a `when` event name to a codegen-ready IR program type, or a precise
/// diagnostic: a registry-described-but-not-yet-compilable event (TC, cgroup) is
/// distinguished from a genuinely unknown one.
fn resolve_event_prog_type(event: &str, span: Span) -> Result<ProgType, BpfError> {
    if let Some(pt) = event_to_prog_type(event) {
        return Ok(pt);
    }
    if crate::event::event_is_described(event) {
        // A real event — say so precisely rather than claiming it is unknown —
        // but the compiler cannot lower it yet.
        return Err(BpfError::new(
            BpfDiag::BadEvent,
            span,
            format!(
                "BPF event `{event}` is registry-described but its codegen is not \
                 yet implemented; compilable events: {}",
                crate::event::codegen_ready_event_names().join(", ")
            ),
        ));
    }
    Err(BpfError::new(
        BpfDiag::BadEvent,
        span,
        format!(
            "unknown BPF event `{event}` (known: {})",
            known_event_names().join(", ")
        ),
    ))
}

/// Whether a `when` header word is a plain literal — no variable or command
/// substitution. The header is static configuration; dynamic events or
/// priorities cannot exist in a verifier-shaped program.
fn is_literal_word(word: &str) -> bool {
    !word.contains('$') && !word.contains('[')
}

/// Remove top-level `profile`/`template`/`allow`/`deny`/`attach` declarations
/// (metadata, macro definitions, or policy — not executable code). A stray
/// `field` outside a `profile` body is deliberately *not* stripped: it flows
/// into lowering, which rejects it with a pointer to where fields belong.
fn strip_decls(script: &Script, registry: &CommandRegistry) -> Script {
    let kept = script
        .statements
        .iter()
        .filter(|s| !is_decl(s, registry))
        .cloned()
        .collect();
    Script::from_statements(kept)
}

/// A display name and span for a stray top-level statement, for the
/// `StrayTopLevel` diagnostic.
fn stray_stmt_info(stmt: &Statement) -> (String, Span) {
    match stmt {
        Statement::Call {
            command,
            canonical_command,
            span,
            ..
        } => (
            canonical_command
                .as_deref()
                .unwrap_or(command.as_str())
                .to_owned(),
            *span,
        ),
        _ => ("statement".to_owned(), stmt.span()),
    }
}

/// The framework-declaration kind of a command name, from its registry
/// descriptor.
fn decl_kind(cmd: &str, registry: &CommandRegistry) -> Option<BpfDeclKind> {
    match registry.bpf_op(cmd).map(|op| op.kind) {
        Some(BpfOpKind::Framework(decl)) => Some(decl),
        _ => None,
    }
}

/// Whether a top-level statement is a consumed framework declaration.
/// Deliberately excluded: `field` (only meaningful inside a `profile` body,
/// so a top-level one is a stray that lowering rejects), `use` (an expansion
/// site, spliced in place by `expand_uses`, not a stripped declaration), and
/// `when` (handled separately).
fn is_decl(stmt: &Statement, registry: &CommandRegistry) -> bool {
    let Statement::Call {
        command,
        canonical_command,
        ..
    } = stmt
    else {
        return false;
    };
    matches!(
        decl_kind(
            canonical_command.as_deref().unwrap_or(command.as_str()),
            registry,
        ),
        Some(
            BpfDeclKind::Profile
                | BpfDeclKind::Template
                | BpfDeclKind::Allow
                | BpfDeclKind::Deny
                | BpfDeclKind::Attach
        )
    )
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{SurfaceQuery, Family};
    use super::*;
    use crate::ir::Term;
    use tcl_dialect::{DialectSet, NumberSyntax, TclVersion};
    use tcl_syntax::number::{runtime_syntax, set_runtime_syntax};

    /// Regression and mutation proof for #1466: loading the BPF command pack
    /// alone is intentionally profile-less. The front end must instead obtain
    /// the cached, profile-stamped registry so every downstream registry fact
    /// describes BPF's exact Tcl 9.0 embedding.
    #[test]
    fn bpf_frontend_uses_the_profile_stamped_tcl90_registry() {
        let registry = bpf_registry();
        let profile = registry.profile().expect("BPF registry has a profile");
        assert_eq!(profile.name, "bpf");
        assert_eq!(
            profile.surface_query(),
            SurfaceQuery::core(Family::Tcl, "9.0").with_packages(&["bpf"])
        );
        assert_eq!(registry.runtime_version(), Some(TclVersion::V9_0));
        assert_eq!(registry.numbers(), NumberSyntax::Tcl90);
        assert_eq!(registry.octal_fold_policy(), Some(false));
        assert!(registry.bpf_op("when").is_some(), "BPF pack is loaded");

        // Mutant: the pre-#1466 construction has the same command pack but no
        // profile, so it cannot carry BPF's release facts through the registry.
        let mut unstamped = CommandRegistry::build_default();
        unstamped.load_bpf();
        assert!(unstamped.profile().is_none());
    }

    /// Every BPF source numeral must read through the BPF profile, never the
    /// ambient grammar a Tcl 8.x interpreter installed on this worker thread.
    #[test]
    fn bpf_source_numbers_ignore_an_ambient_tcl85_runtime() {
        let original_syntax = runtime_syntax();
        set_runtime_syntax(NumberSyntax::Tcl85);

        let cases = [
            ("when priority", "when XDP priority 0d10 { pass }\n"),
            (
                "loop count",
                "when XDP {\n  loop 0o2 index { setint value {$index + 1} }\n  pass\n}\n",
            ),
            (
                "template binding",
                "template init {value} { setint result {$value} }\n\
                 when XDP {\n  use init value=1_000\n  pass\n}\n",
            ),
            (
                "profile field offset and width",
                "profile custom { field first 0d0 0d8 }\nwhen XDP { pass }\n",
            ),
            (
                "expression literal",
                "when XDP {\n  setint value {0d10 + 1_000}\n  pass\n}\n",
            ),
        ];
        let failures: Vec<&str> = cases
            .into_iter()
            .filter_map(|(surface, source)| compile_module(source).err().map(|_| surface))
            .collect();

        set_runtime_syntax(original_syntax);
        assert!(
            failures.is_empty(),
            "BPF Tcl 9.0 numeral grammar was not used by: {}",
            failures.join(", ")
        );
    }

    /// The profile must reach structured conditions in every nested BPF source
    /// body; otherwise `lower_to_ir` reads an ambient Tcl 8.x grammar before
    /// the typed BPF lowerer can apply its target grammar.
    #[test]
    fn bpf_structured_bodies_ignore_an_ambient_tcl85_runtime() {
        let original_syntax = runtime_syntax();
        set_runtime_syntax(NumberSyntax::Tcl85);

        let cases = [
            (
                "top-level structured body",
                "if {0d10} { accept } else { drop }\n",
            ),
            (
                "when body",
                "when XDP {\n  if {0d10} { pass } else { drop }\n}\n",
            ),
            (
                "template body",
                "template branch {} { if {0d10} { pass } else { drop } }\n\
                 when XDP {\n  use branch\n}\n",
            ),
            (
                "loop body",
                "when XDP {\n  loop 0d1 index { if {0d10} { pass } else { drop } }\n}\n",
            ),
        ];
        let failures: Vec<&str> = cases
            .into_iter()
            .filter_map(|(surface, source)| compile_module(source).err().map(|_| surface))
            .collect();

        set_runtime_syntax(original_syntax);
        assert!(
            failures.is_empty(),
            "BPF Tcl 9.0 structured lowering inherited Tcl 8.5 for: {}",
            failures.join(", ")
        );
    }

    #[test]
    fn accept_all_compiles_to_one_program() {
        let src = "when SOCKET_FILTER priority 100 {\n  setbuf pkt ctx\n  accept\n}\n";
        let module = compile_module(src).expect("should compile");
        assert_eq!(module.programs.len(), 1);
        let decl = &module.programs[0];
        assert_eq!(decl.event, "SOCKET_FILTER");
        assert_eq!(decl.priority, 100);
        // Entry block ends in a Return (the `accept`).
        let entry = &decl.program.blocks[0];
        assert!(matches!(entry.term, Term::Return { .. }));
    }

    #[test]
    fn drop_all_raw_dsl() {
        let module = compile_module("drop\n").expect("should compile");
        assert_eq!(module.programs.len(), 1);
        assert_eq!(module.programs[0].event, "SOCKET_FILTER");
    }

    #[test]
    fn stray_top_level_statement_after_when_is_rejected() {
        // A stray top-level statement alongside a `when` block
        // must not be silently dropped — it is a hard error.
        let err =
            compile_module("when SOCKET_FILTER { setbuf pkt ctx\n accept }\ndrop\n").unwrap_err();
        assert_eq!(err.code, BpfDiag::StrayTopLevel);
        assert!(
            err.msg.contains("drop"),
            "message names the stray stmt: {}",
            err.msg
        );
    }

    #[test]
    fn declarations_alongside_when_are_still_accepted() {
        // A recognised declaration (`profile`) beside a `when` block is fine.
        let src = "profile xdp\nwhen XDP { setbuf pkt ctx\n accept }\n";
        // Either compiles or fails for an unrelated reason — but NOT StrayTopLevel.
        if let Err(e) = compile_module(src) {
            assert_ne!(
                e.code,
                BpfDiag::StrayTopLevel,
                "a declaration must not be treated as stray: {}",
                e.msg
            );
        }
    }

    #[test]
    fn plain_set_is_rejected() {
        let err = compile_module("when SOCKET_FILTER { set x 5\n accept }\n").unwrap_err();
        assert_eq!(err.code, BpfDiag::OutOfSubset);
    }

    #[test]
    fn unknown_event_is_rejected() {
        let err = compile_module("when WAT { accept }\n").unwrap_err();
        assert_eq!(err.code, BpfDiag::BadEvent);
    }

    #[test]
    fn every_registry_described_event_now_compiles() {
        // Issue #1310: TC and cgroup codegen landed, so every event the
        // registry describes is codegen-ready — `resolve_event_prog_type`'s
        // "described, but codegen not ready" branch
        // (`BpfDiag::BadEvent` naming the event precisely rather than
        // claiming it unknown) has no live event to fire on today. It stays
        // reachable generically (dispatched off `BpfCodegen::Described`, not
        // a hardcoded event name) for whenever a future event's schema lands
        // ahead of its codegen, as TC/cgroup's did.
        // `drop` (not `pass`) is the one verdict every current program type
        // accepts (a socket filter has no `pass` — it uses `accept`).
        for name in crate::event::known_event_names() {
            let src = format!("when {name} {{ drop }}\n");
            compile_module(&src).unwrap_or_else(|e| panic!("{name} should compile: {}", e.msg));
        }
    }

    #[test]
    fn next_is_a_valid_handler_outcome() {
        // A handler may end a path with `next` (explicit continuation).
        let src = "when XDP { setbuf p ctx\n pktlen n p\n if {$n < 20} { next }\n pass }\n";
        let module = compile_module(src).expect("next compiles");
        assert_eq!(module.programs.len(), 1);
    }

    #[test]
    fn two_handlers_sharing_a_priority_are_ambiguous() {
        // Same event, same explicit priority -> non-deterministic composition.
        let src = "when XDP priority 10 { pass }\nwhen XDP priority 10 { drop }\n";
        let err = compile_module(src).unwrap_err();
        assert_eq!(err.code, BpfDiag::AmbiguousComposition);
    }

    #[test]
    fn two_handlers_with_distinct_priorities_compose() {
        let src = "when XDP priority 10 { next }\nwhen XDP priority 20 { pass }\n";
        let module = compile_module(src).expect("distinct priorities compose");
        // Lower priority first.
        assert_eq!(module.programs[0].priority, 10);
        assert_eq!(module.programs[1].priority, 20);
        let chains = crate::compose::event_chains(&module);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].handler_indices, vec![0, 1]);
    }

    #[test]
    fn port_filter_compiles() {
        let src = "when SOCKET_FILTER {\n\
                   setbuf pkt ctx\n\
                   pktlen len pkt\n\
                   if {$len < 36} { accept }\n\
                   load16 dport pkt 36\n\
                   if {$dport == 22} { drop }\n\
                   accept\n\
                   }\n";
        let module = compile_module(src).expect("port filter should compile");
        assert_eq!(module.programs.len(), 1);
        assert!(module.programs[0].program.blocks.len() >= 3);
    }

    /// Registry/lowering drift gate (issue #1202 acceptance criterion): every
    /// BPF-dialect command carries a typed `bpf_op` descriptor, and the
    /// lowerer/capability policy dispatch on that descriptor — never on a
    /// command-name switch. This walks the *actual* registered BPF command set
    /// and proves each command is describable, so a newly registered command
    /// without a descriptor fails here rather than being silently mishandled.
    #[test]
    fn every_registered_bpf_command_has_a_descriptor() {
        use tcl_registry::bpf_op::BpfOpKind;
        let registry = bpf_registry();
        let bpf_names: Vec<&str> = registry
            .command_names()
            .filter(|n| registry.bpf_op(n).is_some())
            .collect();
        assert!(
            bpf_names.len() >= 25,
            "expected the full BPF command surface, found {}",
            bpf_names.len()
        );
        for name in bpf_names {
            let op = registry.bpf_op(name).expect("descriptor present");
            // The classification the capability policy relies on must be
            // internally consistent: a verdict terminates, a gated op has a
            // packet/map effect, and the two are disjoint.
            match op.kind {
                BpfOpKind::Verdict(_) => assert!(
                    op.effects.is_verdict() && !op.effects.is_gated(),
                    "{name} is a verdict but misclassified"
                ),
                BpfOpKind::PacketLoad { .. }
                | BpfOpKind::PacketLen
                | BpfOpKind::MapGet
                | BpfOpKind::MapHas
                | BpfOpKind::MapSet => assert!(
                    op.effects.is_gated(),
                    "{name} accesses packet/map but is not gated"
                ),
                _ => {}
            }
        }
    }

    /// Every codegen-ready event resolvable through the registry maps to a
    /// program type and the framework front-end compiles a handler for it —
    /// proving the event space is registry-described end to end (no
    /// string-match arm). Schema-described-but-not-ready events (TC, cgroup)
    /// resolve their spec and are rejected with a *precise* diagnostic, never
    /// treated as unknown.
    #[test]
    fn every_registry_event_is_handled_by_its_schema() {
        for event in tcl_registry::bpf_op::BPF_EVENTS {
            if event.is_codegen_ready() {
                assert!(
                    crate::event::event_to_prog_type(event.name).is_some(),
                    "ready event {} has no program type",
                    event.name
                );
                // Its default verdict's verb is a valid handler body.
                let verb = event.default_verdict.verb();
                let src = format!("when {} {{ {verb} }}\n", event.name);
                compile_module(&src)
                    .unwrap_or_else(|e| panic!("event {} handler failed: {e}", event.name));
            } else {
                assert!(
                    crate::event::event_is_described(event.name),
                    "non-ready event {} should be described",
                    event.name
                );
                let src = format!(
                    "when {} {{ {} }}\n",
                    event.name,
                    event.default_verdict.verb()
                );
                let err = compile_module(&src).expect_err("described event must not compile yet");
                assert_eq!(err.code, BpfDiag::BadEvent);
                assert!(
                    err.msg.contains("codegen"),
                    "described-event message should mention codegen: {}",
                    err.msg
                );
            }
        }
    }
}
