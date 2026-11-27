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

//! Navigation for **caller-frame variables** — the ones a callee creates in
//! *this* frame through `upvar`, which the frame's own text never assigns.
//!
//! ```tcl
//! proc gridlayoutHasDataSetObj {dts} { upvar 1 $dts dataset; set dataset … }
//! # …
//! gridlayoutHasDataSetObj dataset      ;# creates `dataset` HERE
//! set _dataset $dataset                ;# and this reads it
//! ```
//!
//! Nothing in the caller writes `dataset`, so the ordinary scope-chain walk
//! finds no [`VarDef`](tcl_compiler::analyser::VarDef) for the `$dataset`
//! read.  Without this module the providers *fall through* to bareword
//! resolution and answer with a coincidentally same-named `TclOO` **method**
//! — a wrong-kind conflation, since Tcl's variable and command namespaces are
//! disjoint and a `$`-led token can never denote a method.  Two answers
//! follow from that, and both live here:
//!
//! * **Abstain.** [`substituted_var_read_at`] answers "the cursor is on a
//!   `$name` Tcl really substitutes", so hover / find-references can stop
//!   rather than fall through.  It is deliberately independent of whether a
//!   `VarDef` resolved: the *token kind* is what forbids the fallback.
//! * **Resolve.** [`caller_frame_bindings`] finds the call sites in the
//!   enclosing frame that create the name, so hover, go-to-definition, and
//!   find-references can answer for real.
//!
//! # What the call site has to say
//!
//! Two per-parameter facts, and a binding needs **both**:
//!
//! * [`ProcArgTrait::VarWrite`] / [`ProcArgTrait::VarRead`] — the parameter's
//!   *value* is used as a variable name through an `upvar`, and whether the
//!   callee writes through the alias or only reads it.  This is what
//!   distinguishes a creating call site from a referencing one.
//! * [`ProcDef::caller_frame_params`](tcl_compiler::analyser::ProcDef::caller_frame_params)
//!   — the alias lands in the **immediate caller's** frame.  The traits carry
//!   no frame level at all, and only `upvar 1` (or an omitted level) reaches
//!   the caller: `upvar 0` aliases the callee's *own* frame, `upvar #0` the
//!   global one, `upvar 2` the caller's caller.  Trusting the trait alone
//!   navigated a variable the frame never gains.
//!
//! No command name appears here: which words name variables, and which
//! nested scripts still run in this frame, are registry- and
//! analyser-derived.
//!
//! C Tcl, pinned on tclsh 9.0.4 and 8.6.14 (identical):
//!
//! ```tcl
//! proc setdef {d} { upvar 1 $d dst; set dst SET }
//! proc build {} { setdef options; return $options }
//! build            ;# → SET — `options` exists in build's frame, unassigned there
//!
//! proc p0 {d} { upvar 0 $d dst; set dst SET }
//! proc build0 {} { p0 options; return [info exists options] }
//! build0           ;# → 0 — `upvar 0` aliased p0's OWN local, nothing here
//! ```
//!
//! # Literal caller-frame targets
//!
//! A callee that binds a **literal** caller-side name (`upvar 1 name name`)
//! spells that name nowhere at the call site, so
//! there is no argument word to key on.  The analyser records those names
//! per proc on
//! [`ProcDef::caller_frame_literals`](tcl_compiler::analyser::ProcDef::caller_frame_literals),
//! and [`caller_frame_bindings`] answers for them with the *call-head word*
//! as the binding span — the point where the variable comes to exist in
//! this frame.  A fully-qualified target (`upvar ::tk::FocusGrab($i) data`)
//! is not a caller-frame variable at all: it names one fixed global cell,
//! which the analyser's alias consumer (`apply_state_transitions`) defines
//! and links directly.
//!
//! # Methods reached by `my` dispatch
//!
//! Self-dispatch navigation uses the shared source owner's retained receiver,
//! dispatcher and original method allocation. The original body supplies the
//! symbolic caller-name template. An advisory class name or guessed method
//! resolution order cannot provide a call/frame receipt.
//!
//! # What it deliberately does not answer
//!
//! `next` / `nextto` dispatch to whatever follows *this* implementation in
//! the MRO, which the call site does not name, so those reads keep the
//! abstaining answer rather than a wrong one; the compiler-side dispatch
//! widening keeps the diagnostics honest for them.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::types::ProcArgTrait;
use tcl_lexer::Span;

