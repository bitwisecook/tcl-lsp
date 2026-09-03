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

//! Rendered-value property lattice — string content analysis
//! over SSA.
//!
//! Tracks properties of each SSA value *after* Tcl backslash
//! substitution so downstream consumers (primarily the taint
//! engine) can query things like "does this value contain a
//! forward slash?" or "has it been through a partial unescape?"
//! without re-traversing source text.
//!
//! Uses a reduced product of boolean domains split into two
//! join kinds:
//!
//! * **may** properties (union at phi joins): overapproximate. If
//!   *any* incoming edge has `HAS_FORWARD_SLASH`, the merged
//!   value has it.
//! * **must** properties (intersection at phi joins):
//!   underapproximate. `STARTS_WITH_SLASH` only survives when
//!   *all* incoming edges agree.
//!
//! Provides the lattice types plus the full SSA-walk propagation
//! pass (`propagate_rendered_props`) used by `FunctionUnit` to
//! enrich taint colours (e.g. `STARTS_WITH_SLASH` →
//! `TaintColour::PATH_PREFIXED`, `STARTS_WITH_DASH` absent →
//! `TaintColour::NON_DASH_PREFIXED`).

use std::collections::HashMap;

use bitflags::bitflags;

use tcl_registry::{CommandRegistry, Traits};

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::{SccpResult, cfg_order};
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution_with_config};

bitflags! {
    /// Properties of a rendered (post-backslash-subst) SSA value.
    ///
    /// Split into two groups — "may" properties (union at joins)
    /// and "must" properties (intersection at joins) — tracked
    /// together in one `u32` bitmask.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RenderedProperties: u32 {
        /// No properties known.
        const NONE                = 0;

        // -- may properties (union at joins) --
        /// Rendered literal text contains `/`.
        const HAS_FORWARD_SLASH   = 1 << 0;
        /// Rendered literal text contains `\\` (path separator).
        const HAS_BACKSLASH       = 1 << 1;
        /// Rendered literal text contains `\r` or `\n`.
        const HAS_CRLF            = 1 << 2;
        /// Value contains `$var` or `[cmd]` interpolation.
        const HAS_INTERPOLATION   = 1 << 3;
        /// Rendered text contains already-escaped sequences.
        const HAS_DOUBLE_ESCAPE   = 1 << 4;
        /// Rendered text contains `\x00` / null byte.
        const HAS_NULL            = 1 << 5;
        /// Value has a top-level word break (a literal space / tab not
        /// inside a `$var` / `[cmd]` interpolation) — prose, a protocol
        /// line, or display text, not a single filesystem path token.
        const HAS_LITERAL_SPACE   = 1 << 11;

        // -- provenance bits (propagated explicitly by commands) --
        /// Value passed through `subst` / `URI::decode` / `b64decode`.
        const WAS_UNESCAPED       = 1 << 6;
        /// Value was already `WAS_UNESCAPED` then unescaped again.
        const DOUBLE_UNESCAPED    = 1 << 7;
        /// Value is fully canonical — no residual encoding
        /// (`-normalized` getters, `file normalize`, …).
        const FULLY_NORMALISED    = 1 << 8;

        // -- must properties (intersection at joins) --
        /// First rendered literal character is `/`.
        const STARTS_WITH_SLASH   = 1 << 9;
        /// First rendered literal character is `-`.
        const STARTS_WITH_DASH    = 1 << 10;
    }
}

impl RenderedProperties {
    /// Mask of "may"-join properties.
    pub const MAY_MASK: Self = Self::from_bits_truncate(
        Self::HAS_FORWARD_SLASH.bits()
            | Self::HAS_BACKSLASH.bits()
            | Self::HAS_CRLF.bits()
            | Self::HAS_INTERPOLATION.bits()
            | Self::HAS_DOUBLE_ESCAPE.bits()
            | Self::HAS_NULL.bits()
            | Self::HAS_LITERAL_SPACE.bits(),
    );

    /// Mask of provenance bits.
    pub const PROVENANCE_MASK: Self = Self::from_bits_truncate(
        Self::WAS_UNESCAPED.bits() | Self::DOUBLE_UNESCAPED.bits() | Self::FULLY_NORMALISED.bits(),
    );

    /// Mask of "must"-join properties.
    pub const MUST_MASK: Self =
        Self::from_bits_truncate(Self::STARTS_WITH_SLASH.bits() | Self::STARTS_WITH_DASH.bits());
}

/// Rendered string content properties for a single SSA value.
///
/// `may` bits union at joins (overapproximate); `must` bits
/// intersect at joins (underapproximate). The bottom element has
/// empty `may` and all must-bits set (assume every must-property
/// holds until disproven); the top element is the reverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderedValueProps {
    /// Properties that *may* hold (union at joins).
    pub may: RenderedProperties,
    /// Properties that *must* hold (intersection at joins).
    pub must: RenderedProperties,
}

