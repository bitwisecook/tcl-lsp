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

//! Call sites inside the script bodies a command invokes: nested original bodies and standalone body arguments.

use super::call_recording::{record_call_site_evidence, record_unenumerable_targets};
use super::{CallSiteEvidence, CallSiteScanCtx, CallerFrame};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution_with_config};
use tcl_registry::{ArgRole, Traits};

/// True when the whole word is a single substitution — `$v`, `${v}`, or
/// `[cmd …]` — so its *value*, not its text, is the script / lambda /
/// command prefix the command receives.
///
/// The distinction matters for every script-bearing argument: `catch {puts
/// $x}` carries the literal script text `puts $x` (which merely *contains*
/// a substitution), whereas `eval $script` carries no script text at all —
/// only a reference to one.  Testing `is_dynamic_word` alone conflates the
/// two and would treat every ordinary braced body that mentions a variable
/// as unresolvable.
pub(super) fn word_is_whole_substitution(word: &str, config: tcl_lexer::LexerConfig) -> bool {
    is_pure_var_ref(word, config) || parse_command_substitution_with_config(word, config).is_some()
}

pub(super) fn record_invocation_body_evidence(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    tokens: &crate::ir::CommandTokens,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    depth: u32,
) {
    let Some(metadata) = metadata else {
        // This compatibility utility is available only after original_metadata
        // has independently checked both explicit Standalone owners.
        record_standalone_invocation_body_evidence(
            out,
            ctx,
            caller,
            invocation,
            Some(tokens),
            depth,
        );
        return;
    };
    let Some(input) = metadata.source_analysis_input() else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    let Some(words) =
        crate::registry_invocation::source_structure::original_registry_words_for_supplied_tokens(
            ctx.identities,
            input,
            tokens,
        )
    else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    let Some(roles) = words.roles() else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    let has_bodies = roles.iter().any(|&(_, role)| role == ArgRole::Body);
    for &(argument, role) in roles {
        if role == ArgRole::LambdaLiteral
            && words
                .arguments()
                .get(argument)
                .and_then(|word| word.literal_bytes())
                .is_none()
        {
            record_unenumerable_targets(out, ctx);
        }
    }
    if !has_bodies {
        return;
    }
    if tcl_source::limits::MAX_SOURCE_NEST_DEPTH.exceeded(depth + 1) {
        record_unenumerable_targets(out, ctx);
        return;
    }
    // Definitions and frame-shifting bodies have independent caller units.
    // The selected authored trait description supplies no entered frame.
    let context = input.context_registry();
    let Some(traits) = words.with_source_schema(&context, |schema| schema.semantics.traits) else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    if traits.intersects(Traits::DEFINES_PROCEDURE | Traits::EVALUATES_IN_SHIFTED_FRAME) {
        return;
    }
    let Some(bodies) = words.source_script_bodies_for_mutation_coverage(&context) else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    for body in bodies {
        record_nested_body_invocation_evidence(out, ctx, caller, tokens, &body, &context, depth);
    }
}

fn record_nested_body_invocation_evidence(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    tokens: &crate::ir::CommandTokens,
    body: &crate::registry_invocation::OriginalSourceScriptBody,
    context: &tcl_registry::model::ContextRegistry,
    depth: u32,
) {
    let Some(commands) = body.original_commands(context) else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    for command in commands {
        let Some(binding) = unique_nested_binding(tokens, command.span.start()) else {
            record_unenumerable_targets(out, ctx);
            continue;
        };
        let Some(site) = binding.invocation_site() else {
            record_unenumerable_targets(out, ctx);
            continue;
        };
        let Some((segment, mut original)) = binding.original_recorded_command() else {
            record_unenumerable_targets(out, ctx);
            continue;
        };
        let config = binding.original_lexer_config_for_tokens(&original);
        if site.offset != command.span.start()
            || site.source.source_image() != body.original_container().image()
            || config != Some(body.original_container().config())
            || crate::registry_invocation::original_native_compiler_words(
                site.source.source_image(),
                original.words(),
                site.offset,
                body.original_container().config(),
            )
            .as_deref()
                != Some(command.words.as_slice())
        {
            record_unenumerable_targets(out, ctx);
            continue;
        }
        original.inherit_nested_bindings(tokens);
        record_call_site_evidence(
            out,
            ctx,
            caller,
            segment.name(),
            segment.args(),
            Some(&original),
            depth + 1,
        );
    }
}

fn unique_nested_binding(
    tokens: &crate::ir::CommandTokens,
    offset: u32,
) -> Option<&crate::command_binding::SourceInvocationBinding> {
    let mut children = tokens
        .nested_bindings
        .iter()
        .filter(|(nested_offset, _)| *nested_offset == offset)
        .map(|(_, binding)| binding);
    let binding = children.next()?;
    children.all(|other| other == binding).then_some(binding)
}

