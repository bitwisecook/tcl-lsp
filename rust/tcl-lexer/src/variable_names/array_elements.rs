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

//! Array elements: the element variable name of a reference or braced word, and splitting a name into array and element.

use super::variable_references::{
    array_key_is_literal, braced_var_name, normalise_var_name_for_style,
};

/// The **SSA variable name** of a reference or write-target word: the
/// element-qualified form `base(key)` for a *constant-keyed* array element,
/// the bare base otherwise.
///
/// Array elements are independent variables at runtime (each has its own
/// value and intrep), so a constant-keyed element gets its own name — and
/// therefore its own SSA symbol, type-lattice cell, and shimmer tracking.
/// A dynamic key (`arr($i)`) may select any element, so it stays on the
/// conflated base name (the SSA build fans its def over the array's known
/// elements). A `${…}`-braced reference substitutes nothing inside the
/// braces, so its key is always literal — `${arr($i)}` names the element
/// whose key is the two characters `$i`.
///
/// ```
/// use tcl_lexer::variable_names::element_var_name;
/// assert_eq!(element_var_name("$arr(k)"), "arr(k)");
/// assert_eq!(element_var_name("$arr(\\x41)"), "arr", "backslash keys substitute");
/// assert_eq!(element_var_name("arr(k)"), "arr(k)");
/// assert_eq!(element_var_name("$arr($i)"), "arr");
/// assert_eq!(element_var_name("${arr($i)}"), "arr($i)");
/// assert_eq!(element_var_name("${arr}(k)"), "arr");
/// assert_eq!(element_var_name("$plain"), "plain");
/// ```
#[must_use]
pub fn element_var_name(name: &str) -> &str {
    element_var_name_braced(name, false)
}

/// [`element_var_name`] under an explicitly resolved `${…}` close rule.
#[must_use]
pub fn element_var_name_for_style(name: &str, style: tcl_dialect::BracedVarStyle) -> &str {
    element_var_name_braced_for_style(name, false, style)
}

/// [`element_var_name`] for a word whose **delimiters make its content a
/// literal name** — a brace-quoted write target (`set {a($x)} v`) or the
/// `${…}` reference form, both of which suppress every substitution inside.
///
/// With `braced_literal` set, the content **is** the variable name, verbatim:
/// no `$` sigil to strip, no `${…}` form to unwrap, and the array key is
/// literal whatever it spells.  Tcl keeps such a name distinct from the
/// `$`-less one it looks like (tclsh 9.0.4 / 8.6.14, byte-identical):
///
/// ```text
/// set {$n} v ; set {$n}   -> v
/// info exists {$n} / n    -> 1 / 0     (two different variables)
/// set n other ; set {$n}  -> v         (unaffected)
/// set i 5 ; set {arr($i)} 1
/// array names arr         -> {$i}      (the literal key, not `arr(5)`)
/// info exists arr(5)      -> 0
/// ```
///
/// Stripping the sigil here keyed every consumer — defs, reads,
/// W210/W211/W220, rename, semantic highlighting — on the wrong variable.
///
/// ```
/// use tcl_lexer::variable_names::element_var_name_braced;
/// // A brace-quoted word: the content is the name, verbatim.
/// assert_eq!(element_var_name_braced("$n", true), "$n");
/// assert_eq!(element_var_name_braced("${n}", true), "${n}");
/// assert_eq!(element_var_name_braced("a b", true), "a b");
/// assert_eq!(element_var_name_braced("arr($i)", true), "arr($i)");
/// assert_eq!(element_var_name_braced("$arr(k)", true), "$arr(k)");
/// // Unbraced: the ordinary substituting spellings.
/// assert_eq!(element_var_name_braced("$n", false), "n");
/// assert_eq!(element_var_name_braced("$arr($i)", false), "arr");
/// ```
#[must_use]
pub fn element_var_name_braced(name: &str, braced_literal: bool) -> &str {
    element_var_name_braced_for_style(name, braced_literal, tcl_dialect::BracedVarStyle::default())
}

