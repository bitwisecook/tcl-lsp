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

//! `upvar` and `uplevel` calls: the level word, alias pairs, body shapes and widening past the direct caller.

use super::super::frame_effects::FrameTarget;
use super::selected_frame_bodies::record_upframe_body_at_depth;
use super::{FrameCommandContext, UpvarBindings, UpvarInfo};
use crate::command_binding::ResolvedBindingInvocation;
use crate::ir::{CommandTokens, Script};
use tcl_registry::frame_effect::{FrameArgLayout, FrameEffectSpec, FrameLevel};
use tcl_registry::{CommandRegistry, InvocationArguments};

fn original_scalar_argument<'a>(
    tokens: &'a CommandTokens,
    args: &[String],
    index: usize,
) -> Option<&'a str> {
    let config = tokens
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(tokens)?;
    let word = tokens.words().get(index.checked_add(1)?)?;
    if word.legacy_text() != *args.get(index)? {
        return None;
    }
    let (spelling, source) = word.sole_variable_substitution()?;
    if source.provenance != crate::ir::Provenance::Source {
        return None;
    }
    tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
}

/// True if *name* is a plain literal name with no substitutions or
/// special characters.
fn is_literal_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('$')
        && !name.contains('[')
        && !name.contains('{')
        && !name.contains('"')
        && !name.contains(' ')
}

/// Resolve a frame-crossing command's level word and its remaining
/// arguments through the registry's [`FrameEffectSpec`].
fn resolve_frame_args<'a>(
    spec: FrameEffectSpec,
    args: &'a [String],
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
    tokens: Option<&CommandTokens>,
) -> (FrameLevel, &'a [String]) {
    let Some(tokens) = tokens.filter(|tokens| {
        tokens.original_words_match_argv_text() && tokens.argv_texts.get(1..) == Some(args)
    }) else {
        return (FrameLevel::Dynamic, &[]);
    };
    let words = tokens
        .words()
        .iter()
        .skip(1)
        .map(crate::registry_invocation::invocation_word)
        .collect::<Vec<_>>();
    let mut arguments = InvocationArguments::Structured(&words).with_profile(registry.profile());
    if let Some(dialect) = dialect {
        arguments = arguments.with_dialect(dialect);
    }
    match spec.successful_layout(arguments).layout {
        tcl_registry::frame_effect::FrameArgumentResolution::Valid {
            level,
            level_word_len,
        } => (level, &args[level_word_len..]),
        tcl_registry::frame_effect::FrameArgumentResolution::Invalid => {
            (FrameLevel::Relative(0), &[])
        }
        tcl_registry::frame_effect::FrameArgumentResolution::Unknown => (FrameLevel::Dynamic, &[]),
    }
}

/// Dispatch a registry-declared frame grammar after command binding has
/// supplied its effective argv. This stays command-neutral: aliases, renamed
/// commands, and direct spellings all consume the same descriptor.
pub(super) fn record_frame_effect(
    spec: FrameEffectSpec,
    args: &[String],
    params: &[String],
    registry: &CommandRegistry,
    context: FrameCommandContext<'_>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    match spec.layout {
        FrameArgLayout::AliasPairs => {
            record_upvar_call(spec, args, params, info, registry, context);
        }
        FrameArgLayout::ScriptInSelectedFrame => {
            record_uplevel_call(spec, args, registry, context, namespace, info);
        }
        // A script that runs in the callee's own frame, and an opaque
        // injection into whoever called the command, are effects on the
        // frame the command is written in — not on this proc's caller.
        // `crate::dynamic_names` owns them.
        FrameArgLayout::ScriptInCurrentFrame | FrameArgLayout::OpaqueCallerVars => {}
    }
}

/// An ordinary call shares values rather than frames. It remains composition
/// input only because a user procedure may itself reach past its caller.
pub(super) fn record_plain_call(command: &str, info: &mut UpvarInfo) {
    info.plain_calls.insert(command.to_owned());
}