/// Scalar lexical compatibility under independently explicit Standalone ownership.
fn record_standalone_invocation_body_evidence(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    tokens: Option<&crate::ir::CommandTokens>,
    depth: u32,
) {
    let args = invocation.arguments.as_slice();
    if tcl_source::limits::MAX_SOURCE_NEST_DEPTH.exceeded(depth + 1) {
        if invocation
            .facts
            .arg_roles
            .iter()
            .any(|&(_, role)| matches!(role, ArgRole::Body | ArgRole::LambdaLiteral))
        {
            record_unenumerable_targets(out, ctx);
        }
        return;
    }
    // A lambda the command receives as a *value* (`apply $fn`) is exactly as
    // unreadable as a script body received as one, and may likewise call
    // anything.  A *literal* lambda needs no walking here — lowering gives it
    // its own body unit, which the scan already visits as a caller.
    let offset = invocation.facts.argument_offset;
    let roles = &invocation.facts.arg_roles;
    let role_indices = |role| {
        roles.iter().filter_map(move |&(index, found)| {
            (found == role).then_some(offset + usize::from(index))
        })
    };
    for idx in role_indices(ArgRole::LambdaLiteral) {
        if args.get(idx).is_some_and(|w| {
            w.as_deref().is_none_or(|word| {
                word_is_whole_substitution(word, ctx.module.lexer_config.nested())
            })
        }) {
            record_unenumerable_targets(out, ctx);
        }
    }
    for idx in role_indices(tcl_registry::ArgRole::Body) {
        let Some(body_text) = args.get(idx).and_then(Option::as_deref) else {
            record_unenumerable_targets(out, ctx);
            continue;
        };
        // `eval $script` / `catch $body` carry no script *text* at all, only
        // a reference to one: unreadable, and it may call any procedure with
        // any argument.  A literal body that merely *mentions* a variable
        // (`catch {puts $x}`) is still walked — the discriminator is "the
        // whole word is one substitution", not "contains a `$`".
        if word_is_whole_substitution(body_text, ctx.module.lexer_config.nested()) {
            record_unenumerable_targets(out, ctx);
            continue;
        }
        // A body whose resolution namespace differs from the caller's must not
        // be walked with the caller's namespace: `namespace eval ::a { helper }`
        // calls `::a::helper`, never the caller's own `helper`
        // (tclsh8.6-confirmed; see `lowering`'s `register_body_unit` call for
        // the same reasoning). Such a command carries an absolute namespace
        // name argument, and lowering has already registered its body as
        // a body unit whose qname encodes it — which
        // `build_extra_call_site_scan_contexts` scans with the *correct*
        // namespace. Recursing here as well would scan it a second time under
        // the wrong one, inventing a call to a same-named proc in the caller's
        // namespace. Cross-file that is a false edge into another file's
        // procedure; in-unit it is merely invisible, because a bare global
        // `::helper` is rarely in a single file's own `known` set.
        //
        // Both name roles are consulted: `ArgRole::NamespaceName` is the
        // precise one (`namespace eval` / `inscope`), while a
        // generic `ArgRole::Name` still covers any other command whose
        // symbolic name word is written absolutely.
        if [
            tcl_registry::ArgRole::NamespaceName,
            tcl_registry::ArgRole::Name,
        ]
        .into_iter()
        .flat_map(role_indices)
        .filter_map(|i| args.get(i).and_then(Option::as_deref))
        .any(crate::naming::is_absolute)
        {
            continue;
        }
        // The same rule for a *definition* body (`Traits::DEFINES_PROCEDURE`
        // — `proc`, `oo::class create`, …): it does not run at the
        // definition site at all, and when it runs it resolves in the
        // defined procedure's own namespace and frame. Lowering has already
        // registered it as a procedure / method / body unit, all of which
        // this scan visits with the right context, so recursing here would
        // only re-walk it under the definer's. Otherwise `namespace eval
        // ::foo { proc runIt {} { uplevel #0 { helper b } } }` invents a
        // call to `::foo::helper` — a proc tclsh8.6/9.0 confirm real Tcl
        // never reaches this way.
        //
        // And the same rule again for a body that runs in another *frame*
        // (`Traits::EVALUATES_IN_SHIFTED_FRAME` — `uplevel`). The frame the
        // level argument selects decides which namespace the body's bare
        // command words resolve against, and the enclosing unit is not it.
        // `upframe_scan_bodies` visits every such body with the frame it
        // actually targets, so recursing here would scan it a second time
        // under the wrong one.
        //
        // Sound-7: `proc runIt {} { catch { uplevel #0 { helper b } } }`
        // inside `::foo`. The `catch` body is walked correctly (catch does
        // not shift anything), and the `uplevel` body inside it was then
        // re-walked as `::foo`, inventing a call to `::foo::helper` on top
        // of the correct `::helper` the upframe scan had already recorded.
        // tclsh8.6/9.0 confirm only `::helper` runs.
        if invocation
            .facts
            .traits
            .intersects(Traits::DEFINES_PROCEDURE | Traits::EVALUATES_IN_SHIFTED_FRAME)
        {
            continue;
        }
        let config = ctx.module.lexer_config.nested();
        let body_base = invocation.effective.words.get(idx + 1).map_or(0, |word| {
            word.source().span.start()
                + u32::from(matches!(
                    word,
                    tcl_source::words::WordExpr::BracedLiteral { .. }
                ))
        });
        let nested = tcl_source::segment::segment_commands_with_offset_and_config(
            body_text, body_base, config,
        );
        let source_map = tcl_lexer::SourceMap::new(body_text).with_base(body_base, 0, 0);
        for cmd in &nested {
            let name = cmd.name();
            if name.is_empty() {
                continue;
            }
            let mut nested_tokens =
                crate::ir::CommandTokens::from_segmented(&source_map, config, cmd);
            if let Some(parent) = tokens {
                nested_tokens.inherit_nested_bindings(parent);
            }
            record_call_site_evidence(
                out,
                ctx,
                caller,
                name,
                cmd.args(),
                Some(&nested_tokens),
                depth + 1,
            );
        }
    }
}
