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

//! External `$obj method` and class-command call sites, matched through their receivers.

use super::dispatch_regions::{
    MAX_DISPATCH_SCAN_DEPTH, frame_shifted_dispatch_regions, nested_dispatch_regions,
    strip_outer_braces,
};
use crate::definition::class_definition_grammar;
use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::AnalysisResult;

/// The external `$obj method` / bare `objcmd method` call sites for `method`
/// on instances of `class_q` **within `source`**, independent of whether
/// `class_q` is *defined* in this document.
///
/// The building block for cross-file references from a **pure-consumer**
/// document — one that only creates and uses instances of a class defined
/// elsewhere (`set d [::other::Cls new]; $d method`).  Such a document defines
/// no class body, so [`method_reference_spans_in_document`] /
/// [`inherited_method_spans_in_document`] (which key off a local class body)
/// find nothing; this keys off `instance_classes`, which the cross-file
/// analysis populates from the workspace class set.
///
/// `classmethod_class_names` carries the caller's *workspace-wide* knowledge
/// of which class names are valid bare-dispatch heads for `method` when it is
/// a `classmethod` — a pure-consumer document has no local `ClassDef` for
/// `class_q` to derive this from (unlike the same-document path, whose
/// [`method_references_for_class`] derives it from `analysis.all_classes`
/// directly), so the caller supplies it from the workspace index's
/// [`WorkspaceMethod`](crate::workspace_index::WorkspaceMethod) `kind`
/// instead.  Empty when `method` is not a classmethod, or the caller has no
/// workspace index.
#[must_use]
pub fn obj_method_call_sites(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
    classmethod_class_names: &[String],
) -> Vec<tcl_lexer::Span> {
    find_obj_method_call_sites_with_extra_cmd_names(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
        classmethod_class_names,
    )
}

/// Direct class-command selector sites in the actual retained Logical source.
/// The shared original class-call owner supplies factory/name lineage and full
/// context; this query grants neither Native receiver identity nor dispatch.
/// Captured selector prefixes cannot donate a written call-site range.
pub(crate) fn logical_class_command_method_sites(
    source: &str,
    analysis: &AnalysisResult,
    receiver: &tcl_compiler::analyser::ClassDef,
    method: &str,
) -> Vec<tcl_lexer::Span> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    if !analysis.allows_retained_logical_declaration_advice() {
        return Vec::new();
    }
    let image = tcl_lexer::SourceImage::document(source);
    let context = current.context();
    let mut seen = FxHashSet::default();
    analysis
        .command_invocations
        .iter()
        .filter_map(|invocation| {
            if !invocation.lookup.is_execution_site() {
                return None;
            }
            let call =
                tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
                    source,
                    analysis,
                    invocation.range.start(),
                )?;
            if !call.matches_source_context(&image, current.config(), &context)
                || !std::ptr::eq(
                    call.class_declaration().logical_source_class(analysis)?,
                    receiver,
                )
                || call.arguments().first()?.literal_bytes()? != method.as_bytes()
            {
                return None;
            }
            let written = call.written_argument(0)?;
            let selector = call.argument_word(0)?;
            if call.original_words().get(written + 1) != Some(selector)
                || selector.tokens().len() != 1
            {
                return None;
            }
            let span = selector.tokens()[0].span;
            seen.insert((span.start(), span.end())).then_some(span)
        })
        .collect()
}

/// Find external object and class method reads whose temporal receiver entry
/// identifies the desired original declaration. Candidate class maps and
/// command spellings cannot supply editable reference authority.
/// Returns spans of the written method-name tokens.
///
/// Scans three region kinds — the top-level command stream,
/// each user proc body, and each class method body — and
/// recurses into command-substitution (`[...]`) args at every
/// level.  This covers the common call forms (`$d bark`,
/// `puts [$d bark]`, calls inside procs / methods).  Method
/// names embedded in quoted / word tokens
/// (`"prefix[$d bark]"`) are not descended — a rare form.
#[cfg(test)]
pub(crate) fn find_obj_method_call_sites(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
    find_obj_method_call_sites_with_extra_cmd_names(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
        &[],
    )
}

