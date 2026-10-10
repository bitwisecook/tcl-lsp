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

//! Compile-time evaluator for `[subst -nocommands {template}]`.
//!
//! Used by the lowering hook when we see a `proc $var [subst
//! -nocommands {…}]` shape and want to materialise the body string
//! at compile time instead of deferring to the runtime interpreter.
//!
//! The template's structure is the registry's template-word plan
//! (`docs/design/compiler/value-transfers.md` § *The template-word plan*):
//! the reads it performs outside any script region, and the backslash
//! escapes that materialise. This file only renders that plan over a
//! const-map, matching `tclsh`'s `subst -nocommands`:
//!
//! * a `$var` / `${var}` read takes the const-map's value; a miss refuses
//!   the whole evaluation by returning `None` (the caller keeps the dynamic
//!   dispatch path in that case);
//! * an escape decodes through [`tcl_lexer::backslash_subst`] (`\n \t \xNN
//!   \uNNNN`, octal and continuation-line forms);
//! * `[` and `]` are ordinary characters under `-nocommands`, copied
//!   verbatim while a read or escape inside them still substitutes;
//! * an array read (`$a(b)`), a namespace-qualified one (`$::ns::var`), and
//!   any script region — an array index runs its `[…]` whatever the switches
//!   say — are refused.

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::BuildHasher;

use tcl_registry::value_transfer::TemplateWordPlan;

