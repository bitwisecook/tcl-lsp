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

//! Specifier tables for `clock format`, `format` and `binary format`, with the byte ruler.

/// Strftime specifier descriptions for clock-format hover.
const CLOCK_SPEC_DESC: &[(char, &str)] = &[
    ('a', "Abbreviated weekday name"),
    ('A', "Full weekday name"),
    ('b', "Abbreviated month name"),
    ('B', "Full month name"),
    ('C', "Century (00–99)"),
    ('d', "Day of month (01–31)"),
    ('D', "Date as %m/%d/%Y"),
    ('e', "Day of month (1–31, no leading zero)"),
    ('H', "Hour (00–23)"),
    ('I', "Hour (01–12)"),
    ('j', "Day of year (001–366)"),
    ('J', "Julian day number"),
    ('k', "Hour (0–23, no leading zero)"),
    ('l', "Hour (1–12, no leading zero)"),
    ('m', "Month (01–12)"),
    ('M', "Minute (00–59)"),
    ('n', "Month number (1–12, no leading zero)"),
    ('p', "AM/PM indicator (uppercase)"),
    ('s', "Seconds since Unix epoch"),
    ('S', "Second (00–59)"),
    ('u', "Day of week (1=Monday–7=Sunday)"),
    ('w', "Day of week (0=Sunday–6=Saturday)"),
    ('x', "Locale date representation"),
    ('y', "2-digit year (00–99)"),
    ('Y', "4-digit year"),
    ('z', "Timezone offset (+hhmm)"),
    ('Z', "Timezone abbreviation"),
    ('%', "Literal percent sign"),
];

/// Look up a clock-format specifier letter's description.
fn clock_spec_desc(letter: char) -> Option<&'static str> {
    CLOCK_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Find every clock-format specifier in `text` —
/// `%[EO]?[a-zA-Z%]`.  Returns each specifier as its source
/// text (including any `%E` / `%O` locale prefix).
pub(super) fn scan_clock_specifiers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        // `%`...
        let start = i;
        i += 1;
        if i < chars.len() && (chars[i] == 'E' || chars[i] == 'O') {
            i += 1;
        }
        if i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '%') {
            i += 1;
            out.push(chars[start..i].iter().collect());
        }
    }
    out
}

/// Render a markdown table of clock-format specifiers found
/// in `text`.
pub(super) fn clock_format_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Clock format string** (strftime-style)\n".to_string()];
    let specs = scan_clock_specifiers(text);
    if specs.is_empty() {
        parts.push("No specifiers found.".to_string());
    } else {
        parts.push("| Specifier | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for spec in specs {
            let last = spec.chars().last().unwrap_or(' ');
            let desc = clock_spec_desc(last).unwrap_or("Unknown");
            let display = if spec.chars().count() == 3 {
                format!("{desc} (locale-modified)")
            } else {
                desc.to_string()
            };
            parts.push(format!("| `{spec}` | {display} |"));
        }
    }
    parts.join("\n")
}

/// `printf`-style format-specifier descriptions for
/// sprintf-hover.
const SPRINTF_SPEC_DESC: &[(char, &str)] = &[
    ('d', "Signed decimal integer"),
    ('i', "Signed decimal integer"),
    ('u', "Unsigned decimal integer"),
    ('o', "Unsigned octal integer"),
    ('x', "Unsigned hexadecimal (lowercase)"),
    ('X', "Unsigned hexadecimal (uppercase)"),
    ('f', "Floating-point (fixed notation)"),
    ('e', "Floating-point (scientific, lowercase)"),
    ('E', "Floating-point (scientific, uppercase)"),
    ('g', "Shorter of %e or %f"),
    ('G', "Shorter of %E or %f"),
    ('s', "String"),
    ('c', "Character (by Unicode code point)"),
    ('%', "Literal percent sign"),
    ('b', "Unsigned binary integer"),
    ('p', "Pointer value"),
];

fn sprintf_spec_desc(letter: char) -> Option<&'static str> {
    SPRINTF_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Scan `text` for sprintf-style format specifiers.  Captures