impl RenderedValueProps {
    /// The lattice bottom — no known may-property, every
    /// must-property assumed until proven otherwise.
    #[must_use]
    pub const fn bottom() -> Self {
        Self {
            may: RenderedProperties::NONE,
            must: RenderedProperties::MUST_MASK,
        }
    }

    /// The lattice top — every may-property conservatively
    /// assumed, no must-property known.
    #[must_use]
    pub const fn top() -> Self {
        Self {
            may: RenderedProperties::MAY_MASK,
            must: RenderedProperties::NONE,
        }
    }
}

impl Default for RenderedValueProps {
    fn default() -> Self {
        Self::bottom()
    }
}

/// Join two lattice values.
///
/// `may` bits use union (overapproximate); `must` bits use
/// intersection (underapproximate).
#[must_use]
pub fn rendered_join(a: RenderedValueProps, b: RenderedValueProps) -> RenderedValueProps {
    RenderedValueProps {
        may: a.may | b.may,
        must: a.must & b.must,
    }
}

/// Analyse a rendered literal string for its property bitmask.
///
/// Sets `HAS_FORWARD_SLASH` / `HAS_BACKSLASH` / `HAS_CRLF` /
/// `HAS_NULL` bits when their character appears. Sets
/// `STARTS_WITH_SLASH` / `STARTS_WITH_DASH` based on the first
/// character. Does *not* set `HAS_INTERPOLATION` — that's the
/// caller's responsibility based on source structure before
/// rendering.
#[must_use]
pub fn analyse_literal(text: &str) -> RenderedValueProps {
    let mut may = RenderedProperties::NONE;
    let bytes = text.as_bytes();
    for &b in bytes {
        match b {
            b'/' => may |= RenderedProperties::HAS_FORWARD_SLASH,
            b'\\' => may |= RenderedProperties::HAS_BACKSLASH,
            b'\r' | b'\n' => may |= RenderedProperties::HAS_CRLF,
            0 => may |= RenderedProperties::HAS_NULL,
            b' ' | b'\t' => may |= RenderedProperties::HAS_LITERAL_SPACE,
            _ => {}
        }
    }
    let mut must = RenderedProperties::NONE;
    if let Some(&first) = bytes.first() {
        if first == b'/' {
            must |= RenderedProperties::STARTS_WITH_SLASH;
        }
        if first == b'-' {
            must |= RenderedProperties::STARTS_WITH_DASH;
        }
    }
    RenderedValueProps { may, must }
}

/// Apply an unescape-step transition.
///
/// If the incoming value had `WAS_UNESCAPED`, the outgoing value
/// gains `DOUBLE_UNESCAPED`. Otherwise sets `WAS_UNESCAPED`.
#[must_use]
pub fn apply_unescape(input: RenderedValueProps) -> RenderedValueProps {
    let mut out = input;
    if out.may.contains(RenderedProperties::WAS_UNESCAPED) {
        out.may |= RenderedProperties::DOUBLE_UNESCAPED;
    } else {
        out.may |= RenderedProperties::WAS_UNESCAPED;
    }
    // Must-bits survive unescaping only when we've proved they do;
    // conservatively clear them.
    out.must = RenderedProperties::NONE;
    out
}

/// Apply a `-normalized` getter — strips all encoding, sets
/// `FULLY_NORMALISED`, clears unescape provenance.
#[must_use]
pub fn apply_normalised(mut input: RenderedValueProps) -> RenderedValueProps {
    input.may.remove(RenderedProperties::WAS_UNESCAPED);
    input.may.remove(RenderedProperties::DOUBLE_UNESCAPED);
    input.may |= RenderedProperties::FULLY_NORMALISED;
    input
}

// Command classification

/// True when the command performs partial unescaping / decoding
/// (`subst`, `URI::decode`, `encoding convertfrom`, …).
///
/// Checked via the `IS_UNESCAPE` trait on the command spec.
fn is_unescape_command(registry: &CommandRegistry, command: &str, _args: &[&str]) -> bool {
    let Some(spec) = registry.get(command) else {
        return false;
    };
    spec.traits.contains(Traits::IS_UNESCAPE)
}

/// True when the command returns filesystem-path content
/// (`pwd`, `file join`, `file normalize`, …).
///
/// Checked via the `RETURNS_PATH` trait on the top-level command
/// spec. Subcommand-specific path-returning metadata is handled by
/// the dispatcher spec rather than per-subcommand flags.
fn is_path_returning(registry: &CommandRegistry, command: &str, _args: &[&str]) -> bool {
    let Some(spec) = registry.get(command) else {
        return false;
    };
    spec.traits.contains(Traits::RETURNS_PATH)
}

/// True when the command is an iRules HTTP getter invoked with
/// `-normalized`, producing a fully-canonical value with no residual
/// encoding.
fn is_normalised_getter(registry: &CommandRegistry, command: &str, args: &[&str]) -> bool {
    let Some(spec) = registry.get(command) else {
        return false;
    };
    spec.traits.contains(Traits::UNNORMALISED_HTTP_GETTER) && args.contains(&"-normalized")
}