/// Render a `subst -nocommands` template at compile time from its plan.
///
/// `template` is the braced word's content and `plan` the plan over it,
/// whose spans count the opening brace. Returns the substituted string, or
/// `None` if any condition above refuses the evaluation. Refusal is always
/// safe — the caller falls back to runtime dispatch, preserving the
/// original semantics.
///
/// *`const_map`* maps variable names (without the leading `$`) to their
/// literal string values; `${foo}` and `$foo` read the same name.
#[must_use]
pub fn subst_nocommands<S: BuildHasher>(
    template: &str,
    plan: &TemplateWordPlan,
    const_map: &HashMap<String, String, S>,
) -> Option<String> {
    if !plan.braced || plan.dynamic || !plan.script_regions.is_empty() {
        return None;
    }
    // The plan's spans count the opening brace; the content does not.
    let content = |span: tcl_lexer::Span| -> Option<(usize, usize)> {
        Some((
            usize::try_from(span.start().checked_sub(1)?).ok()?,
            usize::try_from(span.end().checked_sub(1)?).ok()?,
        ))
    };
    let mut pieces: Vec<(usize, usize, Cow<'_, str>)> =
        Vec::with_capacity(plan.reads.len() + plan.escapes.len());
    for read in &plan.reads {
        if read.element.is_some() || read.name.contains("::") {
            return None;
        }
        let (start, end) = content(read.span)?;
        pieces.push((start, end, Cow::Borrowed(const_map.get(&read.name)?)));
    }
    for &escape in &plan.escapes {
        let (start, end) = content(escape)?;
        pieces.push((
            start,
            end,
            tcl_lexer::backslash_subst(template.get(start..end)?),
        ));
    }
    pieces.sort_by_key(|&(start, end, _)| (start, end));
    let mut out = String::with_capacity(template.len());
    let mut at = 0;
    for (start, end, value) in pieces {
        out.push_str(template.get(at..start)?);
        out.push_str(&value);
        at = end;
    }
    out.push_str(template.get(at..)?);
    Some(out)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    /// `subst -nocommands {template}` rendered through its plan.
    fn subst_nocommands(template: &str, const_map: &HashMap<String, String>) -> Option<String> {
        let registry = tcl_registry::CommandRegistry::build_default();
        let plan = crate::value_transfer::literal_template_plan(
            &registry,
            "subst",
            &["-nocommands", template],
            |index| crate::value_transfer::SourceWord::of(None, index == 1),
        )?;
        super::subst_nocommands(template, &plan, const_map)
    }

    fn map_of(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn simple_var_substitution() {
        let m = map_of(&[("name", "world")]);
        assert_eq!(
            subst_nocommands("hello $name", &m).as_deref(),
            Some("hello world")
        );
    }

    #[test]
    fn braced_var_substitution() {
        let m = map_of(&[("name", "world")]);
        assert_eq!(
            subst_nocommands("hello ${name}", &m).as_deref(),
            Some("hello world")
        );
    }

    #[test]
    fn missing_var_returns_none() {
        let m: HashMap<String, String> = HashMap::new();
        assert!(subst_nocommands("$missing", &m).is_none());
    }

    #[test]
    fn brackets_kept_literal() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(
            subst_nocommands("[cmd arg]", &m).as_deref(),
            Some("[cmd arg]")
        );
    }

    #[test]
    fn unbalanced_bracket_is_literal() {
        // -nocommands disables command substitution, so `[` is an
        // ordinary character and an unclosed `[` is NOT an error
        // (matches tclsh: `subst -nocommands {[unbalanced}` → `[unbalanced`).
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(
            subst_nocommands("[unbalanced", &m).as_deref(),
            Some("[unbalanced")
        );
    }

    #[test]
    fn vars_inside_brackets_are_substituted() {
        // `$field` inside `[...]` still substitutes under
        // -nocommands; only the command is not executed. `\$obj` decodes to
        // a literal `$obj`. Mirrors tclsh:
        //   subst -nocommands {[dict get \$obj $field]} → [dict get $obj email]
        let m = map_of(&[("field", "email")]);
        assert_eq!(
            subst_nocommands(r"[dict get \$obj $field]", &m).as_deref(),
            Some("[dict get $obj email]")
        );
    }

    #[test]
    fn missing_var_inside_brackets_refused() {
        // A variable miss anywhere (even inside brackets) refuses the whole
        // evaluation so the caller keeps the runtime dispatch path.
        let m: HashMap<String, String> = HashMap::new();
        assert!(subst_nocommands("[cmd $missing]", &m).is_none());
    }

    #[test]
    fn backslash_before_multibyte_char() {
        // `\é` must not split the 2-byte `é` and panic; the
        // backslash is dropped and the following char is emitted verbatim.
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands(r"\é", &m).as_deref(), Some("é"));
        assert_eq!(subst_nocommands(r"a\€b", &m).as_deref(), Some("a€b"));
        assert_eq!(subst_nocommands(r"\🎉", &m).as_deref(), Some("🎉"));
    }

    #[test]
    fn array_ref_refused() {
        let m = map_of(&[("a", "x")]);
        assert!(subst_nocommands("$a(idx)", &m).is_none());
    }

    #[test]
    fn namespace_qualified_refused() {
        let m = map_of(&[("a", "x")]);
        assert!(subst_nocommands("$a::b", &m).is_none());
        assert!(subst_nocommands("$::name", &m).is_none());
    }

    #[test]
    fn dollar_dollar_keeps_first_literal() {
        // ``$$`` — first ``$`` followed by ``$`` (not a name char).
        // The first ``$`` stays literal, then the second ``$`` is
        // also followed by end-of-string, so it stays literal too.
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands("$$", &m).as_deref(), Some("$$"));
    }

    #[test]
    fn backslash_n_decoded() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands(r"\n", &m).as_deref(), Some("\n"));
    }

    #[test]
    fn backslash_x_hex() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands(r"\x41", &m).as_deref(), Some("A"));
    }

    #[test]
    fn backslash_x_hex_capped_at_two_digits() {
        // Tcl 9 caps `\x` at two hex digits (TclParseBackslash): the rest of
        // the digit run is literal text.
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands(r"\x41BC", &m).as_deref(), Some("ABC"));
    }

    #[test]
    fn only_raw_lf_is_a_backslash_newline_continuation() {
        // TclParseBackslash recognises raw LF here. A channel may translate
        // CRLF before this parser seam, but raw CR and CRLF remain data.
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands("a\\\n   b", &m).as_deref(), Some("a b"));
        assert_eq!(
            subst_nocommands("a\\\r\n\t b", &m).as_deref(),
            Some("a\r\n\t b")
        );
        assert_eq!(subst_nocommands("a\\\rb", &m).as_deref(), Some("a\rb"));
    }

    #[test]
    fn empty_template() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands("", &m).as_deref(), Some(""));
    }

    #[test]
    fn mixed_var_and_brackets() {
        let m = map_of(&[("name", "foo")]);
        assert_eq!(
            subst_nocommands("$name [list a b]", &m).as_deref(),
            Some("foo [list a b]")
        );
    }

    #[test]
    fn empty_braced_var_refused() {
        let m: HashMap<String, String> = HashMap::new();
        assert!(subst_nocommands("${}", &m).is_none());
    }
}