/// The name of the `$name` read the cursor sits on, when that occurrence is
/// one Tcl actually substitutes.
///
/// The `VarDef`-resolving twin is
/// [`crate::definition::lookup_var_read_at`]; this answers the *token kind*
/// alone, which is what a provider needs to decide whether falling through to
/// bareword (command / class-member) resolution is legitimate.  It never is
/// for a `$`-led read: Tcl keeps variables and commands in disjoint
/// namespaces, so `$dataset` can only ever be the variable, never a method
/// called `dataset`.
#[must_use]
pub(crate) fn substituted_var_read_at(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
    cursor_off: u32,
) -> Option<String> {
    crate::definition::substituting_var_at_position(source, analysis, line, character, cursor_off)
}

/// One selected call in the current scope that instantiates a callee's
/// symbolic caller-name template. This grants no completed runtime store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallerFrameBinding {
    /// Qualified name of the callee whose `upvar` creates the variable.
    pub callee: String,
    /// The callee parameter whose *value* names the variable — or `None`
    /// when the callee spells the name **literally in its own body**
    /// (`upvar 1 name name`), so no call-site word carries it at all.
    pub param: Option<String>,
    /// Span of the call-site word that names it — `dataset` in
    /// `gridlayoutHasDataSetObj dataset`.  This is both the creating write
    /// (go-to-definition's target) and a reference to the variable.  For a
    /// literal-target binding ([`Self::param`] `None`) there is no such
    /// word, so this is the call's command-head word — the point where the
    /// variable comes to exist in this frame.
    pub arg_span: Span,
    /// Span of the call's command-head word.
    pub call_span: Span,
    /// True when the callee template only *reads* through the alias
    /// ([`ProcArgTrait::VarRead`] without
    /// [`ProcArgTrait::VarWrite`]) — the site references the variable but
    /// has no write-through usage. A write template proves no successful store.
    pub read_only: bool,
}

/// Render the independently selected caller-name template, without claiming
/// an entered alias, successful write or completed caller store.
pub(crate) fn caller_frame_hover_text(
    name: &str,
    binding: &crate::caller_frame::CallerFrameBinding,
) -> String {
    let verb = if binding.read_only {
        "named for reading in this frame by"
    } else {
        "named for writing in this frame by"
    };
    match &binding.param {
        Some(param) => format!(
            "**Caller-frame variable** `{name}`\n\n\
             {verb} `{}`, through its `{param}` parameter's `upvar`.\n\n\
             The name is passed at the call site and used by the callee's source template.",
            binding.callee
        ),
        // A literal target (`upvar 1 name name`): the callee
        // spells the name in its own body, so nothing at the call site
        // carries it.
        None => format!(
            "**Caller-frame variable** `{name}`\n\n\
             {verb} `{}`, whose own `upvar` names it literally.\n\n\
             The name is spelled in the callee's source template.",
            binding.callee
        ),
    }
}

/// Byte region of the innermost scope body containing `off`, or the whole
/// document when the cursor is at top level.
///
/// A caller-frame binding is created by a call in the *same frame* as the
/// read, so the search never crosses a proc/method body boundary.
fn enclosing_frame_region(
    global: &tcl_compiler::analyser::Scope,
    off: u32,
    source: &str,
) -> (usize, usize, bool) {
    fn walk(
        scope: &tcl_compiler::analyser::Scope,
        off: u32,
        best: &mut Option<Span>,
    ) -> Option<()> {
        for child in &scope.children {
            if let Some(span) = child.body_span
                && off >= span.start()
                && off <= span.end()
            {
                let better = best.is_none_or(|b: Span| {
                    span.end().saturating_sub(span.start()) < b.end().saturating_sub(b.start())
                });
                if better {
                    *best = Some(span);
                }
            }
            walk(child, off, best)?;
        }
        Some(())
    }
    let mut best = None;
    walk(global, off, &mut best);
    let nested = best.is_some();
    let source_len = source.len();
    let (mut start, mut end) = match best {
        Some(span) => (
            (span.start() as usize).min(source_len),
            (span.end() as usize).min(source_len),
        ),
        None => (0, source_len),
    };
    // A scope's recorded body span can still carry the body word's own
    // delimiters. Left in, the segmenter reads the whole frame as a single
    // braced *word* and finds no commands in it at all, so every lookup here
    // would silently answer "nothing binds this name".
    let bytes = source.as_bytes();
    if nested && start < end && bytes.get(start) == Some(&b'{') {
        start += 1;
    }
    if nested && end > start && bytes.get(end - 1) == Some(&b'}') {
        end -= 1;
    }
    (start, end, nested)
}

