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

//! Rendering a stored callback as Tcl source that is taint-equivalent at word granularity.

use super::{CallbackListBuilder, CallbackReplaySource};
use tcl_lexer::TokenType;
use tcl_source::words::WordExpr;
use tcl_source::words::WordPart;

/// A word whose runtime value is exactly one command substitution. Quotes
/// contribute empty text fragments, so `"[list ...]"` is equivalent to the
/// unquoted form; any real prefix/suffix makes it a different value.
pub(super) fn whole_command_substitution(word: &WordExpr) -> Option<&str> {
    match word {
        WordExpr::CommandSubstitution { spelling, .. } => Some(spelling),
        WordExpr::Template { parts, .. } => {
            let mut substitution = None;
            for part in parts {
                match part {
                    WordPart::Text { text, .. } if text.is_empty() => {}
                    WordPart::CommandSubstitution { spelling, .. } if substitution.is_none() => {
                        substitution = Some(spelling.as_str());
                    }
                    _ => return None,
                }
            }
            substitution
        }
        _ => None,
    }
}

/// Convert one statically materialised callback script into normal Tcl source
/// whose registry-declared external `%` fields are explicit taint sources.
///
/// A braced/literal script is already executable source. A whole `[list ...]`
/// callback is the idiomatic prefix form: `list` builds the script at
/// registration time, so its decoded elements become one invocation here.
/// Any other command substitution or a malformed list remains dynamic and is
/// intentionally not guessed.
pub(super) fn callback_replay_source(
    callback: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
    source_is_command_substitution: bool,
    input_var: &str,
    proc_name: &str,
    config: tcl_lexer::LexerConfig,
    selected_builder: Option<&CallbackListBuilder>,
) -> Option<CallbackReplaySource> {
    let callback = callback.trim().strip_prefix('+').unwrap_or(callback.trim());
    // Current lowering normally preserves the brackets in `args`; older
    // compatibility IR can retain only the command content.  The token kind
    // tells us it was a substitution in both cases, while the spelling avoids
    // wrapping an already bracketed word a second time.
    let bracketed = (source_is_command_substitution && !callback.starts_with('['))
        .then(|| format!("[{callback}]"));
    let callback_command = bracketed.as_deref().unwrap_or(callback);
    if callback_command.starts_with('[') {
        let script = callback_replay_list_prefix(
            callback_command,
            inputs,
            input_var,
            config,
            selected_builder,
        )?;
        return Some(callback_replay_with_input(&script, proc_name, input_var));
    }
    let script = callback_replay_script(callback, inputs, input_var, config)?;
    Some(callback_replay_with_input(&script, proc_name, input_var))
}

