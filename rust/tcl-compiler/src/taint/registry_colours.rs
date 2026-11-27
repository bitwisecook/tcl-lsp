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

//! Registry-declared colours: what a source or a transform stamps on its result, and the bridge to the compiler's colours.

use super::sink_warnings::{original_scalar_reference_name, static_word_expr_value};
use super::{LocalInstanceClasses, TaintColour, TaintCtx, TaintLattice, dialect_to_point};
use tcl_dialect::model::SurfaceQuery;
use tcl_registry::{CommandRegistry, TaintColourAtom, Traits};
use tcl_source::words::WordExpr;

// Source-command classification

/// Return `true` when `command` is a known taint source — i.e. its
/// return value may carry attacker-influenced data.
///
/// Commands with `UNNORMALISED_HTTP_GETTER` trait (iRules dialect) are
/// also included once the registry carries that flag on actual specs.
/// When `dialect` is `"f5-irules"` / `"irules"`, iRules namespace
/// prefixes (`HTTP::`, `URI::`, `IP::`, …) are also treated as
/// attacker-controlled sources.
/// The taint lattice a source `command` produces, or `None` when the
/// call is not a taint source. Wraps the registry's
/// `taint_source_colour` (which carries the per-command source colour
/// and the derived-safety augmentation) into the compiler's mirror
/// lattice. Replaces the old "any source ⇒ bare `TAINTED`" rule so
/// path/IP/port/FQDN getters propagate their option-injection-safe
/// colours.
pub(super) fn source_colour(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    instance_classes: Option<&LocalInstanceClasses>,
    source_position: Option<u32>,
    receiver: Option<(&WordExpr, tcl_lexer::LexerConfig)>,
) -> Option<TaintLattice> {
    let dialect = dialect_to_point(dialect);
    let direct = tcl_registry::taint::taint_source_colour(registry, command, args, dialect);
    let colour = direct.or_else(|| {
        instance_taint_source_colour(
            registry,
            command,
            args,
            dialect,
            instance_classes,
            source_position,
            receiver,
        )
    })?;
    Some(TaintLattice::from_colours(reg_colour(colour)))
}

/// [`source_colour`] restricted to registry-typed object instances, with the
/// method selected in the source's own context registry (its loaded packages).
pub(super) fn instance_source_colour(
    command: &str,
    args: &[&str],
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    instance: (Option<&LocalInstanceClasses>, Option<u32>),
    receiver: Option<(&WordExpr, tcl_lexer::LexerConfig)>,
) -> Option<TaintLattice> {
    let class = unique_instance_class(receiver, instance.0, instance.1)?;
    let context = metadata?.source_analysis_input()?.context_registry();
    let spec = context.commands().get(class)?;
    let words = tcl_registry::InvocationWords::literals(command, args);
    let invocation = context
        .commands()
        .resolve_structured_instance_invocation_for_descriptor_in_context(spec, words, &context)
        .ok()
        .flatten()?;
    let traits = invocation.semantics.traits;
    (traits.contains(Traits::TAINT_SOURCE)
        || traits.contains(Traits::TAINT_SOURCE_ZERO_ARGS) && args.len() == 1)
        .then(TaintLattice::tainted)
}

/// Resolve a registry-declared source on a statically-known object instance.
///
/// Tcl widget and package-object APIs are invoked through a command created at
/// runtime (`.entry get`, `$widget get`) rather than through the constructor's
/// registered command name.  The object-class and `creates_instance_at`
/// descriptors already model that dispatch; this is the taint counterpart.
/// It deliberately knows neither Tk widget names nor method spellings: a
/// source is recognised only when the registry says the receiver's class has
/// an instance method carrying [`Traits::TAINT_SOURCE`].
///
/// A receiver with multiple possible classes is not classified.  A source is
/// a security fact, so guessing a method from a widened object flow would turn
/// an unrelated dynamic object call into a false positive.
fn instance_taint_source_colour(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    dialect: Option<SurfaceQuery<'_>>,
    instance_classes: Option<&LocalInstanceClasses>,
    source_position: Option<u32>,
    receiver: Option<(&WordExpr, tcl_lexer::LexerConfig)>,
) -> Option<tcl_registry::TaintColour> {
    let class = unique_instance_class(receiver, instance_classes, source_position)?;
    registry
        .resolve_instance_invocation(class, command, args, dialect)
        .filter(|invocation| {
            invocation.semantics.traits.contains(Traits::TAINT_SOURCE)
                || invocation
                    .semantics
                    .traits
                    .contains(Traits::TAINT_SOURCE_ZERO_ARGS)
                    && args.len() == 1
        })
        .map(|_| tcl_registry::TaintColour::TAINTED)
}

/// The receiver's single registry class at `source_position`.
///
/// Shared by source-return and linked-variable instance semantics so both
/// abstain identically on an untyped or widened receiver.
pub(crate) fn unique_instance_class<'a>(
    receiver: Option<(&WordExpr, tcl_lexer::LexerConfig)>,
    instance_classes: Option<&'a LocalInstanceClasses>,
    source_position: Option<u32>,
) -> Option<&'a str> {
    let (word, config) = receiver?;
    if matches!(word, WordExpr::Expand { .. }) {
        return None;
    }
    let key = static_word_expr_value(
        word,
        config.escapes,
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
    )
    .or_else(|| original_scalar_reference_name(word, config).map(str::to_owned))?;
    if key.contains('\0') {
        return None;
    }
    let position = source_position?;
    let classes = instance_classes?.at.get(&position)?.get(&key)?;
    if classes.len() != 1 {
        return None;
    }
    classes.iter().next().map(String::as_str)
}