/// Whether any procedure in the document takes a parameter whose value names
/// a caller-frame variable — the cheap pre-filter for
/// [`caller_frame_bindings`]'s source scan.
fn document_has_call_by_name_proc(analysis: &AnalysisResult) -> bool {
    let relevant = |proc_def: &tcl_compiler::analyser::ProcDef| {
        !proc_def.caller_frame_params.is_empty() || !proc_def.caller_frame_literals.is_empty()
    };
    if analysis.allows_lexical_declaration_advice() {
        analysis.all_procs.values().any(relevant)
    } else {
        analysis
            .original_procedure_declarations()
            .any(|declaration| relevant(declaration.metadata()))
    }
}

/// Every call in the frame enclosing `cursor_off` that binds `name` in that
/// frame through a callee's `upvar`.
///
/// Returned in source order.  Empty when nothing binds the name — the caller
/// then keeps abstaining rather than falling through.
#[must_use]
pub(crate) fn caller_frame_bindings(
    analysis: &AnalysisResult,
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    resolution: crate::definition::CallResolution<'_>,
    cursor_off: u32,
    name: &str,
) -> Vec<CallerFrameBinding> {
    // Implementation contract: naming.core.original-caller-frame-navigation
    // docs/design/analysis/name-resolution-proofs/original-caller-frame-navigation.md
    let mut out: Vec<CallerFrameBinding> = Vec::new();
    let Some(config) = analysis.body_lexer_config else {
        return out;
    };
    if name.is_empty()
        || !analysis
            .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
    {
        return out;
    }
    let (Some(profile), Some(registry), Some(identities)) = (
        analysis.resolved_profile(),
        analysis.resolved_registry(),
        analysis.retained_command_realm(),
    ) else {
        return out;
    };
    if profile.name != dialect.name
        || resolution.registry.is_some_and(|requested| {
            requested.snapshot().semantic_key() != registry.snapshot().semantic_key()
        })
    {
        return out;
    }
    let lexical = analysis.allows_lexical_declaration_advice();
    let has_class = if lexical {
        crate::definition::enclosing_class_at(analysis, cursor_off).is_some()
    } else {
        analysis.original_class_declarations().next().is_some()
    };
    if !document_has_call_by_name_proc(analysis) && !has_class {
        return out;
    }
    let ctx = BindingScan {
        analysis,
        source,
        dialect: profile,
        registry,
        identities,
        config,
        name,
        read_offset: cursor_off,
    };
    if lexical {
        let (start, end, nested) =
            enclosing_frame_region(&analysis.global_scope, cursor_off, source);
        collect_bindings_in_region(&ctx, start, end, nested, 0, &mut out);
    } else {
        let Some(structure) =
            crate::source_structure::SourceStructure::capture(source, Some(analysis), config)
        else {
            return out;
        };
        // Body and bracket grammar supplies readonly candidates only. The
        // existing original caller template independently joins every call to
        // this read's genuine declared or entered frame and namespace.
        for command in &structure.commands {
            bindings_from_call(&ctx, command, &mut out);
        }
    }
    out.sort_by_key(|b| b.arg_span.start());
    out.dedup();
    out
}