/// [`element_var_name_braced`] under an explicitly resolved `${…}` close rule.
///
/// The brace form is delimited by [`split_braced_var_ref`], i.e. by the shared
/// owner, rather than by a first-`}` scan that answered for 8.x at every
/// release — a harvest keyed on `a{b` where the lexer spanned `a{b}c` drops the
/// use and lets a live write be reported dead.
///
/// ```
/// use tcl_dialect::BracedVarStyle;
/// use tcl_lexer::variable_names::element_var_name_braced_for_style;
/// assert_eq!(
///     element_var_name_braced_for_style("${a{b}c}", false, BracedVarStyle::Tcl9Nesting),
///     "a{b}c",
/// );
/// assert_eq!(
///     element_var_name_braced_for_style("${a{b}c}", false, BracedVarStyle::FirstClose),
///     "a{b",
/// );
/// ```
#[must_use]
pub fn element_var_name_braced_for_style(
    name: &str,
    braced_literal: bool,
    style: tcl_dialect::BracedVarStyle,
) -> &str {
    if braced_literal {
        // The delimiters suppressed every substitution, so the content is the
        // whole name — including a leading `$` and any `(key)`, which is
        // literal by construction.
        return name;
    }
    // `${…}` brace form: the inner text is the whole (literal) name.
    if let Some(inner) = braced_var_name(name, style) {
        return inner;
    }
    let base = name.strip_prefix('$').unwrap_or(name);
    match split_array_name_for_style(name, style) {
        (_, Some(key)) if array_key_is_literal(key) => base,
        _ => normalise_var_name_for_style(name, style),
    }
}

/// [`normalise_var_name`] for a word whose delimiters make its content a
/// literal name — see [`element_var_name_braced`] for the oracle.
///
/// The *base* name, so an array element still loses its `(key)` suffix
/// (`{arr($i)}` is element `$i` of the array `arr`); what a braced word does
/// **not** lose is its substitution sigil, because there was no substitution:
/// `{$n}` names the variable `$n`, and `{$arr(k)}` element `k` of the array
/// `$arr`.
///
/// ```
/// use tcl_lexer::variable_names::normalise_var_name_braced;
/// assert_eq!(normalise_var_name_braced("$n", true), "$n");
/// assert_eq!(normalise_var_name_braced("${n}", true), "${n}");
/// assert_eq!(normalise_var_name_braced("arr($i)", true), "arr");
/// assert_eq!(normalise_var_name_braced("$arr(k)", true), "$arr");
/// assert_eq!(normalise_var_name_braced("$n", false), "n");
/// ```
#[must_use]
pub fn normalise_var_name_braced(name: &str, braced_literal: bool) -> &str {
    normalise_var_name_braced_for_style(
        name,
        braced_literal,
        tcl_dialect::BracedVarStyle::default(),
    )
}

/// [`normalise_var_name_braced`] under an explicitly resolved `${…}` close
/// rule.
#[must_use]
pub fn normalise_var_name_braced_for_style(
    name: &str,
    braced_literal: bool,
    style: tcl_dialect::BracedVarStyle,
) -> &str {
    if !braced_literal {
        return normalise_var_name_for_style(name, style);
    }
    split_array_name_braced_for_style(name, true, style).0
}

/// [`split_array_name`] for a word whose delimiters make its content a
/// literal name — see [`element_var_name_braced`] for the oracle.
///
/// ```
/// use tcl_lexer::variable_names::split_array_name_braced;
/// assert_eq!(split_array_name_braced("arr($i)", true), ("arr", Some("$i")));
/// assert_eq!(split_array_name_braced("$n", true), ("$n", None));
/// assert_eq!(split_array_name_braced("$arr(k)", true), ("$arr", Some("k")));
/// assert_eq!(split_array_name_braced("$arr(k)", false), ("arr", Some("k")));
/// ```
#[must_use]
pub fn split_array_name_braced(name: &str, braced_literal: bool) -> (&str, Option<&str>) {
    split_array_name_braced_for_style(name, braced_literal, tcl_dialect::BracedVarStyle::default())
}

/// [`split_array_name_braced`] under an explicitly resolved `${…}` close rule.
#[must_use]
pub fn split_array_name_braced_for_style(
    name: &str,
    braced_literal: bool,
    style: tcl_dialect::BracedVarStyle,
) -> (&str, Option<&str>) {
    if !braced_literal {
        return split_array_name_for_style(name, style);
    }
    // A braced-literal word carries no sigil to strip, so the element split is
    // the owner's rule applied to the whole word — call it rather than
    // re-deriving it here.
    match split_element_ref(name) {
        Some((array, elem)) => (array, Some(elem)),
        None => (name, None),
    }
}