/// Record a frame effect that lands **past** the direct caller: an exact
/// `upvar 2` / `uplevel 2`, or a level nothing static can place.
///
/// An exact target is opaque at its own frame only; the frames between are
/// untouched, and [`super::super::detect_upvar_procs`] carries it out along
/// ordinary calls. An unknown one may be any frame, the direct caller
/// included, so it widens that too.
pub(super) fn widen_beyond_caller(info: &mut UpvarInfo, target: FrameTarget) {
    if target == FrameTarget::Unknown {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
    }
    info.beyond_caller.insert(target);
}

/// [`widen_beyond_caller`] for a frame selected by `level` from the
/// procedure's own frame: the global and own frames are not past the caller.
pub(super) fn widen_level_beyond_caller(info: &mut UpvarInfo, level: FrameLevel) {
    match level {
        FrameLevel::Relative(distance) => {
            if let Some(target) = FrameTarget::past_the_caller(distance) {
                widen_beyond_caller(info, target);
            }
        }
        FrameLevel::Absolute(0) => {}
        FrameLevel::Absolute(_) | FrameLevel::Dynamic => {
            widen_beyond_caller(info, FrameTarget::Unknown);
        }
    }
}

pub(super) fn record_resolved_alias_pairs(
    invocation: &ResolvedBindingInvocation,
    spec: FrameEffectSpec,
    parameters: &[String],
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
    info: &mut UpvarInfo,
) {
    use tcl_registry::frame_effect::FrameArgumentResolution;
    let (level, start) = match invocation.frame_alias_layout(spec, registry, dialect) {
        FrameArgumentResolution::Valid {
            level,
            level_word_len,
        } => (level, level_word_len),
        FrameArgumentResolution::Invalid => return,
        FrameArgumentResolution::Unknown => {
            widen_beyond_caller(info, FrameTarget::Unknown);
            return;
        }
    };
    if !level.is_caller_frame() {
        widen_level_beyond_caller(info, level);
        return;
    }
    for source in (start..invocation.arguments.len()).step_by(2) {
        let Some(destination) = source
            .checked_add(1)
            .filter(|index| *index < invocation.arguments.len())
        else {
            info.has_unresolvable_caller_target = true;
            return;
        };
        let local = invocation
            .literal_argument(destination)
            .filter(|name| is_literal_name(name));
        let target = invocation
            .literal_argument(source)
            .filter(|name| is_literal_name(name));
        if local.is_none() {
            info.unnameable_local_aliases
                .insert(invocation.arguments[source].clone());
        }
        if let Some(target) = target {
            if let Some(local) = local {
                info.literal_targets
                    .insert(local.to_owned(), target.to_owned());
            } else {
                info.uplevel_literal_writes.insert(target.to_owned());
            }
        } else if let Some(parameter) = invocation.scalar_parameter_argument(source, parameters) {
            if tcl_syntax::formal_params::is_variadic_formal_name(parameter) {
                if let Some(local) = local {
                    info.args_tail_upvar.push(local.to_owned());
                } else {
                    info.has_unresolvable_caller_target = true;
                }
            } else if let Some(local) = local {
                info.param_targets
                    .insert(local.to_owned(), parameter.to_owned());
            } else {
                info.uplevel_param_writes.insert(parameter.to_owned());
            }
        } else {
            info.has_unresolvable_caller_target = true;
        }
    }
}