/// The immutable inputs one binding scan threads through its recursion.
struct BindingScan<'a> {
    analysis: &'a AnalysisResult,
    source: &'a str,
    dialect: &'static tcl_dialect::DialectProfile,
    /// Actual retained Registry; procedure identity remains allocation-owned.
    registry: &'a tcl_registry::CommandRegistry,
    /// Complete original lexical configuration, including BOM and overrides.
    config: tcl_lexer::LexerConfig,
    /// The document's proven command-identity facts, built once per scan and
    /// handed to every trait scan below so a rebound head resolves here the
    /// same way it does everywhere else.
    identities: &'a tcl_compiler::realm::CommandBindingRealm,
    name: &'a str,
    read_offset: u32,
}

/// Collect the binding call sites in one script region, then recurse into
/// every **same-frame** script nested in it.
///
/// The frame is not the region's outer command list: `if {$ok} { setdef x }`
/// runs `setdef` in the very frame the `if` is written in, so the variable it
/// creates through `upvar` belongs here and navigation must find it.  Which
/// nested arguments are same-frame is the registry's answer, not a keyword
/// list — [`crate::references::nested_dispatch_regions`] is the existing
/// walker for exactly this question (`ArgRole::Body` gated on a `Plain`
/// [`tcl_registry::BodyKind`], plus `[…]` substitutions, plus `switch`-style
/// clause lists via the registry's own `CaseListSpec`).  Reusing it means a
/// spec change reaches this scan too, and `Structural` bodies — `proc`,
/// `namespace eval`, `uplevel`, `oo::define` — plus `apply`'s
/// `LambdaLiteral` stay excluded, which is what the fresh-frame boundary
/// requires: a `setdef x` inside a nested `proc` body creates *that* proc's
/// variable, never this frame's.
///
/// `depth` uses the same [`crate::references::MAX_DISPATCH_SCAN_DEPTH`] guard
/// the sibling dispatch scans use.
fn collect_bindings_in_region(
    ctx: &BindingScan<'_>,
    start: usize,
    end: usize,
    nested: bool,
    depth: u32,
    out: &mut Vec<CallerFrameBinding>,
) {
    if start >= end
        || end > ctx.source.len()
        || crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth)
    {
        return;
    }
    let commands = binding_region_commands(ctx.source, start, end, ctx.config, nested);
    for cmd in &commands {
        bindings_from_call(ctx, cmd, out);
        for (inner_start, inner_end) in
            crate::references::nested_dispatch_regions(ctx.source, ctx.analysis, ctx.dialect, cmd)
        {
            collect_bindings_in_region(ctx, inner_start, inner_end, true, depth + 1, out);
        }
    }
}

/// Local script purpose is selected by the actual enclosing frame or body.
/// The parent `BindingScan` config remains unchanged for original token joins.
fn binding_region_commands(
    source: &str,
    start: usize,
    end: usize,
    config: tcl_lexer::LexerConfig,
    nested: bool,
) -> Vec<tcl_source::segment::SegmentedCommand> {
    tcl_source::segment::segment_script_image_with_offset_and_config(
        &tcl_lexer::SourceImage::document(&source[start..end]),
        u32::try_from(start).unwrap_or(0),
        if nested { config.nested() } else { config },
    )
    .map_or_default(tcl_source::segment::SegmentedScript::into_editor_commands)
}