/// Split a **resolved** variable name into `(array, element)`, or `None` when
/// it names a scalar / whole array.
///
/// This is `TclObjLookupVarEx`'s unparsed-element rule verbatim
/// (`tclVar.c(9.0.4):683-686`): the name is an array element when it is longer
/// than one byte, ends in `)`, and contains a `(`. The array name is
/// everything before the **first** `(` and the element everything between it
/// and the final `)`.
///
/// Note what the rule does *not* require: the `(` may sit at offset 0, so a
/// **zero-length array name** is legal — `set (x) 5` writes element `x` of the
/// array named `""`, exactly as tclsh does. A consumer that adds
/// a "base must be non-empty" test silently demotes those references to
/// ordinary scalars.
///
/// Unlike [`split_array_name`] this takes a name that has already been
/// resolved — no `$` / `${…}` sigil is stripped — so a variable genuinely
/// *named* `$foo(x)` splits as array `$foo`, element `x`.
///
/// ```
/// use tcl_lexer::variable_names::split_element_ref;
/// assert_eq!(split_element_ref("arr(foo)"), Some(("arr", "foo")));
/// assert_eq!(split_element_ref("arr"), None);
/// // Zero-length array name and zero-length element are both legal.
/// assert_eq!(split_element_ref("(x)"), Some(("", "x")));
/// assert_eq!(split_element_ref("arr()"), Some(("arr", "")));
/// assert_eq!(split_element_ref("()"), Some(("", "")));
/// // Not an element reference: no `(`, or nothing after it closes.
/// assert_eq!(split_element_ref(")"), None);
/// assert_eq!(split_element_ref("a(b"), None);
/// ```
#[must_use]
pub fn split_element_ref(name: &str) -> Option<(&str, &str)> {
    // Both halves are cut at an ASCII `(` / `)`, so each offset is a UTF-8
    // character boundary and the byte split re-slices `name` directly.
    let (array, _) = split_element_ref_bytes(name.as_bytes())?;
    let open = array.len();
    Some((&name[..open], &name[open + 1..name.len() - 1]))
}

/// [`split_element_ref`] over raw bytes, for the byte-oriented runtime (a
/// variable name is not required to be UTF-8 there).
#[must_use]
pub fn split_element_ref_bytes(name: &[u8]) -> Option<(&[u8], &[u8])> {
    if name.last() != Some(&b')') {
        return None;
    }
    // C also demands `len > 1`, which is implied here: a one-byte name ending
    // in `)` cannot also contain the `(` this scan requires.
    let open = name.iter().position(|&c| c == b'(')?;
    Some((&name[..open], &name[open + 1..name.len() - 1]))
}

/// Split a Tcl variable reference into `(base, element)`.
///
/// Strips `$` / `${…}` substitution sigils first, then separates the
/// optional `(element)` array-index suffix from the base name.  Returns
/// `(base, None)` for scalar references.  Follows the brace-form rule that
/// `${arr}(foo)` is the scalar `arr` followed by literal `(foo)`, whereas
/// `${arr(foo)}` *is* the array element `arr(foo)`.
///
/// ```
/// use tcl_lexer::variable_names::split_array_name;
/// assert_eq!(split_array_name("arr"), ("arr", None));
/// assert_eq!(split_array_name("arr(foo)"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("$arr(foo)"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("${arr(foo)}"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("${arr}(foo)"), ("arr", None));
/// assert_eq!(split_array_name("${arr}"), ("arr", None));
/// ```
#[must_use]
pub fn split_array_name(name: &str) -> (&str, Option<&str>) {
    split_array_name_for_style(name, tcl_dialect::BracedVarStyle::default())
}

/// [`split_array_name`] under an explicitly resolved `${…}` close rule.
///
/// ```
/// use tcl_dialect::BracedVarStyle;
/// use tcl_lexer::variable_names::split_array_name_for_style;
/// // 9.x: the nested pair is inside the name, so this is the scalar `a{b}c`.
/// assert_eq!(
///     split_array_name_for_style("${a{b}c}", BracedVarStyle::Tcl9Nesting),
///     ("a{b}c", None),
/// );
/// // 8.x: the name ends at the first `}` and `c}` is ordinary word text.
/// assert_eq!(
///     split_array_name_for_style("${a{b}c}", BracedVarStyle::FirstClose),
///     ("a{b", None),
/// );
/// ```
#[must_use]
pub fn split_array_name_for_style(
    name: &str,
    style: tcl_dialect::BracedVarStyle,
) -> (&str, Option<&str>) {
    // `${…}` brace form: only the chars inside the braces are the reference;
    // an `(idx)` *inside* the braces is an element, one *after* `}` is not.
    if let Some(inner) = braced_var_name(name, style) {
        return match split_element_ref(inner) {
            Some((array, elem)) => (array, Some(elem)),
            None => (inner, None),
        };
    }
    // No closing brace — fall through (gated on `"}" in base`).
    let base = name.strip_prefix('$').unwrap_or(name);
    match split_element_ref(base) {
        Some((array, elem)) => (array, Some(elem)),
        None => (base, None),
    }
}

/// Split an editor's incomplete array-name fragment into source prefixes.
/// Neither prefix is an evaluated name, complete reference or variable lookup.
#[must_use]
pub fn array_variable_completion_prefix(partial: &str) -> Option<(&str, &str)> {
    let (name, index) = partial.split_once('(')?;
    (!name.is_empty()).then_some((name, index))
}