/// Native profiles: the written selectors whose actual receiver entry
/// selects `class_q`'s own `method` declaration on the requested side. Self
/// dispatch stays with the member-body scan, and a receiver with no retained
/// entry contributes no site.
fn original_obj_method_call_sites(
    source: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
    let Some(registry) = analysis.resolved_registry() else {
        return Vec::new();
    };
    let receiver = if is_classmethod {
        tcl_compiler::command_binding::SourceMethodReceiver::Class
    } else {
        tcl_compiler::command_binding::SourceMethodReceiver::Instance
    };
    let mut sites = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            let self_dispatch = command.texts.first().is_some_and(|head| {
                registry.method_dispatch_keyword(head)
                    == Some(tcl_registry::MethodDispatchKind::SelfDispatch)
            });
            if !self_dispatch
                && let Some(selected) =
                    crate::receiver_identity::method_at_command(analysis, source, command)
                && selected.receiver == receiver
                && selected.class.qualified_name == class_q
                && selected.method.name == method
                && !sites.contains(&selected.selector)
            {
                sites.push(selected.selector);
            }
            false
        },
    );
    sites.sort_by_key(|span| (span.start(), span.end()));
    sites
}

/// Every call site in this document that dispatches `class_q`'s [incr Tcl]
/// class-scoped `proc` named `member` — the single `::`-qualified
/// `Factory::make` shape, which is a *different call shape entirely* from
/// the two-word `Factory make` dispatch `classmethod` / `typemethod` use.
///
/// Each returned span covers only the call's final `::`-segment (the `make`
/// of `Factory::make`), so a rename rewrites the member name and leaves the
/// as-written qualifier alone — `Factory::make` → `Factory::produce`.
///
/// Sites come from `analysis.command_invocations` rather than a text scan:
/// an itcl class proc really is an ordinary command, so the analyser has
/// already indexed every call to it, including those nested inside `[...]`
/// substitutions and control-flow bodies.  Each candidate is then resolved
/// with [`crate::definition::itcl_class_proc_target`], which applies Tcl's
/// own current-namespace-then-global rule to the call's *lexical* namespace
/// — so `Factory::make` written inside `namespace eval ::app` reaches
/// `::app::Factory`, and the same text written at the top level (where only
/// `::app::Factory` exists) reaches nothing at all.
pub(super) fn itcl_class_proc_call_sites(
    source: &str,
    analysis: &AnalysisResult,
    _dialect: &'static tcl_dialect::DialectProfile,
    class_q: &str,
    member: &str,
) -> Vec<tcl_lexer::Span> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    let image = tcl_lexer::SourceImage::document(source);
    let Some(realm) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    analysis
        .command_invocations
        .iter()
        .filter_map(|inv| {
            if !inv.lookup.is_execution_site() {
                return None;
            }
            let (word, _site) = realm.original_written_head_at_span_in_source(
                &image,
                inv.range,
                current.config(),
            )?;
            if word.group().expand {
                return None;
            }
            let value = tcl_syntax::word_rules::original_static_word_unicode_value(&word)?;
            let written = std::str::from_utf8(&value).ok()?;
            let namespace = crate::definition::namespace_context_at(
                &analysis.global_scope,
                word.span().start(),
                &analysis.namespace_overrides,
            );
            let (target_class, target_member) =
                crate::definition::itcl_class_proc_target(analysis, &namespace, written)?;
            if target_class != class_q || target_member != member {
                return None;
            }
            crate::original_name_edit::original_plain_source_command_tail(source, analysis, &word)
        })
        .collect()
}

/// Editable receiver targets retain their original declaration spans. Names
/// supplied by a workspace caller remain candidates until a local source
/// declaration and the temporal method entry agree.
fn dispatch_receivers(
    analysis: &AnalysisResult,
    target: (&str, &str, bool),
    extra_cmd_names: &[String],
) -> CommandReceivers {
    let (class_q, method, is_classmethod) = target;
    let mut receivers = CommandReceivers {
        class_targets: FxHashMap::default(),
        instance_target: None,
    };
    if let Some(class) = analysis.all_classes.get(class_q) {
        if is_classmethod
            && class_definition_grammar(analysis, class)
                .is_some_and(|grammar| !grammar.profile.class_procedures_are_qualified())
        {
            if let Some(member) = class.class_methods.get(method) {
                receivers.add_class_target(
                    &class.qualified_name,
                    Some((class.name_span, member.name_span)),
                );
            }
        } else if !is_classmethod && let Some(member) = class.methods.get(method) {
            receivers.instance_target = Some((class.name_span, member.name_span));
        }
    }
    if is_classmethod {
        for name in extra_cmd_names {
            receivers.class_targets.entry(name.clone()).or_insert(None);
        }
    }
    receivers
}