/// The bindings one call site contributes — at most one per parameter whose
/// actual argument is the bare name being navigated.
fn bindings_from_call(
    ctx: &BindingScan<'_>,
    cmd: &tcl_source::segment::SegmentedCommand,
    out: &mut Vec<CallerFrameBinding>,
) {
    let Some(head) = cmd.argv.first() else {
        return;
    };
    let registry = ctx.registry;
    let tokens = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(ctx.source),
        ctx.config,
        cmd,
    );
    let Some(template) =
        ctx.identities
            .caller_frame_invocation_template_at(&tokens, ctx.read_offset, registry)
    else {
        let declared =
            ctx.analysis
                .original_declared_self_method_template(ctx.source, cmd, ctx.read_offset);
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_CALLER_FRAME").is_some() {
            eprintln!(
                "ORIGINAL_DECLARED_MY offset={} template={}",
                head.span.start(),
                declared.is_some(),
            );
        }
        if let Some(template) = declared
            && template.invocation().matches_registry(registry)
        {
            bindings_from_declared_self_dispatch(ctx, cmd, &template, out);
        }
        return;
    };
    let Some(definition) = template.procedure_definition() else {
        bindings_from_self_dispatch(ctx, cmd, &template, out);
        return;
    };
    let Some(proc_def) = ctx.analysis.proc_for_definition(definition, ctx.source) else {
        return;
    };
    if proc_def.caller_frame_params.is_empty() && proc_def.caller_frame_literals.is_empty() {
        return;
    }
    // A literal caller-frame target (`upvar 1 name name`): the
    // callee spells the name in its own body, so the *call itself* is the
    // binding — no argument word to key on.  The command-head word stands in
    // as the binding span: it is the point where the variable comes to exist
    // in this frame (tclsh 9.0.4 / 8.6.14: `proc np {} {upvar name name;
    // set name W1}` then `np; puts $name` prints `W1`).
    if let Some(written) = proc_def.caller_frame_literals.get(ctx.name) {
        out.push(CallerFrameBinding {
            callee: proc_def.qualified_name.clone(),
            param: None,
            arg_span: head.span,
            call_span: head.span,
            read_only: !written,
        });
    }
    for (i, param) in proc_def.params.iter().enumerate() {
        let Some((argument, value)) = template.literal_parameter_argument(i) else {
            continue;
        };
        let Some(arg_tok) = cmd.argv.get(argument + 1) else {
            continue;
        };
        if value != ctx.name {
            continue;
        }
        // The trait alone is not enough. `VarWrite` / `VarRead` say the
        // parameter's value is used as a variable *name* through an `upvar`,
        // but never which frame the alias lands in — and only `upvar 1` lands
        // in the caller's. tclsh 9.0.4 and 8.6.14 agree exactly: with `proc q
        // {n} {upvar 1 $n a; set a 1}` the caller's variable exists after
        // `q y`; with `upvar 0` / `upvar #0` / `upvar 2` in its place it
        // never does. A binding claimed from the trait alone would navigate a
        // variable this frame does not have.
        // `ProcDef::caller_frame_params` is that missing level fact.
        if !proc_def.caller_frame_params.contains(&param.name) {
            continue;
        }
        let Some(traits) = proc_def.param_traits.get(&param.name) else {
            continue;
        };
        let writes = traits.contains(&ProcArgTrait::VarWrite);
        let reads = traits.contains(&ProcArgTrait::VarRead);
        if !writes && !reads {
            continue;
        }
        out.push(CallerFrameBinding {
            callee: proc_def.qualified_name.clone(),
            param: Some(param.name.clone()),
            arg_span: arg_tok.span,
            call_span: head.span,
            read_only: !writes,
        });
    }
}

/// The bindings a **`TclOO` self-dispatch** call site contributes —
/// `my NameProcess …` (`SpiceGenTcl`'s `Utility::NameProcess`, mixed into
/// the class and invoked from its constructor).
///
/// tclsh 9.0.4 / 8.6.16, identical — the mixin's `upvar name name` really
/// does create the *constructor's* `name`, which the constructor's own text
/// never assigns:
///
/// ```text
/// oo::class create Utility { method NameProcess {arguments object} {
///     upvar name name; set name $object } }
/// oo::class create Widget { mixin Utility
///     constructor {arguments} { my NameProcess $arguments [self object]
///         puts "name=$name" } }
/// Widget new {-base 1}      → name=::oo::Obj24 …
/// ```
///
/// The invocation template retains the selected method declaration and its
/// original source. Trait scans use that source and the retained identities;
/// argument projection comes from the same caller-frame template.
fn bindings_from_self_dispatch(
    ctx: &BindingScan<'_>,
    cmd: &tcl_source::segment::SegmentedCommand,
    template: &tcl_compiler::command_binding::SourceCallerFrameInvocationTemplate,
    out: &mut Vec<CallerFrameBinding>,
) {
    let Some(entry) = template.method() else {
        return;
    };
    let Some(class) = entry.declaring_class() else {
        return;
    };
    let names = entry
        .formals()
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    scan_self_method_body(
        ctx,
        cmd,
        entry.body(),
        &tcl_syntax::naming::rooted_member_key(&class.command, entry.name()),
        &names,
        |i| template.literal_parameter_argument(i),
        out,
    );
}