/// `upvar ?level? otherVar myVar …` — classify each pair against the frame
/// the level word selects.
fn record_upvar_call(
    spec: FrameEffectSpec,
    args: &[String],
    params: &[String],
    info: &mut UpvarInfo,
    registry: &CommandRegistry,
    context: FrameCommandContext<'_>,
) {
    let (level, rest) = resolve_frame_args(spec, args, registry, context.dialect, context.tokens);
    if !level.is_caller_frame() {
        // A level this summary cannot place at the direct caller. `#0` and
        // `0` genuinely miss the caller's frame (the global frame is
        // `global_write_info`'s business, the callee's own frame nobody
        // else's), so they contribute nothing. Everything else — `upvar 2`,
        // `upvar #3`, `upvar $lvl` — writes *some* frame further out, and
        // dropping it would leave that write invisible where it lands, so
        // widen there. See the module doc's frame table.
        widen_level_beyond_caller(info, level);
        return;
    }
    let start = args.len().saturating_sub(rest.len());
    let mut i = 0;
    while i + 1 < rest.len() {
        let parameter = context
            .tokens
            .and_then(|tokens| original_scalar_argument(tokens, args, start + i))
            .filter(|name| params.iter().any(|parameter| parameter == name));
        let (src, dst) = (rest[i].as_str(), rest[i + 1].as_str());
        i += 2;
        let dst_is_literal = is_literal_name(dst);
        if !dst_is_literal {
            // Dynamic destination (`upvar 1 x $dst`) — there is no local
            // key to file the alias under, so the per-local maps cannot
            // carry it.  The alias is still real: record the structural
            // fact for caller-frame reachability, and classify the CALLER
            // side below so call sites still widen the right caller name
            // — otherwise such a proc reads as summary-empty and `p x` leaves
            // the caller's `x` foldable to a stale constant.
            // tclsh 9.0.4 / 8.6.16: `proc p {dst} {upvar 1 x $dst; set $dst
            // 2}; proc c {} {set x 1; p x; puts $x}` prints 2).
            info.unnameable_local_aliases.insert(src.to_string());
        }
        if is_literal_name(src) {
            if dst_is_literal {
                // `upvar 1 caller_x x` — literal source, literal local.
                info.literal_targets
                    .insert(dst.to_string(), src.to_string());
            } else {
                // `upvar 1 caller_x $dst` — the caller-side name is still
                // exactly `caller_x`; only the local key is unknowable, so
                // it lands in the keyless bucket.
                info.uplevel_literal_writes.insert(src.to_string());
            }
        } else if let Some(param) = parameter {
            // `upvar 1 $param x` — substitution-driven, gated on
            // the param existing on the proc.
            if tcl_syntax::formal_params::is_variadic_formal_name(param) {
                if dst_is_literal {
                    info.args_tail_upvar.push(dst.to_string());
                } else {
                    // `upvar 1 $args $dst` — no positional local to pair
                    // the tail against; widen.
                    info.has_unresolvable_caller_target = true;
                }
            } else if params.iter().any(|p| p == param) {
                if dst_is_literal {
                    info.param_targets
                        .insert(dst.to_string(), param.to_string());
                } else {
                    // `upvar 1 $param $dst` — the caller-side name is the
                    // parameter's value, resolvable at the call site; only
                    // the local key is missing.
                    info.uplevel_param_writes.insert(param.to_string());
                }
            } else {
                // `$foo` where `foo` isn't a param — the caller-side name
                // depends on runtime state; the callee can clobber any
                // caller variable, so the summary must widen (a silent
                // skip would let SCCP propagate stale caller values past
                // the call).
                info.has_unresolvable_caller_target = true;
            }
        } else {
            // Fully dynamic source (`upvar 1 [pick] x`, `upvar 1 a($i) x`,
            // …) — same widening as above, whatever the local side.
            info.has_unresolvable_caller_target = true;
        }
    }
}

/// `uplevel ?level? arg …` — the concatenated `arg`s run as a script in the
/// frame the level selects.
///
/// Three body shapes are distinguishable, and only the third is opaque:
///
/// * a **brace literal** never reaches here — the lowering turned it into
///   [`Statement::UpFrame`], handled by [`record_upframe_body`];
/// * a **fully literal constructed list with an absolute head** (`[list ::set
///   v 1]`) has namespace-invariant result words, so
///   [`record_constructed_body`] can read it;
/// * anything else (`$body`, `[build]`, `"$a $b"`) is arbitrary code —
///   widen both directions.
fn record_uplevel_call(
    spec: FrameEffectSpec,
    args: &[String],
    registry: &CommandRegistry,
    context: FrameCommandContext<'_>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    let (level, rest) = resolve_frame_args(spec, args, registry, context.dialect, context.tokens);
    if !level.is_caller_frame() {
        // Same reasoning as `record_upvar_call`: `uplevel 0` stays in the
        // callee's own frame and `uplevel #0` runs in the global one, so
        // neither touches the caller; any other level does, somewhere this
        // summary places by its distance.
        widen_level_beyond_caller(info, level);
        return;
    }
    // `uplevel` concat-joins every remaining word into one script (its
    // `SCRIPT_CONCATENATES_ARGS` trait). A single constructed-list word is
    // the readable case; a multi-word join is not worth reassembling.
    if let [single] = rest
        && record_constructed_body(single, registry, context.bindings, namespace, info)
    {
        return;
    }
    info.caller_frame_opaque_writes = true;
    info.caller_frame_opaque_reads = true;
}