/// External method selector with `extra_cmd_names` — bare command
/// names to treat as valid classmethod-dispatch heads for `method`
/// regardless of what this document's own `analysis.all_classes` knows.
///
/// `is_classmethod` selects `method`'s dispatch shape explicitly rather than
/// inferring it from map membership: a class may legally define a `method`
/// and a `classmethod` of the *same name* (they occupy separate dispatch
/// tables — the instance's and the class object's own), so "does
/// `class_methods` contain this name" cannot answer "which one does the
/// caller mean" when both do.
///
/// A same-document call (`extra_cmd_names` empty) derives everything from
/// `analysis` directly.  The cross-file *pure-consumer* path
/// ([`obj_method_call_sites`]) cannot: a document that only calls `Factory
/// make` and never declares/extends `Factory` has no `::Factory` entry in
/// its own `all_classes` for the local classmethod check below to find, so
/// its caller supplies the workspace-wide answer here instead.
fn find_obj_method_call_sites_with_extra_cmd_names(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
    extra_cmd_names: &[String],
) -> Vec<tcl_lexer::Span> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }
    if !analysis.allows_lexical_declaration_advice() {
        return original_obj_method_call_sites(source, analysis, class_q, method, is_classmethod);
    }
    // [incr Tcl] class-scoped `proc`s land in the same `class_methods`
    // bucket as a `classmethod`, but dispatch as a single `::`-qualified
    // identifier (`Factory::make`) — a shape the two-word scanner below
    // cannot see, and whose two-word look-alike is unrelated
    // instance-creation syntax.  Collected first so every consumer of this
    // scanner (references, rename, the code lens, call hierarchy) gets itcl
    // edges from one place.
    let mut out: Vec<tcl_lexer::Span> = if is_classmethod
        && analysis.all_classes.get(class_q).is_some_and(|cd| {
            class_definition_grammar(analysis, cd)
                .is_some_and(|grammar| grammar.profile.class_procedures_are_qualified())
        }) {
        itcl_class_proc_call_sites(source, analysis, dialect, class_q, method)
    } else {
        Vec::new()
    };
    let receivers =
        dispatch_receivers(analysis, (class_q, method, is_classmethod), extra_cmd_names);
    if !receivers.has_any() {
        return out;
    }
    let mut seen: FxHashSet<(u32, u32)> = out.iter().map(|s| (s.start(), s.end())).collect();
    let Some(_) = analysis.retained_command_realm() else {
        return out;
    };
    let Some(config) = analysis.body_lexer_config else {
        return out;
    };
    let ctx = ObjMethodScan {
        source,
        dialect,
        config,
        analysis,
        receivers: &receivers,
        method,
        var_receivers_in_scope: true,
    };

    // Region 1: the whole document.
    {
        let mut sink = SpanSink {
            out: &mut out,
            seen: &mut seen,
        };
        scan_obj_method_region(ctx, 0, source.len(), false, 0, &mut sink);
    }
    // Regions 2/3: proc + method bodies (the top-level scan
    // skips braced body args, so descend explicitly).
    for proc_def in analysis.all_procs.values() {
        let mut sink = SpanSink {
            out: &mut out,
            seen: &mut seen,
        };
        scan_obj_method_body(ctx, proc_def.body_span, &mut sink);
    }
    for class_def in analysis.all_classes.values() {
        for m in class_def
            .methods
            .values()
            .chain(class_def.class_methods.values())
            .chain(class_def.constructors.iter())
            .chain(class_def.destructor.iter())
        {
            let mut sink = SpanSink {
                out: &mut out,
                seen: &mut seen,
            };
            scan_obj_method_body(ctx, m.body_span, &mut sink);
        }
    }
    out
}

/// Candidate declarations for an editable method reference. Every match also
/// needs the actual retained receiver and original method-entry receipt.
pub(super) struct CommandReceivers {
    class_targets: FxHashMap<String, Option<(tcl_lexer::Span, tcl_lexer::Span)>>,
    instance_target: Option<(tcl_lexer::Span, tcl_lexer::Span)>,
}

impl CommandReceivers {
    /// Register a class whose own command dispatches the method, by its
    /// fully qualified name.
    fn add_class_target(
        &mut self,
        qualified_name: &str,
        declaration: Option<(tcl_lexer::Span, tcl_lexer::Span)>,
    ) {
        self.class_targets
            .insert(qualified_name.to_owned(), declaration);
    }

    fn has_any(&self) -> bool {
        !self.class_targets.is_empty() || self.instance_target.is_some()
    }

    pub(super) fn matches(
        &self,
        selected: &crate::receiver_identity::RetainedReceiverMethod<'_>,
    ) -> bool {
        use tcl_compiler::command_binding::SourceMethodReceiver;
        let expected = match selected.receiver {
            SourceMethodReceiver::Instance => self.instance_target,
            SourceMethodReceiver::Class => self
                .class_targets
                .get(&selected.class.qualified_name)
                .copied()
                .flatten(),
        };
        expected == Some((selected.class.name_span, selected.method.name_span))
    }
}