/// Materialise one static `[list COMMAND ARG ...]` callback prefix.
///
/// The list builder executes while the callback is registered, so each source
/// word is first reduced to its Tcl value under the document's release grammar.
/// Variable/command substitutions remain dynamic and make the replay abstain.
fn callback_replay_list_prefix(
    callback: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
    input_var: &str,
    _config: tcl_lexer::LexerConfig,
    selected_builder: Option<&CallbackListBuilder>,
) -> Option<String> {
    let selected_builder = selected_builder?;
    if selected_builder.source != callback
        || !selected_builder
            .invocation
            .facts
            .traits
            .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
        || selected_builder.invocation.facts.argument_offset != 0
        || selected_builder.invocation.facts.native_result
            != Some(tcl_registry::native_result::NativeResultContract::ListArguments { from: 0 })
    {
        return None;
    }
    let invocation = &selected_builder.invocation;
    if invocation.evaluated_arguments.len() != invocation.evaluated_words.len() {
        return None;
    }
    let mut elements = Vec::new();
    for (word, presentation) in invocation
        .evaluated_words
        .iter()
        .zip(&invocation.evaluated_arguments)
    {
        match word {
            crate::registry_invocation::EffectiveInvocationWord::Literal(value)
                if presentation.as_deref() == Some(value.as_str()) =>
            {
                elements.push(value.clone());
            }
            crate::registry_invocation::EffectiveInvocationWord::KnownExpansion(values)
                if presentation.is_none() =>
            {
                elements.extend(values.iter().cloned());
            }
            _ => return None,
        }
    }
    if elements.is_empty() {
        return None;
    }
    Some(
        elements
            .iter()
            .map(|element| callback_replay_list_word(element, inputs, input_var))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Build a callback script that is taint-equivalent at Tcl word granularity.
///
/// Tk replaces its `%` fields *before* it evaluates the callback, after
/// quoting each replacement as one Tcl element.  Replacing raw text would get
/// a braced word wrong: `eval {%P}` would become `eval {$input}`, whose first
/// evaluation only sees a literal.  The segmenter gives us Tcl's actual word
/// boundaries, so a marked non-head word instead becomes the symbolic input
/// value itself (`eval ${input}`). This also keeps quoted/bare embedded
/// prefix/suffix as one tainted word. A marked head is preserved as the
/// seeded dynamic value and classified by the ordinary dynamic-dispatch sink.
pub(super) fn callback_replay_script(
    callback: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
    input_var: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let config = config.nested();
    let commands =
        tcl_source::segment::segment_commands_with_offset_and_config(callback, 0, config);
    if commands.is_empty() {
        return None;
    }
    let mut replayed = Vec::with_capacity(commands.len());
    for command in commands {
        if command.is_partial || command.texts.is_empty() || !command.word_views_aligned() {
            return None;
        }
        let mut words = Vec::with_capacity(command.texts.len());
        let expand = command.expand_word.as_deref().unwrap_or_default();
        for (index, (word, fragments)) in command
            .texts
            .iter()
            .zip(&command.word_fragments)
            .enumerate()
        {
            let marked = callback_word_has_input(word, inputs);
            let rendered = if marked {
                callback_replay_tainted_word(word, inputs, input_var)
            } else {
                // Preserve dynamic substitutions for the ordinary compiler
                // pipeline (including a phase-correct `$command` head), while
                // reducing static words to their exact runtime value.
                callback_replay_script_word(
                    word,
                    fragments,
                    config.escapes,
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                )?
            };
            // `{*}` is an argv operation outside the word itself. Canonical
            // word rendering must retain that marker or a callback whose
            // command prefix is a list silently changes into one command name.
            words.push(if expand.get(index).copied().unwrap_or(false) {
                format!("{{*}}{rendered}")
            } else {
                rendered
            });
        }
        replayed.push(words.join(" "));
    }
    Some(replayed.join("\n"))
}

/// Render a list-prefix element back into a source word. A replacement in head
/// position remains a seeded dynamic value; the ordinary taint pass diagnoses
/// that runtime dispatch without inventing a concrete target.
fn callback_replay_list_word(
    word: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
    input_var: &str,
) -> String {
    if callback_word_has_input(word, inputs) {
        return callback_replay_tainted_word(word, inputs, input_var);
    }
    // The list builder has already reduced each element to one Tcl value. The
    // command head still needs canonical one-word quoting: `{eval foo}` names
    // a single command containing a space and must never be replayed as the
    // builtin `eval` plus an argument.
    tcl_syntax::list::list_element(word)
}

/// Replay source contains only callback text; its input variable is seeded
/// directly in the taint solver, so user definitions of `gets`, `set`, or
/// aliases cannot influence the callback-source model. Every callback runs in
/// its own synthetic procedure: event handlers are independent invocations,
/// not a registration-ordered shared local scope. The solver analyses each
/// retained procedure entry with its explicit taint input; abnormal completion
/// remains inside that callback's own CFG.
fn callback_replay_with_input(
    script: &str,
    proc_name: &str,
    input_var: &str,
) -> CallbackReplaySource {
    CallbackReplaySource {
        definition: format!(
            "::proc {proc_name} {{{input_var}}} {}",
            tcl_syntax::list::list_element(script),
        ),
    }
}

/// Whether a logical callback word contains an authorised, unescaped `%`
/// marker. Metadata replacements remain ordinary literal text.
fn callback_word_has_input(text: &str, inputs: &[tcl_registry::CallbackTaintInput]) -> bool {
    inputs.iter().copied().any(|input| input.occurs_in(text))
}

/// User-facing label for a callback replay source. A callback with exactly
/// one declared replacement keeps that useful spelling; a mixed callback is
/// deliberately described generically rather than pretending every value was
/// the first marker seen.
pub(super) fn callback_input_label(
    text: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
) -> Option<String> {
    let markers: Vec<&str> = inputs
        .iter()
        .copied()
        .filter(|input| input.occurs_in(text))
        .map(tcl_registry::CallbackTaintInput::spelling)
        .collect();
    match markers.as_slice() {
        [] => None,
        [marker] => Some(format!("Tk callback input {marker}")),
        _ => Some("Tk callback input".to_owned()),
    }
}

/// Render one callback-script word without changing its substitution shape.
///
/// The segmenter's logical `word` value alone cannot distinguish a dynamic
/// quoted word (`"puts $x"`) from literal list data (`{puts $x}`).  Its
/// lossless fragments can: variable and command fragments remain executable,
/// while every literal byte is escaped inside one quoted Tcl word.  A word
/// with no substitution stays a canonical literal list element.
fn callback_replay_script_word(
    word: &str,
    fragments: &[tcl_source::segment::WordFragment],
    escapes: tcl_dialect::EscapeSyntax,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<String> {
    if !fragments
        .iter()
        .any(|fragment| matches!(fragment.token.kind, TokenType::Var | TokenType::Cmd))
    {
        return callback_static_word_value(word, fragments, escapes, rules)
            .map(|value| tcl_syntax::list::list_element(&value));
    }

    let mut out = String::with_capacity(word.len() + 2);
    out.push('"');
    for fragment in fragments {
        match fragment.token.kind {
            TokenType::Var | TokenType::Cmd => out.push_str(&fragment.text),
            TokenType::Esc => {
                let decoded = decoded_text_fragment(&fragment.text, escapes)?;
                for ch in decoded.chars() {
                    if matches!(ch, '\\' | '"' | '$' | '[') {
                        out.push('\\');
                    }
                    out.push(ch);
                }
            }
            // A braced literal cannot legally concatenate with a substitution
            // in one Tcl word; recovery-shaped input is not safe to replay.
            _ => return None,
        }
    }
    out.push('"');
    Some(out)
}

/// Text-only analysis must decline native byte escapes that are not UTF-8.
/// The shared decoder retains bytes before this checked projection.
pub(super) fn decoded_text_fragment(
    text: &str,
    escapes: tcl_dialect::EscapeSyntax,
) -> Option<String> {
    let bytes = tcl_lexer::backslash_subst_bytes_in(text.as_bytes(), escapes);
    std::str::from_utf8(&bytes).ok().map(str::to_owned)
}

/// Exact runtime value of one word containing no variable/command
/// substitutions. Braced words preserve backslashes except Tcl's universal
/// backslash-newline continuation; bare/quoted words decode them under the
/// document release's escape grammar.
pub(super) fn callback_static_word_value(
    word: &str,
    fragments: &[tcl_source::segment::WordFragment],
    escapes: tcl_dialect::EscapeSyntax,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<String> {
    match fragments {
        [fragment] if fragment.token.kind == TokenType::Str => {
            Some(rules.collapse_braced_word(word).into_owned())
        }
        fragments
            if fragments
                .iter()
                .all(|fragment| fragment.token.kind == TokenType::Esc) =>
        {
            decoded_text_fragment(word, escapes)
        }
        _ => None,
    }
}

/// Render one non-head word containing a source marker as an interpolated Tcl
/// word. Static bytes are quoted so braces and quotes in the original spelling
/// cannot suppress the symbolic source after Tk's pre-evaluation replacement.
fn callback_replay_tainted_word(
    text: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
    input_var: &str,
) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + input_var.len() + 4);
    out.push('"');
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && bytes.get(index + 1) == Some(&b'%') {
            out.push_str("%%");
            index += 2;
            continue;
        }
        if bytes[index] == b'%'
            && let Some(input) = inputs
                .iter()
                .find(|input| bytes[index..].starts_with(input.spelling().as_bytes()))
        {
            out.push_str("${");
            out.push_str(input_var);
            out.push('}');
            index += input.spelling().len();
            continue;
        }
        let ch = text[index..]
            .chars()
            .next()
            .expect("index is a UTF-8 boundary while copying callback text");
        match ch {
            '\\' | '"' | '$' | '[' => out.push('\\'),
            _ => {}
        }
        out.push(ch);
        index += ch.len_utf8();
    }
    out.push('"');
    out
}