fn bindings_from_declared_self_dispatch(
    ctx: &BindingScan<'_>,
    cmd: &tcl_source::segment::SegmentedCommand,
    template: &tcl_compiler::command_binding::OriginalDeclaredSelfMethodTemplate,
    out: &mut Vec<CallerFrameBinding>,
) {
    // naming.tcloo.original-declared-self-method-caller-template
    // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-self-method-caller-template.md
    let names = template
        .formals()
        .parameters()
        .iter()
        .map(|formal| core::str::from_utf8(&formal.name))
        .collect::<Result<Vec<_>, _>>();
    let Ok(names) = names else {
        return;
    };
    scan_self_method_body(
        ctx,
        cmd,
        template.callee_body().source(),
        template.display_name(),
        &names,
        |i| template.literal_parameter_argument(i),
        out,
    );
}

fn scan_self_method_body<'a>(
    ctx: &BindingScan<'_>,
    cmd: &tcl_source::segment::SegmentedCommand,
    script: &tcl_compiler::command_binding::ExecutedScriptSource,
    callee: &str,
    param_names: &[&str],
    literal_argument: impl Fn(usize) -> Option<(usize, &'a str)>,
    out: &mut Vec<CallerFrameBinding>,
) {
    use tcl_compiler::analyser::param_traits::{
        ShallowProcedureTraits, TraitScanEnv, infer_param_traits_with_caller_frame,
    };
    let Some(head) = cmd.argv.first() else {
        return;
    };
    let Ok(body) = script.text.try_text() else {
        return;
    };
    let registry = ctx.registry;
    let env = TraitScanEnv {
        surface: tcl_registry::model::DocumentCommandSurface::new(registry, None),
        config: ctx.config,
        identities: ctx.identities,
        executed_source: Some(script),
        original_logical_body: None,
    };
    let ShallowProcedureTraits {
        traits,
        caller_frame_params,
        caller_frame_literals: targets,
    } = infer_param_traits_with_caller_frame(param_names, body, env);
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_CALLER_FRAME").is_some() {
        eprintln!(
            "ORIGINAL_DECLARED_MY body={} source_base={} literal_targets={targets:?}",
            body.len(),
            script.base(),
        );
    }
    if let Some(written) = targets.get(ctx.name) {
        out.push(CallerFrameBinding {
            callee: callee.to_owned(),
            param: None,
            arg_span: head.span,
            call_span: head.span,
            read_only: !written,
        });
    }
    if caller_frame_params.is_empty() {
        return;
    }
    for (i, param) in param_names.iter().enumerate() {
        // `my <method> <arg>…` — the actual arguments start one word later
        // than a plain call's, because the method name is itself a word.
        let Some((argument, value)) = literal_argument(i) else {
            continue;
        };
        let Some(arg_tok) = cmd.argv.get(argument + 1) else {
            continue;
        };
        if value != ctx.name {
            continue;
        }
        // Same two-fact rule as the plain-proc path: the trait says the
        // value is used as a variable name, `caller_frame_params` says the
        // alias lands one frame up (`upvar 1`, not `0` / `#0` / `2`).
        if !caller_frame_params.contains(*param) {
            continue;
        }
        let Some(traits) = traits.get(*param) else {
            continue;
        };
        let writes = traits.contains(&ProcArgTrait::VarWrite);
        let reads = traits.contains(&ProcArgTrait::VarRead);
        if !writes && !reads {
            continue;
        }
        out.push(CallerFrameBinding {
            callee: callee.to_owned(),
            param: Some((*param).to_string()),
            arg_span: arg_tok.span,
            call_span: head.span,
            read_only: !writes,
        });
    }
}