// Per-statement evaluation

/// Compute rendered properties for a constant literal value.
#[must_use]
fn evaluate_const(value: &str) -> RenderedValueProps {
    analyse_literal(value)
}

/// Scan a possibly-interpolated source word for rendered-property
/// bits.
///
/// Identifies may-property bits (`HAS_FORWARD_SLASH`, `HAS_BACKSLASH`,
/// `HAS_CRLF`, `HAS_NULL`, `HAS_INTERPOLATION`) from literal/escape
/// segments, and resolves must-property bits (`STARTS_WITH_SLASH`,
/// `STARTS_WITH_DASH`) from the first non-interpolation character.
///
/// A light-weight scan: it treats `$…` and `[…]` spans as interpolation
/// holes and analyses the surrounding literal text. When an interpolation
/// hole occurs before the first literal character the must-bits become
/// unknown (`leading_resolved` stays false).
#[must_use]
fn scan_value_text(text: &str) -> RenderedValueProps {
    let mut may = RenderedProperties::NONE;
    let mut must = RenderedProperties::NONE;
    let mut leading_resolved = false;

    let mut saw_escape = false;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'$' => {
                may |= RenderedProperties::HAS_INTERPOLATION;
                leading_resolved = true;
                i = skip_variable_ref(bytes, i);
            }
            b'[' => {
                may |= RenderedProperties::HAS_INTERPOLATION;
                leading_resolved = true;
                i = skip_command_sub(bytes, i);
            }
            b'\\' => {
                saw_escape = true;
                let escape = scan_escape(text, i, leading_resolved);
                may |= escape.may_add;
                must |= escape.must_add;
                leading_resolved |= escape.starts_leading;
                i += escape.advance;
            }
            b'/' => {
                may |= RenderedProperties::HAS_FORWARD_SLASH;
                if !leading_resolved {
                    must |= RenderedProperties::STARTS_WITH_SLASH;
                    leading_resolved = true;
                }
                i += 1;
            }
            b'-' => {
                if !leading_resolved {
                    must |= RenderedProperties::STARTS_WITH_DASH;
                    leading_resolved = true;
                }
                i += 1;
            }
            b'\r' | b'\n' => {
                may |= RenderedProperties::HAS_CRLF;
                leading_resolved = true;
                i += 1;
            }
            0 => {
                may |= RenderedProperties::HAS_NULL;
                leading_resolved = true;
                i += 1;
            }
            // A top-level word separator in the (unquoted) value means the
            // original source was a multi-word quoted string — prose, a
            // protocol line, display text — not a single path token.  A
            // command substitution `[cmd a b]` stays one CMD token (skipped
            // above via `skip_command_sub`), so its internal spaces do NOT
            // set this.
            b' ' | b'\t' => {
                may |= RenderedProperties::HAS_LITERAL_SPACE;
                leading_resolved = true;
                i += 1;
            }
            // Delimiters — don't contribute a content char but keep scanning.
            b'"' | b'{' | b'}' => {
                i += 1;
            }
            _ => {
                leading_resolved = true;
                i += 1;
            }
        }
    }
    // Double-escape: if rendering the literal text leaves a residual
    // backslash-escape sequence (e.g. `a\\b` → `a\b`, still `\b`), the
    // value was double-escaped.
    if saw_escape && has_double_escape(&tcl_lexer::backslash_subst(text)) {
        may |= RenderedProperties::HAS_DOUBLE_ESCAPE;
    }
    RenderedValueProps { may, must }
}

/// Detect double-escaping: after rendering, the text still contains a
/// backslash followed by a recognised escape character.
#[must_use]
fn has_double_escape(rendered: &str) -> bool {
    // Recognised-escape set (note the literal space and semicolon).
    const ESC_FOLLOW: &[u8] = b"abfnrtv\\{}[]$\"; xuU01234567";
    let bytes = rendered.as_bytes();
    if !bytes.contains(&b'\\') {
        return false;
    }
    bytes
        .windows(2)
        .any(|w| w[0] == b'\\' && ESC_FOLLOW.contains(&w[1]))
}

/// Skip past `$name` or `${...}` starting at the `$` byte.
fn skip_variable_ref(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    if i < bytes.len() && bytes[i] == b'{' {
        i += 1;
        while i < bytes.len() && bytes[i] != b'}' {
            i += 1;
        }
        if i < bytes.len() {
            i += 1; // consume '}'
        }
        return i;
    }
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_alphanumeric() || b == b'_' {
            i += 1;
        } else if b == b':' && i + 1 < bytes.len() && bytes[i + 1] == b':' {
            i += 2;
        } else {
            break;
        }
    }
    i
}

/// Skip past `[...]` command substitution starting at the `[` byte.
/// Nesting is not tracked — downstream passes that care about exact
/// boundaries must re-lex.
fn skip_command_sub(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() && bytes[i] != b']' {
        i += 1;
    }
    if i < bytes.len() {
        i += 1; // consume ']'
    }
    i
}