/// the full specifier as written, e.g. `%05d` or `%-10s`.
/// Scan for `printf`-style specifiers
/// (`%[positional$]?[flags]*[width]?[.prec]?[type]`).
pub(super) fn scan_sprintf_specifiers(
    text: &str,
    profile: &tcl_dialect::DialectProfile,
) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(offset) = bytes[at..].iter().position(|&b| b == b'%') {
        let start = at + offset;
        at = start + 1;
        let mut end = at;
        if let Some(spec) = tcl_syntax::format::parse_spec(bytes, &mut end)
            && tcl_syntax::format::is_verb(spec.verb)
            && tcl_syntax::format::is_available(&spec, profile)
        {
            out.push(text[start..end].to_string());
            at = end;
        }
    }
    out
}

/// Render a markdown table of sprintf-style specifiers in
/// `text`.
pub(super) fn sprintf_format_hover_text(
    text: &str,
    profile: &tcl_dialect::DialectProfile,
) -> String {
    let mut parts: Vec<String> = vec!["**Format string** (sprintf-style)\n".to_string()];
    let specs = scan_sprintf_specifiers(text, profile);
    if specs.is_empty() {
        parts.push("No specifiers found.".to_string());
    } else {
        parts.push("| Specifier | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for spec in specs {
            let type_char = spec.chars().last().unwrap_or(' ');
            let desc = sprintf_spec_desc(type_char).unwrap_or("Unknown");
            parts.push(format!("| `{spec}` | {desc} |"));
        }
    }
    parts.join("\n")
}

/// Describe the value conversions in a selected scan operand.
pub(super) fn scan_format_hover_text(text: &str, profile: &tcl_dialect::DialectProfile) -> String {
    let mut parts = vec!["**Scan format string**".to_owned()];
    let conversions = tcl_syntax::scan::conversion_spans(text, profile);
    if conversions.is_empty() {
        parts.push("No conversions found.".to_owned());
    } else {
        parts.extend([
            "| Conversion | Meaning |".to_owned(),
            "|------------|---------|".to_owned(),
        ]);
        for conversion in conversions {
            let verb = conversion.spec.verb;
            let description = match verb {
                '[' => "Characters from the bracket set",
                'n' => "Number of characters consumed",
                'i' => "Integer with inferred base",
                'c' => "Character code",
                's' => "Whitespace-delimited string",
                'f' | 'e' | 'E' | 'g' | 'G' => "Floating-point number",
                'x' | 'X' => "Hexadecimal integer",
                'o' => "Octal integer",
                'b' => "Binary integer",
                'd' | 'u' => "Decimal integer",
                _ => "Conversion",
            };
            let directive = &text[conversion.percent.start..conversion.conversion.end];
            parts.push(format!("| `{directive}` | {description} |"));
        }
    }
    parts.join("\n")
}

/// Compact type label for the detail table.
fn binary_short_type(letter: char) -> &'static str {
    match letter {
        'a' => "str (null-pad)",
        'A' => "str (space-pad)",
        'b' => "bits lo→hi",
        'B' => "bits hi→lo",
        'h' => "hex lo→hi",
        'H' => "hex hi→lo",
        'c' => "int8",
        's' => "int16 LE",
        'S' => "int16 BE",
        'i' => "int32 LE",
        'I' => "int32 BE",
        'n' => "int32 native",
        'w' => "int64 LE",
        'W' => "int64 BE",
        'm' => "int64 native",
        'r' => "float32 LE",
        'R' => "float32 BE",
        'f' => "float32 native",
        'd' => "float64 native",
        'q' => "float64 LE",
        'Q' => "float64 BE",
        'x' => "pad/skip",
        'X' => "back",
        '@' => "seek",
        't' => "int16 native",
        _ => "?",
    }
}

/// Unit byte size per element for fixed-width binary types.
fn binary_unit_bytes(letter: char) -> Option<u32> {
    match letter {
        'c' => Some(1),
        's' | 'S' | 't' => Some(2),
        'i' | 'I' | 'n' | 'r' | 'R' | 'f' => Some(4),
        'w' | 'W' | 'm' | 'd' | 'q' | 'Q' => Some(8),
        _ => None,
    }
}

/// Specifiers that don't consume a variable / value argument.
fn binary_no_var(letter: char) -> bool {
    matches!(letter, 'x' | 'X' | '@')
}