/// Whether a registry-typed instance call exposes `variable` to external
/// input through one of its class/factory options.
pub(super) fn instance_taints_var_write(
    ctx: TaintCtx<'_>,
    command: &str,
    args: &[&str],
    variable: &str,
) -> bool {
    if args.is_empty() {
        return false;
    }
    let receiver = ctx
        .source_tokens
        .and_then(|tokens| tokens.words().first())
        .map(|word| (word, ctx.source.config));
    let Some(class) = unique_instance_class(receiver, ctx.instance_classes, ctx.source_position)
    else {
        return false;
    };
    let dialect = dialect_to_point(ctx.dialect);
    let Some(invocation) = ctx
        .registry
        .resolve_instance_invocation(class, command, args, dialect)
    else {
        return false;
    };
    invocation
        .semantics
        .traits
        .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
        && tcl_registry::taint::taints_var_write(ctx.registry, class, &args[1..], dialect, variable)
}

/// Bridge a registry colour to the compiler's mirror enum.
///
/// This must remain a total conversion: a registry bit must never disappear
/// while crossing into the compiler's lattice. `from_bits` makes a future
/// mismatch fail closed during development instead of silently truncating a
/// sanitiser declaration.
const fn compiler_colour_atom(atom: TaintColourAtom) -> TaintColour {
    match atom {
        TaintColourAtom::Tainted => TaintColour::TAINTED,
        TaintColourAtom::PathPrefixed => TaintColour::PATH_PREFIXED,
        TaintColourAtom::NonDashPrefixed => TaintColour::NON_DASH_PREFIXED,
        TaintColourAtom::CrlfFree => TaintColour::CRLF_FREE,
        TaintColourAtom::ShellAtom => TaintColour::SHELL_ATOM,
        TaintColourAtom::ListCanonical => TaintColour::LIST_CANONICAL,
        TaintColourAtom::RegexLiteral => TaintColour::REGEX_LITERAL,
        TaintColourAtom::PathNormalised => TaintColour::PATH_NORMALISED,
        TaintColourAtom::PathBounded => TaintColour::PATH_BOUNDED,
        TaintColourAtom::HeaderTokenSafe => TaintColour::HEADER_TOKEN_SAFE,
        TaintColourAtom::HtmlEscaped => TaintColour::HTML_ESCAPED,
        TaintColourAtom::UrlEncoded => TaintColour::URL_ENCODED,
        TaintColourAtom::IpAddress => TaintColour::IP_ADDRESS,
        TaintColourAtom::Port => TaintColour::PORT,
        TaintColourAtom::Fqdn => TaintColour::FQDN,
        TaintColourAtom::PathJoined => TaintColour::PATH_JOINED,
        TaintColourAtom::Channel => TaintColour::CHANNEL,
    }
}

pub(super) fn reg_colour(c: tcl_registry::TaintColour) -> TaintColour {
    let mut out = TaintColour::empty();
    for atom in TaintColourAtom::ALL {
        if c.contains(atom.colour()) {
            out |= compiler_colour_atom(atom);
        }
    }
    assert_eq!(
        out.bits(),
        c.bits(),
        "registry and compiler TaintColour definitions must stay aligned"
    );
    out
}

/// The inverse bridge, kept beside [`reg_colour`] so conversions stay audited
/// in both directions whenever the registry adds a colour.
#[cfg(test)]
pub(super) fn compiler_colour(c: TaintColour) -> tcl_registry::TaintColour {
    let mut out = tcl_registry::TaintColour::empty();
    for atom in TaintColourAtom::ALL {
        if c.contains(compiler_colour_atom(atom)) {
            out |= atom.colour();
        }
    }
    assert_eq!(
        out.bits(),
        c.bits(),
        "compiler and registry TaintColour definitions must stay aligned"
    );
    out
}

/// The colour a command stamps on a tainted value it returns — its
/// `taint_transform` (e.g. `uri::encode` ⇒ `URL_ENCODED`,
/// `file normalize` ⇒ `PATH_NORMALISED`).  Subcommand transforms take
/// precedence over the bare-command form, and a declaration carrying a
/// `taint_transform_when` condition (`string map`, whose sanitising effect is
/// a property of the mapping it was given) colours only the calls that satisfy
/// it.
pub(super) fn transform_colour(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
) -> Option<TaintColour> {
    tcl_registry::taint::taint_transform_for_call(registry, command, args).map(reg_colour)
}

/// Human-readable label for a double-encode colour, for the T106
/// message.
pub(super) fn double_encode_label(colour: TaintColour) -> &'static str {
    if colour.contains(TaintColour::URL_ENCODED) {
        "URL-encoded"
    } else if colour.contains(TaintColour::HTML_ESCAPED) {
        "HTML-escaped"
    } else if colour.contains(TaintColour::REGEX_LITERAL) {
        "regex-escaped"
    } else {
        "encoded"
    }
}
