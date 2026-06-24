//! The framework front-end: a `.bpftcl` source string → a priority-ordered
//! [`BpfModule`] of typed programs.
//!
//! F5-inspired `when <EVENT> priority N { body }` blocks define programs, but in
//! a *separate* event space: we load a vanilla `build_default()` registry (no
//! iRules), so `when` flows through as a generic [`Statement::Call`] rather than
//! the F5 `::when::` lowering. We recognise that call, map the event via our own
//! table, and re-lower each handler body independently.

use tcl_compiler::cfg_builder::build_cfg_function;
use tcl_compiler::ir::CommandTokens;
use tcl_compiler::lowering::lower_to_ir;
use tcl_compiler::{Script, Statement};
use tcl_lexer::Span;
use tcl_registry::registry::CommandRegistry;

use crate::diag::{BpfDiag, BpfError};
use crate::event::{KNOWN_EVENTS, event_to_prog_type};
use crate::ir::{BpfModule, BpfProgramDecl, ProgType};
use crate::lower::lower_function;
use crate::profile::{BpfProfileSpec, collect_profile, expand_fields};
use crate::template::{TemplateDef, collect_templates, expand_uses};
use crate::unroll::unroll_loops;

/// Compile a `.bpftcl` translation unit into a bundle of typed programs.
///
/// # Errors
/// Returns the first [`BpfError`] encountered (bad event, out-of-subset
/// construct, type error, …).
pub fn compile_module(source: &str) -> Result<BpfModule, BpfError> {
    // Load the BPF dialect (the typed verbs + `when`). This is deliberately NOT
    // the iRules dialect, and the BPF `when` spec carries no lowering hook, so
    // `when` stays a generic call we re-lower ourselves — a separate event space
    // from F5's `::when::`.
    let mut registry = CommandRegistry::build_default();
    registry.load_bpf();
    let module = lower_to_ir(source, &registry);

    // The (optional) active profile — the top-layer config selected for the file.
    let profile = collect_profile(&module.top_level, &registry)?;
    // Reusable parameterised handlers a `use` site can splice in.
    let templates = collect_templates(&module.top_level, &registry)?;

    let mut programs = Vec::new();
    let mut saw_when = false;

    for stmt in &module.top_level.statements {
        if let Statement::Call {
            command,
            args,
            tokens,
            span,
            ..
        } = stmt
            && command == "when"
        {
            saw_when = true;
            programs.push(lower_when_decl(
                args,
                tokens.as_ref(),
                *span,
                &registry,
                profile.as_ref(),
                &templates,
            )?);
        }
    }

    // No framework envelope: treat the top level (minus profile/field decls) as a
    // single anonymous SOCKET_FILTER program (the raw-DSL path, handy for tests).
    if !saw_when {
        let body = strip_decls(&module.top_level);
        if !body.statements.is_empty() {
            let used = expand_uses(&body, &templates)?;
            let unrolled = unroll_loops(&used, &registry)?;
            let expanded = match profile.as_ref() {
                Some(p) => expand_fields(&unrolled, p)?,
                None => unrolled,
            };
            let cfg = build_cfg_function("main", &expanded, false);
            let program = lower_function(&cfg, ProgType::SocketFilter)?;
            programs.push(BpfProgramDecl {
                event: "SOCKET_FILTER".to_owned(),
                priority: 500,
                program,
                source_base: 0,
            });
        }
    }

    // Deterministic order: ascending priority (lower = runs first, F5-style),
    // then event name as a tiebreaker.
    programs.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.event.cmp(&b.event))
    });

    Ok(BpfModule { programs })
}

fn lower_when_decl(
    args: &[String],
    tokens: Option<&CommandTokens>,
    call_span: Span,
    registry: &CommandRegistry,
    profile: Option<&BpfProfileSpec>,
    templates: &[TemplateDef],
) -> Result<BpfProgramDecl, BpfError> {
    if args.len() < 2 {
        return Err(BpfError::new(
            BpfDiag::BadArity,
            call_span,
            "`when` expects: when EVENT ?priority N? { body }",
        ));
    }

    let event = args[0].clone();
    let prog_type = event_to_prog_type(&event).ok_or_else(|| {
        let espan = tokens
            .and_then(|t| t.argv.first().copied())
            .unwrap_or(call_span);
        BpfError::new(
            BpfDiag::BadEvent,
            espan,
            format!(
                "unknown BPF event `{event}` (known: {})",
                KNOWN_EVENTS.join(", ")
            ),
        )
    })?;

    let mut priority = 500u32;
    if args.len() >= 4
        && args[1] == "priority"
        && let Ok(p) = args[2].parse::<u32>()
    {
        priority = p;
    }

    let body_idx = args.len() - 1;
    let body_text = &args[body_idx];
    // The body word's span starts at the opening brace; +1 skips it so
    // body-relative diagnostic offsets map back into the original file.
    let source_base = tokens
        .and_then(|t| t.argv.get(body_idx).copied())
        .map_or(0, |sp| sp.start() + 1);

    let body_module = lower_to_ir(body_text, registry);
    let used = expand_uses(&body_module.top_level, templates).map_err(|e| e.offset(source_base))?;
    let unrolled = unroll_loops(&used, registry).map_err(|e| e.offset(source_base))?;
    let expanded = match profile {
        Some(p) => expand_fields(&unrolled, p).map_err(|e| e.offset(source_base))?,
        None => unrolled,
    };
    let cfg = build_cfg_function(&format!("::bpf::{event}"), &expanded, false);
    let program = lower_function(&cfg, prog_type).map_err(|e| e.offset(source_base))?;

    Ok(BpfProgramDecl {
        event,
        priority,
        program,
        source_base,
    })
}

/// Remove top-level `profile`/`field`/`template` declarations (metadata or
/// macro definitions, not executable code).
fn strip_decls(script: &Script) -> Script {
    let kept = script
        .statements
        .iter()
        .filter(|s| !is_decl(s))
        .cloned()
        .collect();
    Script::from_statements(kept)
}

fn is_decl(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Call { command, canonical_command, .. }
            if matches!(
                canonical_command.as_deref().unwrap_or(command.as_str()),
                "profile" | "field" | "template"
            )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Term;

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
}