/// Analyse the backslash escape starting at `bytes[start]`.
struct EscapeScan {
    advance: usize,
    may_add: RenderedProperties,
    must_add: RenderedProperties,
    starts_leading: bool,
}

fn scan_escape(text: &str, start: usize, leading_resolved: bool) -> EscapeScan {
    let bytes = text.as_bytes();
    // A bare trailing backslash renders to a literal `\`.
    if start + 1 >= bytes.len() {
        return EscapeScan {
            advance: 1,
            may_add: RenderedProperties::HAS_BACKSLASH,
            must_add: RenderedProperties::NONE,
            starts_leading: false,
        };
    }

    // Render the *exact* escape via the canonical backslash decoder, then
    // derive the property bits from what it actually produces.  This is the
    // key fix over the old hand-rolled table: a numeric / hex / unicode /
    // octal escape (`\x2f`, `/`, `\057`) renders to `/` and therefore
    // sets `HAS_FORWARD_SLASH` (its absence caused W201 false-negatives).
    let advance = escape_byte_len(bytes, start);
    let rendered = tcl_lexer::backslash_subst(&text[start..start + advance]);

    let mut may_add = RenderedProperties::NONE;
    for c in rendered.bytes() {
        match c {
            b'/' => may_add |= RenderedProperties::HAS_FORWARD_SLASH,
            b'\\' => may_add |= RenderedProperties::HAS_BACKSLASH,
            b'\r' | b'\n' => may_add |= RenderedProperties::HAS_CRLF,
            0 => may_add |= RenderedProperties::HAS_NULL,
            _ => {}
        }
    }

    // Rendered-text starts-with: the escape's first rendered character is
    // the first content char of the word when nothing precedes it.
    let mut must_add = RenderedProperties::NONE;
    if !leading_resolved {
        match rendered.as_bytes().first() {
            Some(b'/') => must_add |= RenderedProperties::STARTS_WITH_SLASH,
            Some(b'-') => must_add |= RenderedProperties::STARTS_WITH_DASH,
            _ => {}
        }
    }

    EscapeScan {
        advance,
        may_add,
        must_add,
        // An escape always renders at least one character, resolving the
        // leading position (even when that char sets no must-bit).
        starts_leading: !rendered.is_empty(),
    }
}

/// Source-byte length of one Tcl backslash escape beginning at
/// `bytes[start]` (which must be `\`).  Matches the per-escape consumption
/// rules of [`tcl_lexer::backslash_subst`] so the exact escape can be
/// sliced out and rendered.
fn escape_byte_len(bytes: &[u8], start: usize) -> usize {
    let Some(&esc) = bytes.get(start + 1) else {
        return 1;
    };
    let count_digits = |max: usize, from: usize, ok: &dyn Fn(u8) -> bool| {
        let mut n = 0;
        while n < max && bytes.get(from + n).is_some_and(|c| ok(*c)) {
            n += 1;
        }
        n
    };
    match esc {
        // `\xHH` (1–2 hex), `\uHHHH` (1–4 hex), `\UHH..` (1–8 hex).
        b'x' => 2 + count_digits(2, start + 2, &|c| c.is_ascii_hexdigit()),
        b'u' => 2 + count_digits(4, start + 2, &|c| c.is_ascii_hexdigit()),
        b'U' => 2 + count_digits(8, start + 2, &|c| c.is_ascii_hexdigit()),
        // Octal `\NNN`: the first digit is at `start+1`; the cap is 3 for a
        // leading 0–3, else 2 (matching `scan_octal_escape`).
        b'0'..=b'7' => {
            let max = if (b'0'..=b'3').contains(&esc) { 3 } else { 2 };
            1 + count_digits(max, start + 1, &|c| (b'0'..=b'7').contains(&c))
        }
        // Line continuation: `\` + newline (+ optional LF after CR) + a run
        // of spaces/tabs, collapsing to a single space.
        b'\n' | b'\r' => {
            let mut j = start + 2;
            if esc == b'\r' && bytes.get(j) == Some(&b'\n') {
                j += 1;
            }
            while bytes.get(j).is_some_and(|c| *c == b' ' || *c == b'\t') {
                j += 1;
            }
            j - start
        }
        // Simple / unknown `\X` — backslash plus the (possibly multibyte
        // UTF-8) char after it.
        _ => 1 + utf8_char_len(esc),
    }
}