/// Actual analysis context and editable declaration targets for method reads.
/// The retained realm and grammar came from the original analysis ingress.
#[derive(Clone, Copy)]
pub(super) struct ObjMethodScan<'a> {
    pub(super) source: &'a str,
    pub(super) dialect: &'static tcl_dialect::DialectProfile,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) analysis: &'a AnalysisResult,
    receivers: &'a CommandReceivers,
    pub(super) method: &'a str,
    /// Variable receivers are withheld in frame-shifting regions until the
    /// scan retains that region's exact body and read coordinate mapping.
    var_receivers_in_scope: bool,
}

impl ObjMethodScan<'_> {
    /// This context as it applies inside a frame-shifting region.
    fn frame_shifted(self) -> Self {
        Self {
            var_receivers_in_scope: false,
            ..self
        }
    }

    /// Whether this context can still match anything — a frame-shifted scan
    /// with no command receivers to look for has nothing left to do.
    fn has_receivers(&self) -> bool {
        !self.receivers.class_targets.is_empty()
            || (self.var_receivers_in_scope && self.receivers.instance_target.is_some())
    }
}

/// Mutable sink for matched call-site spans plus the dedup set, threaded
/// alongside [`ObjMethodScan`] through the recursive scan.
pub(super) struct SpanSink<'a> {
    pub(super) out: &'a mut Vec<tcl_lexer::Span>,
    pub(super) seen: &'a mut FxHashSet<(u32, u32)>,
}

/// Scan a brace-delimited body span for `$v method` call sites
/// (stripping the surrounding braces first).
pub(super) fn scan_obj_method_body(
    ctx: ObjMethodScan<'_>,
    body_span: tcl_lexer::Span,
    sink: &mut SpanSink<'_>,
) {
    if body_span.is_empty() {
        return;
    }
    let (start, end) = strip_outer_braces(ctx.source, body_span);
    if start >= end {
        return;
    }
    scan_obj_method_region(ctx, start, end, true, 0, sink);
}

/// Segment `source[start..end]` and record every `$v method` call site,
/// recursing into command-substitution (`[...]`) args **and** every
/// same-frame (`Plain` `BodyKind`) control-flow / `eval` body argument
/// ([`nested_dispatch_regions`]), so a dispatch nested inside an `if` /
/// `while` / `foreach` / `switch` / `try` / `catch` body is found too
/// `depth` guards against runaway recursion
/// — see [`MAX_DISPATCH_SCAN_DEPTH`].
pub(super) fn scan_obj_method_region(
    ctx: ObjMethodScan<'_>,
    start: usize,
    end: usize,
    nested: bool,
    depth: u32,
    sink: &mut SpanSink<'_>,
) {
    use tcl_source::segment::segment_commands_with_offset_and_config;
    let source = ctx.source;
    if start >= end || end > source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return;
    }
    let region = &source[start..end];
    let commands = segment_commands_with_offset_and_config(
        region,
        u32::try_from(start).unwrap_or(0),
        if nested {
            ctx.config.nested()
        } else {
            ctx.config
        },
    );
    for cmd in &commands {
        // Only a written method selector may become an editable reference.
        // Receiver matching uses its retained post-argv declaration receipt.
        if let (Some(head), Some(method_tok)) = (cmd.argv.first(), cmd.argv.get(1)) {
            let h_start = head.span.start() as usize;
            let h_end = head.span.end() as usize;
            if h_start < source.len() && h_end <= source.len() {
                let receiver_matches =
                    crate::receiver_identity::method_at_command(ctx.analysis, source, cmd)
                        .is_some_and(|selected| {
                            selected.editable_selector().is_some()
                                && (ctx.var_receivers_in_scope
                                || selected.receiver
                                    == tcl_compiler::command_binding::SourceMethodReceiver::Class)
                                && ctx.receivers.matches(&selected)
                        });
                if receiver_matches {
                    let m_start = method_tok.span.start() as usize;
                    let m_end = method_tok.span.end() as usize;
                    if m_start < source.len()
                        && m_end <= source.len()
                        && &source[m_start..m_end] == ctx.method
                    {
                        let key = (method_tok.span.start(), method_tok.span.end());
                        if sink.seen.insert(key) {
                            sink.out.push(method_tok.span);
                        }
                    }
                }
            }
        }
        for (inner_start, inner_end) in
            nested_dispatch_regions(source, ctx.analysis, ctx.dialect, cmd)
        {
            scan_obj_method_region(ctx, inner_start, inner_end, true, depth + 1, sink);
        }
        let shifted = ctx.frame_shifted();
        if shifted.has_receivers() {
            for (inner_start, inner_end) in
                frame_shifted_dispatch_regions(source, ctx.analysis, ctx.dialect, cmd)
            {
                scan_obj_method_region(shifted, inner_start, inner_end, true, depth + 1, sink);
            }
        }
    }
}