/// Every span in the enclosing frame that refers to the caller-frame variable
/// `name`: each call-site word that names it, plus each `$name` read.
///
/// Both halves are the point of the idiom — the caller writes the name once,
/// bare, at the call site and then reads it with a `$`, so a reference set
/// that showed only one of the two would miss what the user is looking for.
/// Returns empty when no call site binds the name, so a provider that gets
/// nothing here keeps abstaining.
#[must_use]
pub(crate) fn caller_frame_reference_spans(
    analysis: &AnalysisResult,
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    resolution: crate::definition::CallResolution<'_>,
    cursor_off: u32,
    name: &str,
) -> Vec<Span> {
    let bindings = caller_frame_bindings(analysis, source, dialect, resolution, cursor_off, name);
    caller_frame_reference_spans_from_bindings(analysis, source, cursor_off, name, &bindings)
}

/// Project references from an already selected readonly caller template.
/// Selection and frame correspondence are not reconstructed by providers.
pub(crate) fn caller_frame_reference_spans_from_bindings(
    analysis: &AnalysisResult,
    source: &str,
    cursor_off: u32,
    name: &str,
    bindings: &[CallerFrameBinding],
) -> Vec<Span> {
    if bindings.is_empty()
        || crate::original_context::CurrentSourceContext::capture(source, analysis).is_none()
    {
        return Vec::new();
    }
    let mut spans: Vec<Span> = bindings.iter().map(|b| b.arg_span).collect();
    if analysis.allows_lexical_declaration_advice() {
        let (start, end, _) = enclosing_frame_region(&analysis.global_scope, cursor_off, source);
        spans.extend(substituted_read_spans(
            source, analysis, start, end, cursor_off, name,
        ));
    } else {
        spans.extend(original_caller_read_spans(analysis, source, bindings, name));
    }
    spans.sort_by_key(|span: &Span| span.start());
    spans.dedup();
    spans
}

/// Lexical original variable roots supply read geometry; the caller template
/// independently joins each read to one of the selected call/frame recipes.
/// Reporting scopes, text search and standalone catalogues supply no Native
/// frame or reference coverage through this projection.
fn original_caller_read_spans(
    analysis: &AnalysisResult,
    source: &str,
    bindings: &[CallerFrameBinding],
    name: &str,
) -> Vec<Span> {
    use tcl_compiler::signature_scan::scope::SignatureSourceNameInput;
    let (Some(config), Some(registry), Some(realm)) = (
        analysis.body_lexer_config,
        analysis.resolved_registry(),
        analysis.retained_command_realm(),
    ) else {
        return Vec::new();
    };
    let Some(structure) =
        crate::source_structure::SourceStructure::capture(source, Some(analysis), config)
    else {
        return Vec::new();
    };
    let calls = structure
        .commands
        .iter()
        .filter(|command| {
            command.argv.first().is_some_and(|head| {
                bindings
                    .iter()
                    .any(|binding| binding.call_span == head.span)
            })
        })
        .map(|command| {
            tcl_compiler::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                config,
                command,
            )
        })
        .collect::<Vec<_>>();
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .original_variable_symbols
        .iter()
        .filter_map(|occurrence| {
            let SignatureSourceNameInput::OriginalVariableRoot(root) =
                occurrence.original_name_input()
            else {
                return None;
            };
            let span = root.name_span()?;
            if root.source_image() != &image
                || root.lexer_config() != config
                || source.get(span.as_range())? != name
                || !calls.iter().any(|tokens| {
                    realm
                        .caller_frame_invocation_template_at(tokens, span.start(), registry)
                        .is_some()
                })
            {
                return None;
            }
            Some(span)
        })
        .collect()
}

/// Whether the `$` at byte `at` is **escaped** by the backslash run
/// immediately before it, and so introduces no substitution.
///
/// Tcl's rule is the run's parity, not merely "is the previous byte a
/// backslash": each `\\` pair is itself an escaped backslash and leaves the
/// `$` free.  Pinned identical on tclsh 9.0.4 and 8.6.14 with `name` set to
/// `SET` through the very `upvar` idiom this module navigates:
///
/// | source | result | `$` substitutes? |
/// |---|---|---|
/// | `"$name"` | `SET` | yes |
/// | `"\$name"` | `$name` | no |
/// | `"\\$name"` | `\SET` | **yes** |
/// | `"\\\$name"` | `\$name` | no |
/// | `"\\\\$name"` | `\\SET` | **yes** |
/// | `"\${name}"` | `${name}` | no |
/// | `"\\${name}"` | `\SET` | **yes** |
///
/// Counting only the immediately-preceding byte would call the even-run rows
/// escaped and silently drop real references.
#[cfg(test)]
fn dollar_is_escaped(bytes: &[u8], at: usize) -> bool {
    let run = bytes[..at]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count();
    run % 2 == 1
}