/// The statements of an inlined `uplevel 1 {literal}` body: every literal
/// name it assigns is a caller-frame write.
///
/// A body statement whose target is *not* a plain literal name means the
/// body writes somewhere this summary cannot name, so widen — the same
/// direction the constructed-list path takes.
pub(super) fn record_upframe_body(
    body: &Script,
    source_body_is_braced: bool,
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    record_upframe_body_at_depth(
        body,
        source_body_is_braced,
        registry,
        bindings,
        namespace,
        info,
        0,
    );
}

/// Whether the original `uplevel` body was an unsubstituted brace word.
///
/// A body recovered from `$tmp` after lowering a `[list ...]` constructor is
/// readable, but its relative command heads still resolve in the selected
/// caller namespace. The original token snapshot is the source-of-truth for
/// that distinction; the lowered body alone cannot recover it after `set`
/// becomes `AssignConst`.
pub(super) fn upframe_body_is_braced(tokens: Option<&CommandTokens>) -> bool {
    let Some(tokens) = tokens else {
        return false;
    };
    let Some(argument_index) = tokens.words().len().checked_sub(2) else {
        return false;
    };
    tokens.arg_is_braced_literal(argument_index)
}

/// Read a `[list CMD ARG…]` body word, recording the caller-frame names the
/// constructed command writes.  Returns `false` when the word is not a
/// readable constructed list, so the caller can widen.
///
/// `[list …]` is recognised through the registry's
/// [`ReturnElements::ListOfArgs`](tcl_registry::ReturnElements) — the same
/// fact the concat rules read — not by matching the name `list`.
fn record_constructed_body(
    word: &str,
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
) -> bool {
    let Some(bindings) = bindings else {
        return false;
    };
    let Some(constructed) = crate::command_binding::constructed_script_words_with_metadata_context(
        word,
        registry,
        bindings.source,
        namespace,
        Some(bindings.metadata),
    ) else {
        return false;
    };
    let Some((command, cargs)) = constructed.split_first() else {
        return false;
    };
    // The constructed script runs in the selected caller frame, so an
    // unqualified command word resolves there rather than in this procedure's
    // namespace. The constructor is statically known, but its result's head
    // is not namespace-invariant: a caller-local `set`/`worker` alias can
    // write or read arbitrary variables. Preserve exact handling only for an
    // explicitly absolute command spelling.
    if !tcl_syntax::naming::is_absolute(command) {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return true;
    }
    let arg_refs: Vec<&str> = cargs.iter().map(String::as_str).collect();
    if let Some(invocation) = tcl_registry::model::assembly::resolve_invocation_in_context(
        registry,
        Some(bindings.metadata.context()),
        command,
        &arg_refs,
    ) {
        let projection =
            CommandRegistry::variable_write_projection_for_selected_source(&invocation);
        info.uplevel_literal_writes.extend(projection.literal_names);
        info.caller_frame_opaque_writes |= projection.opaque_variable_frame;
        return true;
    }
    // Not a registry command — a candidate user proc. Its own `upvar 1`
    // reaches *this* proc's caller, one frame further out than a plain call
    // would, but only the module-wide pass can resolve it.
    info.uplevel_forwarded_calls
        .push((command.clone(), cargs.to_vec()));
    true
}