/// Total byte size for one binary format field, or `None` if
/// unknown (`*` count, `X` move-back, …).
pub(super) fn binary_field_bytes(letter: char, count: u32, star: bool) -> Option<u32> {
    if star {
        return None;
    }
    if let Some(unit) = binary_unit_bytes(letter) {
        // A large-but-parseable count (`d600000000` ⇒ 8 × 600000000) overflows
        // u32; report an unknown size rather than panicking in debug or
        // wrapping to a bogus value in release (issue 182).
        return unit.checked_mul(count);
    }
    match letter {
        'a' | 'A' | 'x' => Some(count),
        'b' | 'B' => Some(count.div_ceil(8)),
        'h' | 'H' => Some(count.div_ceil(2)),
        _ => None,
    }
}

/// One parsed `binary format` / `binary scan` field.
#[derive(Debug, Clone)]
pub(super) struct BinaryField {
    /// The full spec token as written (e.g. `"i4"`, `"a*"`).
    pub(super) full: String,
    /// Type character (e.g. `'i'`).
    letter: char,
    /// Numeric count (defaults to `1` when omitted).
    count: Option<u32>,
    /// Whether the unsigned suffix was selected.
    unsigned: bool,
    /// `true` when the spec used `*` for the count.
    pub(super) star: bool,
    /// Per-unit byte size before seek/skip adjustment.
    pub(super) byte_size: Option<u32>,
    /// `true` when this field consumes a value/variable argument.
    consumes_var: bool,
}

/// Scan a `binary format` / `binary scan` spec string into
/// structured fields.  Tcl grammar: `type [modifier] [count|*]`,
/// repeated.
pub(super) fn scan_binary_fields(
    text: &str,
    profile: &tcl_dialect::DialectProfile,
) -> Vec<BinaryField> {
    tcl_cmd_core::binary::complete_specifiers(text.as_bytes(), profile)
        .unwrap_or_default()
        .into_iter()
        .map(|f| {
            let letter = char::from(f.letter);
            let count = u32::try_from(f.count.unwrap_or(1)).ok();
            BinaryField {
                full: text[f.letter_start..f.end].to_owned(),
                letter,
                count,
                unsigned: f.modifier.is_some(),
                star: f.star,
                byte_size: count.and_then(|n| binary_field_bytes(letter, n, f.star)),
                consumes_var: !binary_no_var(letter),
            }
        })
        .collect()
}

/// Surrounding-command context the binary-hover renderer uses
/// to label fields with variable / value argument names.
#[derive(Debug, Clone)]
pub(super) struct BinaryContext {
    /// Format-string content (between the surrounding quotes
    /// / braces).
    pub(super) text: String,
    /// `"format"` or `"scan"`.
    pub(super) subcmd: String,
    /// Trailing argument tokens (variable names for `scan`,
    /// value expressions for `format`).  Filled best-effort
    /// from the line tokenisation — may be empty.
    pub(super) args: Vec<String>,
}