/// Byte length of a UTF-8 character from its leading byte.
fn utf8_char_len(first: u8) -> usize {
    match first {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

/// Compute rendered properties for an `AssignValue` RHS word.
///
/// Handles pure variable references (inherit from source), pure
/// command substitutions (classify via the registry), and otherwise
/// falls through to `scan_value_text`.
#[must_use]
fn evaluate_value(
    value: &str,
    uses: &HashMap<Symbol, u32>,
    props: &HashMap<ValueKey, RenderedValueProps>,
    registry: &CommandRegistry,
    ssa: &SsaFunction,
) -> RenderedValueProps {
    let stripped = value.trim();

    // Pure variable reference → inherit from the source.
    if is_pure_var_ref(stripped) {
        let name = normalise_var_name(stripped);
        if let Some(sym) = ssa.var_symbol(name)
            && let Some(&ver) = uses.get(&sym)
            && ver > 0
        {
            return props.get(&(sym, ver)).copied().unwrap_or_default();
        }
        // Version 0 / unseen: the value comes from an enclosing scope
        // or hasn't been defined on this path. Be conservative — every
        // may-property is possible, no must-property is known.
        return unknown_top();
    }

    // Pure command substitution → interpolated value. The source *word*
    // carries no literal slash/backslash/CRLF/space — only the fact that
    // it is interpolated (`HAS_INTERPOLATION`), refined with semantic
    // registry hints.
    // (a minimal baseline, not the conservative lattice top — using `top`
    // here over-reported every may-flag for command-substitution values).
    if let Some((cmd, args)) = parse_command_substitution_with_config(
        stripped,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    ) {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut may = RenderedProperties::HAS_INTERPOLATION;
        if is_path_returning(registry, &cmd, &arg_refs) {
            may |= RenderedProperties::HAS_FORWARD_SLASH;
        }
        if is_unescape_command(registry, &cmd, &arg_refs) {
            may |= RenderedProperties::WAS_UNESCAPED;
            // Escalate to DOUBLE_UNESCAPED when an input use was already
            // WAS_UNESCAPED (and not FULLY_NORMALISED).
            let input_may = collect_use_may(uses, props);
            if input_may.contains(RenderedProperties::WAS_UNESCAPED)
                && !input_may.contains(RenderedProperties::FULLY_NORMALISED)
            {
                may |= RenderedProperties::DOUBLE_UNESCAPED;
            }
        }
        if is_normalised_getter(registry, &cmd, &arg_refs) {
            may |= RenderedProperties::FULLY_NORMALISED;
        }
        return RenderedValueProps {
            may,
            must: RenderedProperties::NONE,
        };
    }

    // Generic word — scan the literal + interpolation pattern.
    scan_value_text(value)
}

/// Lattice value for "unknown content" — every may-property
/// conservatively possible, no must-property proven.
#[must_use]
fn unknown_top() -> RenderedValueProps {
    RenderedValueProps {
        may: RenderedProperties::MAY_MASK,
        must: RenderedProperties::NONE,
    }
}

/// Union the `may` bits of every SSA input used by a statement.
fn collect_use_may(
    uses: &HashMap<Symbol, u32>,
    props: &HashMap<ValueKey, RenderedValueProps>,
) -> RenderedProperties {
    let mut out = RenderedProperties::NONE;
    for (&sym, &ver) in uses {
        if ver == 0 {
            continue;
        }
        if let Some(p) = props.get(&(sym, ver)) {
            out |= p.may;
        }
    }
    out
}

/// Compute rendered properties for a `Call`-defined value.
fn evaluate_call(
    command: &str,
    args: &[String],
    uses: &HashMap<Symbol, u32>,
    props: &HashMap<ValueKey, RenderedValueProps>,
    registry: &CommandRegistry,
) -> RenderedValueProps {
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    // `-normalized` getter: fully canonical, no residual encoding.
    if is_normalised_getter(registry, command, &arg_refs) {
        return RenderedValueProps {
            may: RenderedProperties::MAY_MASK | RenderedProperties::FULLY_NORMALISED,
            must: RenderedProperties::NONE,
        };
    }

    // Track unescape provenance across SSA.
    if is_unescape_command(registry, command, &arg_refs) {
        let input_may = collect_use_may(uses, props);
        let mut result_may = RenderedProperties::MAY_MASK | RenderedProperties::WAS_UNESCAPED;
        if input_may.contains(RenderedProperties::WAS_UNESCAPED)
            && !input_may.contains(RenderedProperties::FULLY_NORMALISED)
        {
            result_may |= RenderedProperties::DOUBLE_UNESCAPED;
        }
        return RenderedValueProps {
            may: result_may,
            must: RenderedProperties::NONE,
        };
    }

    // Path-returning commands contribute HAS_FORWARD_SLASH.
    if is_path_returning(registry, command, &arg_refs) {
        return RenderedValueProps {
            may: RenderedProperties::MAY_MASK | RenderedProperties::HAS_FORWARD_SLASH,
            must: RenderedProperties::NONE,
        };
    }

    // Generic command: conservative top.
    RenderedValueProps::top()
}

/// Infer rendered properties for a statement's def(s).
fn evaluate_def(
    stmt: &Statement,
    uses: &HashMap<Symbol, u32>,
    props: &HashMap<ValueKey, RenderedValueProps>,
    registry: &CommandRegistry,
    ssa: &SsaFunction,
) -> RenderedValueProps {
    match stmt {
        Statement::AssignConst { value, .. } => evaluate_const(value),
        Statement::AssignValue { value, .. } => {
            // Copy-propagate when the word is a pure var ref and we
            // have a known source; otherwise fall through to the
            // scanner. The `evaluate_value` helper already handles
            // both cases and the unescape-escalation logic.
            evaluate_value(value, uses, props, registry, ssa)
        }
        Statement::AssignExpr { .. } | Statement::Incr { .. } => {
            // Numeric result: no path separators, no interpolation.
            RenderedValueProps {
                may: RenderedProperties::NONE,
                must: RenderedProperties::NONE,
            }
        }
        Statement::Call {
            command,
            args,
            defs,
            ..
        } if !defs.is_empty() => evaluate_call(command, args, uses, props, registry),
        // Barriers / anything else we can't reason about → top.
        _ => RenderedValueProps::top(),
    }
}

// SSA walk

/// Run rendered-properties propagation over one SSA function.
///
/// Returns a map from `(variable_name, ssa_version)` to its
/// `RenderedValueProps`. Entries absent from the map are implicitly
/// `RenderedValueProps::bottom()`.
///
/// Iterates blocks in `cfg_order(cfg)` (RPO-first, unreachable
/// tail) for deterministic output; skips blocks / edges the
/// SCCP pass proved dead.
#[must_use]
pub fn propagate_rendered_props(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    registry: &CommandRegistry,
) -> HashMap<ValueKey, RenderedValueProps> {
    let preds = cfg.predecessors();
    let order = cfg_order(cfg);

    let mut props: HashMap<ValueKey, RenderedValueProps> = HashMap::new();

    let set_props = |map: &mut HashMap<ValueKey, RenderedValueProps>,
                     key: ValueKey,
                     candidate: RenderedValueProps,
                     changed: &mut bool| {
        let old = map.get(&key).copied().unwrap_or_default();
        let merged = rendered_join(old, candidate);
        if merged != old {
            map.insert(key, merged);
            *changed = true;
        }
    };

    let mut changed = true;
    while changed {
        changed = false;
        for bn in &order {
            if !sccp.executable_blocks.contains(bn) {
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };

            // Phi joins at non-entry blocks.
            if bn != &cfg.entry {
                let exec_preds: Vec<BlockId> = preds
                    .get(bn)
                    .map(|ps| {
                        ps.iter()
                            .copied()
                            .filter(|p| sccp.executable_edges.contains(&(*p, *bn)))
                            .collect()
                    })
                    .unwrap_or_default();

                for phi in &ssa_block.phis {
                    if exec_preds.is_empty() {
                        continue;
                    }
                    let mut phi_props = RenderedValueProps::bottom();
                    for pred in &exec_preds {
                        let ver = phi.incoming.get(pred).copied().unwrap_or(0);
                        // Version 0 = undefined-on-this-path / enclosing
                        // scope. Model it as top so the merge stays sound
                        // instead of silently narrowing to the other
                        // predecessors' facts.
                        let incoming = if ver == 0 {
                            unknown_top()
                        } else {
                            props.get(&(phi.name, ver)).copied().unwrap_or_default()
                        };
                        phi_props = rendered_join(phi_props, incoming);
                    }
                    set_props(&mut props, (phi.name, phi.version), phi_props, &mut changed);
                }
            }

            // Statements.
            for ssa_stmt in &ssa_block.statements {
                let stmt = &ssa_stmt.statement;
                if matches!(stmt, Statement::Barrier { .. }) {
                    for (&var, &ver) in &ssa_stmt.defs {
                        set_props(
                            &mut props,
                            (var, ver),
                            RenderedValueProps::top(),
                            &mut changed,
                        );
                    }
                    continue;
                }
                for (&var, &ver) in &ssa_stmt.defs {
                    let inferred = evaluate_def(stmt, &ssa_stmt.uses, &props, registry, ssa);
                    set_props(&mut props, (var, ver), inferred, &mut changed);
                }
            }
        }
    }

    props
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_has_all_must_none_may() {
        let b = RenderedValueProps::bottom();
        assert_eq!(b.may, RenderedProperties::NONE);
        assert_eq!(b.must, RenderedProperties::MUST_MASK);
    }

    #[test]
    fn top_is_reverse_of_bottom() {
        let t = RenderedValueProps::top();
        assert_eq!(t.may, RenderedProperties::MAY_MASK);
        assert_eq!(t.must, RenderedProperties::NONE);
    }

    #[test]
    fn default_is_bottom() {
        assert_eq!(RenderedValueProps::default(), RenderedValueProps::bottom());
    }

    #[test]
    fn join_unions_may_intersects_must() {
        let a = RenderedValueProps {
            may: RenderedProperties::HAS_FORWARD_SLASH,
            must: RenderedProperties::STARTS_WITH_SLASH | RenderedProperties::STARTS_WITH_DASH,
        };
        let b = RenderedValueProps {
            may: RenderedProperties::HAS_CRLF,
            must: RenderedProperties::STARTS_WITH_SLASH,
        };
        let j = rendered_join(a, b);
        assert!(j.may.contains(RenderedProperties::HAS_FORWARD_SLASH));
        assert!(j.may.contains(RenderedProperties::HAS_CRLF));
        // Intersection — only STARTS_WITH_SLASH survives.
        assert!(j.must.contains(RenderedProperties::STARTS_WITH_SLASH));
        assert!(!j.must.contains(RenderedProperties::STARTS_WITH_DASH));
    }

    #[test]
    fn analyse_literal_slash() {
        let p = analyse_literal("/etc/hosts");
        assert!(p.may.contains(RenderedProperties::HAS_FORWARD_SLASH));
        assert!(p.must.contains(RenderedProperties::STARTS_WITH_SLASH));
        assert!(!p.must.contains(RenderedProperties::STARTS_WITH_DASH));
    }

    #[test]
    fn analyse_literal_dash() {
        let p = analyse_literal("-flag");
        assert!(p.must.contains(RenderedProperties::STARTS_WITH_DASH));
        assert!(!p.must.contains(RenderedProperties::STARTS_WITH_SLASH));
    }

    #[test]
    fn analyse_literal_crlf_and_null() {
        let p = analyse_literal("line1\r\nline2");
        assert!(p.may.contains(RenderedProperties::HAS_CRLF));
        let p = analyse_literal("a\0b");
        assert!(p.may.contains(RenderedProperties::HAS_NULL));
    }

    #[test]
    fn analyse_literal_plain_string() {
        let p = analyse_literal("hello");
        assert_eq!(p.may, RenderedProperties::NONE);
        assert_eq!(p.must, RenderedProperties::NONE);
    }

    #[test]
    fn command_substitution_value_uses_minimal_baseline() {
        let registry = CommandRegistry::build_default();
        let uses = HashMap::new();
        let props = HashMap::new();
        // No interned variables are referenced by these opaque substitutions, so
        // a bare (block-less) SSA function suffices for symbol resolution.
        let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        // `[list 1 2 3]` is opaque interpolation — only HAS_INTERPOLATION,
        // never the full may-mask (the old over-report).
        let p = evaluate_value("[list 1 2 3]", &uses, &props, &registry, &ssa);
        assert_eq!(p.may, RenderedProperties::HAS_INTERPOLATION);
        assert_eq!(p.must, RenderedProperties::NONE);
        // `[exec ls]` likewise.
        let p = evaluate_value("[exec ls]", &uses, &props, &registry, &ssa);
        assert_eq!(p.may, RenderedProperties::HAS_INTERPOLATION);
    }

    #[test]
    fn double_escaped_literal_word_flags_double_escape() {
        // `a\\b` renders to `a\b`; the residual `\b` is a double-escape.
        let p = scan_value_text("a\\\\b");
        assert!(p.may.contains(RenderedProperties::HAS_BACKSLASH));
        assert!(p.may.contains(RenderedProperties::HAS_DOUBLE_ESCAPE));
        // A bare interpolated word with no escapes has neither.
        let p = scan_value_text("plain");
        assert!(!p.may.contains(RenderedProperties::HAS_DOUBLE_ESCAPE));
    }

    #[test]
    fn analyse_literal_flags_space() {
        let p = analyse_literal("a b");
        assert!(p.may.contains(RenderedProperties::HAS_LITERAL_SPACE));
    }

    #[test]
    fn apply_unescape_first_time_sets_was_unescaped() {
        let out = apply_unescape(RenderedValueProps::bottom());
        assert!(out.may.contains(RenderedProperties::WAS_UNESCAPED));
        assert!(!out.may.contains(RenderedProperties::DOUBLE_UNESCAPED));
    }

    #[test]
    fn apply_unescape_second_time_sets_double_unescaped() {
        let mut input = RenderedValueProps::bottom();
        input.may |= RenderedProperties::WAS_UNESCAPED;
        let out = apply_unescape(input);
        assert!(out.may.contains(RenderedProperties::WAS_UNESCAPED));
        assert!(out.may.contains(RenderedProperties::DOUBLE_UNESCAPED));
    }

    #[test]
    fn apply_normalised_strips_unescape_provenance() {
        let mut input = RenderedValueProps::bottom();
        input.may |= RenderedProperties::WAS_UNESCAPED;
        input.may |= RenderedProperties::DOUBLE_UNESCAPED;
        let out = apply_normalised(input);
        assert!(!out.may.contains(RenderedProperties::WAS_UNESCAPED));
        assert!(!out.may.contains(RenderedProperties::DOUBLE_UNESCAPED));
        assert!(out.may.contains(RenderedProperties::FULLY_NORMALISED));
    }

    // SSA walk tests

    use crate::compilation_unit::CompilationUnit;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn const_with_slash_gets_starts_with_slash() {
        let cu = CompilationUnit::build_for("set path /etc/hosts", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let p = fu
            .rendered_props
            .iter()
            .find(|((n, _), _)| fu.ssa.var_name(*n) == "path")
            .expect("path entry");
        assert!(p.1.may.contains(RenderedProperties::HAS_FORWARD_SLASH));
        assert!(p.1.must.contains(RenderedProperties::STARTS_WITH_SLASH));
    }

    #[test]
    fn const_with_dash_gets_starts_with_dash() {
        let cu = CompilationUnit::build_for("set flag -x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let p = fu
            .rendered_props
            .iter()
            .find(|((n, _), _)| fu.ssa.var_name(*n) == "flag")
            .expect("flag entry");
        assert!(p.1.must.contains(RenderedProperties::STARTS_WITH_DASH));
    }

    #[test]
    fn pure_var_ref_inherits_props() {
        let cu =
            CompilationUnit::build_for("set path /etc/hosts\nset copy $path", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let copy = fu
            .rendered_props
            .iter()
            .find(|((n, _), _)| fu.ssa.var_name(*n) == "copy")
            .expect("copy entry");
        assert!(copy.1.may.contains(RenderedProperties::HAS_FORWARD_SLASH));
        assert!(copy.1.must.contains(RenderedProperties::STARTS_WITH_SLASH));
    }

    #[test]
    fn expr_result_is_numeric_no_props() {
        let cu = CompilationUnit::build_for("set n [expr {1 + 2}]", &registry(), false);
        let fu = cu.function("::top").unwrap();
        // n should have neither forward slash nor starts-with bits.
        if let Some(((_, _), p)) = fu
            .rendered_props
            .iter()
            .find(|((n, _), _)| fu.ssa.var_name(*n) == "n")
        {
            assert!(!p.may.contains(RenderedProperties::HAS_FORWARD_SLASH));
            assert!(!p.must.contains(RenderedProperties::STARTS_WITH_DASH));
        }
    }

    /// `\-flag` (backslash-escaped dash) should still resolve the
    /// leading character to `-`, setting `STARTS_WITH_DASH`.
    #[test]
    fn escape_dash_gets_starts_with_dash() {
        let p = scan_value_text("\\-flag");
        assert!(
            p.must.contains(RenderedProperties::STARTS_WITH_DASH),
            "expected STARTS_WITH_DASH on '\\-flag', got {p:?}",
        );
    }

    /// A hex / octal / unicode escape that renders to `/` must set
    /// `HAS_FORWARD_SLASH` — its absence caused W201 false-negatives.
    #[test]
    fn numeric_escapes_rendering_to_slash_set_forward_slash() {
        for src in ["\\x2f", "\\x2Fetc", "\\057", "\\u002f", "\\U0000002f"] {
            let p = scan_value_text(src);
            assert!(
                p.may.contains(RenderedProperties::HAS_FORWARD_SLASH),
                "expected HAS_FORWARD_SLASH for {src:?}, got {p:?}",
            );
        }
        // A leading `\x2f` also resolves the must-bit STARTS_WITH_SLASH.
        let p = scan_value_text("\\x2fetc");
        assert!(p.must.contains(RenderedProperties::STARTS_WITH_SLASH));
    }

    /// A hex escape that renders to a null / CRLF byte sets the matching
    /// may-bit (it did not before, the table only knew `\0` and `\n`/`\r`).
    #[test]
    fn numeric_escapes_render_null_and_crlf() {
        assert!(
            scan_value_text("a\\x00b")
                .may
                .contains(RenderedProperties::HAS_NULL)
        );
        assert!(
            scan_value_text("a\\x0db")
                .may
                .contains(RenderedProperties::HAS_CRLF)
        );
    }

    /// A non-slash escape (`\t`) renders to a tab — none of the tracked
    /// content bits should be set (the old table over-set `HAS_BACKSLASH`).
    #[test]
    fn tab_escape_sets_no_content_bits() {
        let p = scan_value_text("a\\tb");
        assert!(!p.may.contains(RenderedProperties::HAS_FORWARD_SLASH));
        assert!(!p.may.contains(RenderedProperties::HAS_BACKSLASH));
        assert!(!p.may.contains(RenderedProperties::HAS_CRLF));
    }

    #[test]
    fn phi_join_of_const_slash_and_const_other() {
        // if { ... } { set x /abs } else { set x notslash }
        let cu = CompilationUnit::build_for(
            "if { $::flag } { set x /abs } else { set x notslash }\nputs $x",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        // The phi'd version of x has HAS_FORWARD_SLASH (may-union)
        // but neither STARTS_WITH_SLASH nor STARTS_WITH_DASH
        // (must-intersection).
        let merged = fu
            .rendered_props
            .iter()
            .filter(|((n, _), _)| fu.ssa.var_name(*n) == "x")
            .map(|(_, p)| *p)
            .fold(RenderedValueProps::bottom(), rendered_join);
        assert!(merged.may.contains(RenderedProperties::HAS_FORWARD_SLASH));
    }
}