/// Actual scalar substitutions in the selected original caller source frame.
/// Original components own escaping, inertness and complete name geometry;
/// array roots cannot supply a same-named scalar read.
fn substituted_read_spans(
    source: &str,
    analysis: &AnalysisResult,
    start: usize,
    end: usize,
    cursor_off: u32,
    name: &str,
) -> Vec<Span> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    let frame = crate::definition::source_frame_extent(analysis, cursor_off);
    let Some(walk) = crate::refactor::FrameWalk::new(source, analysis) else {
        return Vec::new();
    };
    let config = current.config();
    let mut out = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            let Some(head) = command.argv.first() else {
                return false;
            };
            if crate::definition::source_frame_extent(analysis, head.span.start()) != frame {
                return false;
            }
            let Some(arenas) = walk.components(source, command) else {
                return false;
            };
            for arena in arenas {
                for part in arena.all_parts() {
                    if !matches!(part.part, tcl_lexer::ExecutablePart::Variable { .. })
                        || usize::try_from(part.span.start()).map_or(true, |at| at < start)
                        || usize::try_from(part.span.end()).map_or(true, |at| at > end)
                    {
                        continue;
                    }
                    let Some(spelling) = source.get(part.span.as_range()) else {
                        continue;
                    };
                    if tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
                        != Some(name)
                    {
                        continue;
                    }
                    let Some(reference) = tcl_lexer::word_parts::scan_var_ref(
                        source.as_bytes(),
                        part.span.as_range().start,
                        config,
                    )
                    .ok()
                    .flatten() else {
                        continue;
                    };
                    if u32::try_from(reference.next).ok() != Some(part.span.end()) {
                        continue;
                    }
                    let Some(range) = reference.name_range_in(source.as_bytes()) else {
                        continue;
                    };
                    let (Ok(first), Ok(last)) =
                        (u32::try_from(range.start), u32::try_from(range.end))
                    else {
                        continue;
                    };
                    out.push(Span::new(first, last));
                }
            }
            false
        },
    );
    out.sort_by_key(|span| (span.start(), span.end()));
    out.dedup();
    out
}

/// The caller-frame variable the cursor's **bare** word names, when that word
/// is a call-site argument binding one.
///
/// The bareword half of the idiom — `gridlayoutHasDataSetObj dataset` — is a
/// creating write, so hover, go-to-definition, and find-references must all
/// answer for it exactly as they do for the `$dataset` reads it feeds.
///
/// A **literal-target** binding is deliberately excluded: its `arg_span` is
/// the call's command-head word, and a bare cursor on the command name must
/// keep resolving as the *command* (definition/references of the proc), not
/// silently become a variable.
#[must_use]
pub(crate) fn binding_at_offset(
    analysis: &AnalysisResult,
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    resolution: crate::definition::CallResolution<'_>,
    cursor_off: u32,
    word: &str,
) -> Option<CallerFrameBinding> {
    caller_frame_bindings(analysis, source, dialect, resolution, cursor_off, word)
        .into_iter()
        .filter(|b| b.param.is_some())
        .find(|b| cursor_off >= b.arg_span.start() && cursor_off <= b.arg_span.end())
}

/// The binding a caller-frame *read* resolves to — the first call site that
/// **creates** the variable, falling back to the first that references it.
///
/// Go-to-definition and hover both want the creating write: it is the nearest
/// thing the frame has to a declaration, and it is the word the user would
/// rename.
#[must_use]
pub(crate) fn primary_binding(bindings: &[CallerFrameBinding]) -> Option<&CallerFrameBinding> {
    bindings
        .iter()
        .find(|b| !b.read_only)
        .or_else(|| bindings.first())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod caller_frame_navigation_tests;

#[cfg(test)]
mod original_binding_region_purpose_tests;

#[cfg(test)]
mod original_logical_caller_read_geometry_tests;