/// Render the binary format-spec hover markdown, including
/// the byte-ruler diagram when every field has a known byte
/// size, no field uses `X` (move-back), and the total fits in
/// 32 bytes.
pub(super) fn binary_format_hover_text(
    ctx: &BinaryContext,
    profile: &tcl_dialect::DialectProfile,
) -> String {
    let fields = scan_binary_fields(&ctx.text, profile);
    if fields.is_empty() {
        return "**Binary format string**\n\nNo specifiers found.".to_string();
    }

    // Map each consuming field → arg name (variable for scan,
    // value expr for format).  Fields without a corresponding
    // arg fall back to the spec text as their label.
    let mut field_labels: Vec<String> = Vec::with_capacity(fields.len());
    let mut var_idx = 0;
    for field in &fields {
        if field.consumes_var && var_idx < ctx.args.len() {
            field_labels.push(ctx.args[var_idx].clone());
            var_idx += 1;
        } else {
            field_labels.push(field.full.clone());
        }
    }

    // Resolve effective byte deltas, including absolute seek
    // (`@N` jumps to absolute offset N — count the gap from the
    // current cursor).  A backward seek (target < cursor)
    // disables the diagram entirely.
    let mut effective_bytes: Vec<Option<u32>> = Vec::with_capacity(fields.len());
    let mut cursor: u64 = 0;
    let mut has_backward_seek = false;
    for field in &fields {
        if field.letter == '@' {
            let Some(target) = field.count.filter(|_| !field.star).map(u64::from) else {
                effective_bytes.push(None);
                continue;
            };
            if target < cursor {
                effective_bytes.push(Some(0));
                has_backward_seek = true;
            } else {
                effective_bytes.push(u32::try_from(target - cursor).ok());
            }
            cursor = target;
            continue;
        }
        match field.byte_size {
            Some(bs) => {
                effective_bytes.push(Some(bs));
                cursor += u64::from(bs);
            }
            None => effective_bytes.push(None),
        }
    }

    let n_vars = fields.iter().filter(|f| f.consumes_var).count();
    let total_known: u64 = effective_bytes
        .iter()
        .filter_map(|bs| bs.map(u64::from))
        .sum();
    let has_unknown = effective_bytes.iter().any(Option::is_none);
    let plural = if n_vars == 1 { "" } else { "s" };
    let size_suffix = if has_unknown { "+ " } else { "" };

    let mut parts: Vec<String> = vec![format!(
        "**binary {}** — {n_vars} field{plural}, {total_known}{size_suffix} bytes\n",
        ctx.subcmd
    )];

    // Byte-ruler diagram — skipped when any field has unknown
    // size, when a backward seek scrambled the offsets, or when
    // the total exceeds the 32-byte rendering budget.
    let can_diagram = !has_backward_seek
        && !effective_bytes.is_empty()
        && effective_bytes
            .iter()
            .all(|bs| matches!(bs, Some(n) if *n > 0));
    if can_diagram && (1..=32).contains(&total_known) {
        parts.push("```".to_string());
        parts.extend(render_byte_ruler(
            &fields,
            &effective_bytes,
            &field_labels,
            u32::try_from(total_known).expect("diagram is limited to 32 bytes"),
        ));
        parts.push("```\n".to_string());
    }

    // Detail table — Spec / Variable / Type / Bytes.
    parts.push("| Spec | Variable | Type | Bytes |".to_string());
    parts.push("|------|----------|------|------:|".to_string());
    for (j, field) in fields.iter().enumerate() {
        let var = if field.consumes_var {
            field_labels[j].as_str()
        } else {
            "—"
        };
        let mut typ = binary_short_type(field.letter).to_string();
        if field.unsigned {
            typ = typ.replace("int", "uint");
        }
        if let Some(count) = field.count.filter(|&n| n > 1)
            && binary_unit_bytes(field.letter).is_some()
        {
            typ = format!("{typ} ×{count}");
        }
        let bs_str = if field.star {
            "…".to_string()
        } else {
            effective_bytes[j].map_or_else(|| "?".to_string(), |n| n.to_string())
        };
        parts.push(format!("| `{}` | {var} | {typ} | {bs_str} |", field.full));
    }
    parts.join("\n")
}

/// Render the four-line byte-ruler diagram: a numeric ruler
/// across the byte axis, then top / middle / bottom rows of
/// box-drawing characters labelling each field.  `total_known`
/// is guaranteed to be in `1..=32` (gated by the caller).
fn render_byte_ruler(
    fields: &[BinaryField],
    effective_bytes: &[Option<u32>],
    field_labels: &[String],
    total_known: u32,
) -> Vec<String> {
    use std::fmt::Write;
    const CPB: u32 = 4; // chars per byte
    let indent = "      ";
    let mut ruler = String::from(indent);
    for b in 0..total_known {
        let _ = write!(ruler, "{b:<width$}", width = CPB as usize);
    }
    let mut top = String::from(indent);
    let mut mid = String::from(indent);
    let mut bot = String::from(indent);
    for j in 0..fields.len() {
        let bs = effective_bytes[j].expect("caller gates on all-Some");
        let w = (CPB * bs).saturating_sub(1) as usize;
        let label = field_labels[j].chars().take(w).collect::<String>();
        let sep_t = if j == 0 { '┌' } else { '┬' };
        let sep_b = if j == 0 { '└' } else { '┴' };
        top.push(sep_t);
        top.push_str(&"─".repeat(w));
        mid.push('│');
        mid.push_str(&center(&label, w));
        bot.push(sep_b);
        bot.push_str(&"─".repeat(w));
    }
    top.push('┐');
    mid.push('│');
    bot.push('┘');
    vec![ruler, top, mid, bot]
}

/// Center `s` within a `width`-character cell using spaces (the
/// byte-ruler labels).
fn center(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_string();
    }
    let extra = width - len;
    let left = extra / 2;
    let right = extra - left;
    let mut out = String::with_capacity(width);
    for _ in 0..left {
        out.push(' ');
    }
    out.push_str(s);
    for _ in 0..right {
        out.push(' ');
    }
    out
}
