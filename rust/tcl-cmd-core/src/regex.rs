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

//! `regexp` / `regsub` command **plumbing**, shared over a [`RegexEngine`]
//! provider and [`ValueOps`](tcl_syntax::value::ValueOps).
//!
//! The engine compiles patterns and reports character-unit match offsets.
//! Concrete original-object adapters select the actual native C recipe,
//! retain executable compiled patterns on the original header, and reach
//! native counted Unicode storage for subjects and replacement operands.
//! Range construction and match-variable assignments preserve original
//! storage and callback order. Original cache hits precede string getters.
//!
//! Compatibility byte APIs explicitly decode UTF-8 and do not issue native
//! `RegExp`, object-identity or cache authority. An engine without the exact
//! native-unit compilation doorway returns a typed capability refusal.

use tcl_dialect::TclVersion;
use tcl_syntax::value::{ValueError, ValueOps};

use crate::prefix::OptionTable;

/// The "did not participate" sentinel for a subexpression's offset (mirrors the
/// engine's `(size_t)-1`).
pub const NO_MATCH: usize = usize::MAX;

/// One reported (sub-)match: half-open `[so, eo)` in **character** offsets.
/// `so == NO_MATCH` means the subexpression did not participate.
#[derive(Clone, Copy)]
pub struct RegMatch {
    pub so: usize,
    pub eo: usize,
}

/// The compile-time options that affect matching, in an engine-neutral form
/// (each provider maps these to its own flags). `-line` sets both `linestop`
/// and `lineanchor`.
#[derive(Default, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // option flags, not a state machine
pub struct RegexFlags {
    /// `-nocase` — case-insensitive.
    pub nocase: bool,
    /// `-expanded` — whitespace/comments in the pattern are ignored.
    pub expanded: bool,
    /// `-linestop` — `.` and `[^…]` stop at a newline.
    pub linestop: bool,
    /// `-lineanchor` — `^`/`$` match at line boundaries.
    pub lineanchor: bool,
    /// `\z` is an end-of-string anchor ([`TclVersion::regex_z_anchor`]).
    pub z_anchor: bool,
    /// Native LSEARCH suppresses subexpression storage on its first compilation.
    pub nosub: bool,
}

impl RegexFlags {
    /// Exact command compile configuration key; this is not an ABI flag word.
    #[must_use]
    pub const fn cache_key(self) -> u32 {
        (self.nocase as u32)
            | ((self.expanded as u32) << 1)
            | ((self.linestop as u32) << 2)
            | ((self.lineanchor as u32) << 3)
            | ((self.z_anchor as u32) << 4)
            | ((self.nosub as u32) << 5)
    }

    /// No switches, with the escapes `version` accepts.
    #[must_use]
    pub const fn for_release(version: TclVersion) -> Self {
        Self {
            nocase: false,
            expanded: false,
            linestop: false,
            lineanchor: false,
            z_anchor: version.regex_z_anchor(),
            nosub: false,
        }
    }
}

/// A compiled-regex provider. Stateless at the type level (compiling is a pure
/// function of pattern + flags), so the plumbing is generic over it without ever
/// holding a provider borrow across the match loop.
pub trait RegexEngine {
    /// The provider's compiled-pattern handle.
    type Regex;

    /// Compile `pattern` (UTF-8 bytes) with `flags`. On failure, return the
    /// engine's error *detail* bytes (the plumbing adds the standard prefix).
    ///
    /// # Errors
    /// The engine's compile-error detail for a malformed pattern.
    fn compile(pattern: &[u8], flags: RegexFlags) -> Result<Self::Regex, Vec<u8>>;

    /// Compile the independently selected bundled Jim `CString` engine.
    #[must_use]
    fn compile_jim(_pattern: &[u8], _flags: RegexFlags) -> Option<Result<Self::Regex, Vec<u8>>> {
        None
    }
    /// Execute the bundled Jim engine with byte offsets and capture count.
    fn exec_jim(
        _pattern: &mut Self::Regex,
        _subject: &[u8],
        _captures: usize,
        _notbol: bool,
    ) -> Result<Option<Vec<RegMatch>>, &'static str> {
        Err("bundled Jim regexp engine unavailable")
    }

    /// Compile exact native character units without a Unicode text projection.
    /// None means this engine cannot accept the selected counted unit input.
    #[must_use]
    fn compile_units(_pattern: &[u32], _flags: RegexFlags) -> Option<Result<Self::Regex, Vec<u8>>> {
        None
    }

    /// Number of capturing subexpressions (so the whole match plus this many).
    fn nsub(re: &Self::Regex) -> usize;

    /// The `re_info` flag names the engine recorded while compiling, in
    /// `re_info` bit order — the second element of `regexp -about`
    /// (`REG_UBACKREF`, `REG_ULOOKAHEAD`, `REG_UBOUNDS`, `REG_UBRACES`,
    /// `REG_UBSALNUM`, `REG_UPBOTCH`, `REG_UBBS`, `REG_UNONPOSIX`,
    /// `REG_UUNSPEC`, `REG_UUNPORT`, `REG_ULOCALE`, `REG_UEMPTYMATCH`,
    /// `REG_UIMPOSSIBLE`, `REG_USHORTEST`; `TclRegAbout` in `tclRegexp.c`).
    ///
    /// Defaulted to "none recorded" rather than made required: `re_info` is
    /// the compiler's own bookkeeping — which constructs the pattern used —
    /// and nothing out here can recompute it from the pattern bytes without
    /// being a second ARE parser. An engine that tracks it (the Tcl ARE engine
    /// does, as `tcl_regex::Regex::info`) overrides this; one that does not
    /// still answers `-about` with the right subexpression count, which is the
    /// half of the answer every caller actually branches on.
    fn info_names(_re: &Self::Regex) -> Vec<&'static str> {
        Vec::new()
    }

    /// Find the leftmost match in `cps` (the whole subject as codepoints) at or
    /// after character `offset`. `notbol` requests that `^` not match at
    /// `offset` (the FFI engine's `REG_NOTBOL`; a context-aware crate engine can
    /// ignore it). Returns the match vector (index 0 = whole match, then each
    /// subexpression) in **absolute** character offsets, or `None` on no match.
    fn exec(
        re: &mut Self::Regex,
        cps: &[i32],
        offset: usize,
        notbol: bool,
    ) -> Option<Vec<RegMatch>>;
}

/// A regex command failure retaining its full portable command-error receipt.
#[derive(Debug)]
pub struct RegexError(crate::CmdError);

impl RegexError {
    /// Neutral regex result bytes, without an inferred semantic error identity.
    #[must_use]
    pub fn new(message: impl Into<Vec<u8>>) -> Self {
        Self(crate::CmdError::new_bytes(message))
    }

    /// Exact guest diagnostic bytes; host refusal remains in the retained receipt.
    #[must_use]
    pub fn message_bytes(&self) -> &[u8] {
        self.0.message_bytes()
    }

    /// Consume every error-code, primitive state and host-access obligation.
    #[must_use]
    pub fn into_cmd_error(self) -> crate::CmdError {
        self.0
    }
}

impl From<crate::CmdError> for RegexError {
    fn from(error: crate::CmdError) -> Self {
        Self(error)
    }
}

/// The outcome of [`regexp`] for the adapter to apply (var writes stay in the
/// adapter — they are Family-B state).
pub enum RegexpResult<V> {
    /// `-inline`: set the command result to this list value (no var writes).
    Inline(V),
    /// Non-inline: set the command result to the integer `count`. If `assign` is
    /// `Some`, also write each `(var-name, value)` first (a match occurred);
    /// `None` means no match — the match variables are left **untouched** (tclsh
    /// does not modify them on a failed match).
    Count {
        assign: Option<Vec<(Vec<u8>, V)>>,
        count: i64,
    },
}

/// The outcome of [`regsub`]: the substituted text, the substitution count, and
/// the optional result-variable name (the adapter sets the var or returns the
/// text, and owns the const-variable check).
pub struct RegsubResult {
    pub text: Vec<u8>,
    pub count: i64,
    pub var: Option<Vec<u8>>,
}

/// Decode UTF-8 `bytes` into codepoints plus a parallel byte-offset table. The
/// returned `offsets` has length `codepoints.len() + 1`: `offsets[i]` is the
/// byte index where codepoint `i` starts, the final entry being `bytes.len()` —
/// so a character range `[so, eo)` slices the original bytes as
/// `bytes[offsets[so]..offsets[eo]]`. Invalid sequences decode to U+FFFD.
#[must_use]
pub fn decode_utf8(bytes: &[u8]) -> (Vec<i32>, Vec<usize>) {
    let mut cps = Vec::with_capacity(bytes.len());
    let mut offs = Vec::with_capacity(bytes.len() + 1);
    let mut i = 0;
    while i < bytes.len() {
        offs.push(i);
        let b0 = bytes[i];
        let (cp, n) = if b0 < 0x80 {
            (u32::from(b0), 1)
        } else if b0 & 0xE0 == 0xC0 {
            (u32::from(b0 & 0x1F), 2)
        } else if b0 & 0xF0 == 0xE0 {
            (u32::from(b0 & 0x0F), 3)
        } else if b0 & 0xF8 == 0xF0 {
            (u32::from(b0 & 0x07), 4)
        } else {
            (0xFFFD, 1)
        };
        let mut cp = cp;
        let mut taken = 1;
        if n > 1 && i + n <= bytes.len() {
            let mut ok = true;
            for j in 1..n {
                let b = bytes[i + j];
                if b & 0xC0 != 0x80 {
                    ok = false;
                    break;
                }
                cp = (cp << 6) | u32::from(b & 0x3F);
                taken += 1;
            }
            if !ok || taken != n {
                cp = 0xFFFD;
                taken = 1;
            }
        } else if n > 1 {
            cp = 0xFFFD;
            taken = 1;
        }
        // A codepoint is at most 0x10FFFF, well within i32 range.
        #[allow(clippy::cast_possible_wrap)]
        cps.push(cp as i32);
        i += taken;
    }
    offs.push(bytes.len());
    (cps, offs)
}

/// Tcl's per-iteration `REG_NOTBOL` rule: set unless `offset` is the very start
/// or follows a newline (so `^` behaves correctly in `-line` mode and at resumed
/// offsets).
fn notbol_at(cps: &[i32], offset: usize) -> bool {
    if offset == 0 {
        false
    } else if offset > cps.len() {
        true
    } else {
        cps[offset - 1] != i32::from(b'\n')
    }
}

/// Resolve a `-start` index spec (integer / `end` / `end±N`) against the
/// character length, clamped to `0` (Tcl resets negatives to the start).
///
/// `N` in `end±N` is a user-supplied integer, so `len - 1 ± N` is done with
/// `saturating_*`: `end+9999999999999999999` would otherwise overflow `isize`
/// and wrap to a bogus (possibly in-range) offset. Saturating pins it to the
/// `isize` extremes — a too-large `+N` becomes "past the end" (the match loop's
/// `offset >= char_len` check then yields no match) and a too-large `-N` becomes
/// the start, both the intended clamp behaviour.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
fn resolve_start_checked<O: ValueOps>(
    ops: &mut O,
    spec: &[u8],
    char_len: usize,
) -> Result<usize, RegexError> {
    let text = core::str::from_utf8(spec).map_err(|_| RegexError::new(b"bad index".to_vec()))?;
    let syntax = ops
        .index_syntax()
        .ok_or_else(|| RegexError::new(b"regex offset dialect is not selected".to_vec()))?;
    let idx = if syntax.regex_start_grammar() == tcl_dialect::RegexStartGrammar::Integer {
        let flags = tcl_syntax::number::ParseFlags {
            integer_only: true,
            ..tcl_syntax::number::ParseFlags::for_syntax(syntax.numbers)
        };
        if tcl_syntax::number::parse_whole_with(text, flags).is_none() {
            return Err(RegexError::new(
                format!("expected integer but got \"{text}\"").into_bytes(),
            ));
        }
        crate::index::resolve_in(text, char_len, syntax)
    } else {
        crate::index::resolve_for_ops(ops, text, char_len)
    }
    .map_err(RegexError::from)?;
    Ok(usize::try_from(idx).unwrap_or(0))
}

/// The compile flags + `-all`/`-start` shared by both commands' option sets.
struct Common {
    all: bool,
    flags: RegexFlags,
    start: Option<Vec<u8>>,
}

impl Common {
    const fn for_release(version: TclVersion) -> Self {
        Self {
            all: false,
            flags: RegexFlags::for_release(version),
            start: None,
        }
    }
}

fn wrong_args(usage: &[u8]) -> RegexError {
    crate::CmdError::wrong_args_bytes(usage).into()
}

/// Wrap a provider compile-error detail in `version`'s standard prefix.
fn compile_error(version: TclVersion, detail: &[u8]) -> RegexError {
    let mut m = version.regex_compile_error_prefix().as_bytes().to_vec();
    m.extend_from_slice(detail);
    RegexError::new(m)
}

const REGEXP_USAGE: &[u8] = b"regexp ?-option ...? exp string ?matchVar? ?subMatchVar ...?";

// C's `options[]` in `Tcl_RegexpObjCmd` (`tclCmdMZ.c`), matched with
// `TCL_EXACT`: abbreviations are rejected, so `regexp -no …` is a bad option
// here. tclsh's bytecode compiler (`TclCompileRegexpCmd`) separately accepts
// any two-plus-character prefix of `-nocase` in its no-match-variable fast
// path `regexp ?-nocase? ?--? exp string`, so that one form abbreviates in
// tclsh scripts; every other form (match variables, any other option, an
// `eval`'d word list) reaches the runtime command and is exact-only. We
// implement the runtime semantics everywhere (probed tclsh 8.6.14; 9.0.4
// source).
const RE_ALL: usize = 0;
const RE_ABOUT: usize = 1;
const RE_INDICES: usize = 2;
const RE_INLINE: usize = 3;
const RE_EXPANDED: usize = 4;
const RE_LINE: usize = 5;
const RE_LINESTOP: usize = 6;
const RE_LINEANCHOR: usize = 7;
const RE_NOCASE: usize = 8;
const RE_START: usize = 9;
static REGEXP_NAMES: [&str; 11] = [
    "-all",
    "-about",
    "-indices",
    "-inline",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-nocase",
    "-start",
    "--",
];
static REGEXP_OPTIONS: OptionTable<'static> = OptionTable::exact_only("option", &REGEXP_NAMES);

/// Drive `regexp` over the engine `E` and value-ops `O` for `version`. `args`
/// is the command's arguments **without** the command name.
///
/// # Errors
/// Option/arg/compile errors as ready-to-report [`RegexError`] messages.
#[allow(clippy::too_many_lines)] // option scan + match loop + result build, read top-to-bottom
// Both byte compatibility entry points and native original-object callers
// use this scanner. Physical Index selection is owned by ValueOps, never by
// reconstructed option strings.
trait OptionArguments {
    fn len(&self) -> usize;
    fn bytes(&mut self, index: usize) -> Result<Vec<u8>, RegexError>;
    fn option(&mut self, index: usize, table: &OptionTable<'static>) -> Result<usize, RegexError>;
}

struct ByteOptionArguments<'a>(&'a [&'a [u8]]);
impl OptionArguments for ByteOptionArguments<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn bytes(&mut self, index: usize) -> Result<Vec<u8>, RegexError> {
        Ok(self.0[index].to_vec())
    }
    fn option(&mut self, index: usize, table: &OptionTable<'static>) -> Result<usize, RegexError> {
        table.index_of(self.0[index]).map_err(RegexError::new)
    }
}

struct OriginalOptionArguments<'a, O: ValueOps> {
    ops: &'a mut O,
    args: &'a [O::Value],
}
impl<O: ValueOps> OptionArguments for OriginalOptionArguments<'_, O> {
    fn len(&self) -> usize {
        self.args.len()
    }
    fn bytes(&mut self, index: usize) -> Result<Vec<u8>, RegexError> {
        self.ops
            .native_string_bytes(&self.args[index])
            .map(|bytes| bytes.to_vec())
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))
    }
    fn option(&mut self, index: usize, table: &OptionTable<'static>) -> Result<usize, RegexError> {
        table
            .index_of_original(self.ops, &self.args[index])
            .map_err(RegexError::from)
    }
}

fn regexp_option_scan(
    args: &mut impl OptionArguments,
    version: TclVersion,
) -> Result<(Common, bool, bool, bool, usize), RegexError> {
    let mut c = Common::for_release(version);
    let mut indices = false;
    let mut inline = false;
    let mut about = false;

    let mut i = 0;
    while i < args.len() {
        let name = args.bytes(i)?;
        if name.first() != Some(&b'-') {
            break;
        }
        let idx = args.option(i, &REGEXP_OPTIONS)?;
        i += 1;
        match idx {
            RE_ALL => c.all = true,
            RE_ABOUT => about = true,
            RE_INDICES => indices = true,
            RE_INLINE => inline = true,
            RE_EXPANDED => c.flags.expanded = true,
            RE_LINE => {
                c.flags.linestop = true;
                c.flags.lineanchor = true;
            }
            RE_LINESTOP => c.flags.linestop = true,
            RE_LINEANCHOR => c.flags.lineanchor = true,
            RE_NOCASE => c.flags.nocase = true,
            RE_START => match (i < args.len()).then(|| args.bytes(i)).transpose()? {
                Some(v) => {
                    c.start = Some(v);
                    i += 1;
                }
                // A trailing `-start` ends the options (C's `goto
                // endOfForLoop`); the arity check below then reports it.
                None => break,
            },
            // `--`: explicit end of options.
            _ => break,
        }
    }

    Ok((c, indices, inline, about, i))
}

/// Regex preparation or an original variable setter completion. Setter failures
/// stop the current match before any later variable or match is evaluated.
#[derive(Debug)]
pub enum RegexpExecutionError<E> {
    Regex(RegexError),
    Assignment(E),
}

/// Resolve original options and assign each original target inside the match
/// loop. Unmatched, inline and about paths never materialise target objects.
pub fn regexp_original<O: NativeRegexObjects<E>, E: RegexEngine, Err>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
    mut assign: impl FnMut(&mut O, &O::Value, O::Value) -> Result<(), Err>,
) -> Result<RegexpResult<O::Value>, RegexpExecutionError<Err>> {
    if ops
        .jim_regex_recipe()
        .map_err(|error| {
            RegexpExecutionError::Regex(RegexError::from(crate::CmdError::from(error)))
        })?
        .is_some()
    {
        return regexp_jim_original::<O, E, Err>(ops, args, version, assign);
    }
    let (c, indices, inline, about, offset) =
        regexp_option_scan(&mut OriginalOptionArguments { ops, args }, version)
            .map_err(RegexpExecutionError::Regex)?;
    let remaining = &args[offset..];
    // Validate arity before reaching any non-option getter.
    if remaining.len() + usize::from(about) < 2 {
        return Err(RegexpExecutionError::Regex(wrong_args(REGEXP_USAGE)));
    }
    if inline && remaining.len() != 2 {
        return Err(RegexpExecutionError::Regex(RegexError::new(
            b"regexp match variables not allowed when using -inline".to_vec(),
        )));
    }
    let recipe = ops.regex_recipe().map_err(|error| {
        RegexpExecutionError::Regex(RegexError::from(crate::CmdError::from(error)))
    })?;
    if let Some(recipe) = recipe {
        return regexp_native_selected::<O, E, Err>(
            ops,
            remaining,
            recipe,
            RegexpOptions {
                version,
                common: c,
                indices,
                inline,
                about,
            },
            |ops, index, value| assign(ops, &remaining[index + 2], value),
        );
    }
    let bytes = remaining[..if about { 1 } else { 2 }]
        .iter()
        .map(|value| {
            ops.native_string_bytes(value)
                .map(|bytes| bytes.to_vec())
                .map_err(|error| {
                    RegexpExecutionError::Regex(RegexError::from(crate::CmdError::from(error)))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rest = bytes.iter().map(Vec::as_slice).collect::<Vec<_>>();
    rest.resize(remaining.len(), &[]);
    regexp_selected_with_sink::<O, E, Err>(
        ops,
        &rest,
        RegexpOptions {
            version,
            common: c,
            indices,
            inline,
            about,
        },
        |ops, index, value| assign(ops, &remaining[index + 2], value),
    )
}

static JIM_REGEXP_NAMES: [&str; 10] = [
    "-indices",
    "-nocase",
    "-line",
    "-linestop",
    "-lineanchor",
    "-all",
    "-inline",
    "-start",
    "-expanded",
    "--",
];
static JIM_REGSUB_NAMES: [&str; 9] = [
    "-nocase",
    "-line",
    "-linestop",
    "-lineanchor",
    "-all",
    "-start",
    "-command",
    "-expanded",
    "--",
];
#[derive(Clone, Copy, PartialEq, Eq)]
enum JimCaptureOutput {
    Strings,
    Indices,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JimResultDelivery {
    Variables,
    Inline,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JimReplacement {
    Template,
    Command,
}

struct JimOptions {
    flags: RegexFlags,
    all: bool,
    indices: JimCaptureOutput,
    inline: JimResultDelivery,
    command: JimReplacement,
    start: i32,
    offset: usize,
}
fn jim_options<O: NativeRegexSource>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
    substitution: bool,
) -> Result<JimOptions, RegexError> {
    let names = if substitution {
        JIM_REGSUB_NAMES.as_slice()
    } else {
        JIM_REGEXP_NAMES.as_slice()
    };
    let mut selected = JimOptions {
        flags: RegexFlags::for_release(version),
        all: false,
        indices: JimCaptureOutput::Strings,
        inline: JimResultDelivery::Variables,
        command: JimReplacement::Template,
        start: 0,
        offset: 0,
    };
    while selected.offset < args.len() {
        let bytes = ops
            .native_string_bytes(&args[selected.offset])
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
        if bytes.first() != Some(&b'-') {
            break;
        }
        let index = ops
            .jim_regex_option(&args[selected.offset], names)
            .map_err(RegexError::from)?;
        selected.offset += 1;
        match names[index] {
            "-indices" => selected.indices = JimCaptureOutput::Indices,
            "-nocase" => selected.flags.nocase = true,
            "-line" => {
                selected.flags.lineanchor = true;
                selected.flags.linestop = true;
            }
            "-linestop" => selected.flags.linestop = true,
            "-lineanchor" => selected.flags.lineanchor = true,
            "-all" => selected.all = true,
            "-inline" => selected.inline = JimResultDelivery::Inline,
            "-command" => selected.command = JimReplacement::Command,
            "-expanded" => selected.flags.expanded = true,
            "-start" => {
                let original = args.get(selected.offset).ok_or_else(|| {
                    wrong_args(if substitution {
                        REGSUB_USAGE
                    } else {
                        REGEXP_USAGE
                    })
                })?;
                selected.start = ops.jim_regex_index(original).map_err(RegexError::from)?;
                selected.offset += 1;
            }
            _ => break,
        }
    }
    Ok(selected)
}
fn jim_byte_offset(bytes: &[u8], index: i32) -> Result<usize, ValueError> {
    let adjusted = if index < 0 {
        i64::from(index)
            + i64::try_from(bytes.len())
                .map_err(|_| ValueError::CommandProtocolUnavailable("Jim regexp source length"))?
            + 1
    } else {
        i64::from(index)
    };
    if adjusted <= 0 {
        return Ok(0);
    }
    let index = usize::try_from(adjusted)
        .map_err(|_| ValueError::CommandProtocolUnavailable("Jim regexp start width"))?;
    if index > bytes.len() {
        return Ok(bytes.len());
    }
    tcl_syntax::raw_string::RawString::from_bytes(bytes)
        .jim084_byte_offset(index)
        .map_err(Into::into)
}
fn jim_capture_value<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    subject: &[u8],
    span: RegMatch,
    offset: usize,
    character_offset: i64,
    output: JimCaptureOutput,
) -> Result<O::Value, ValueError> {
    if span.so != NO_MATCH && (span.eo < span.so || span.eo > subject.len() - offset) {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim regexp capture geometry",
        ));
    }
    if output == JimCaptureOutput::Indices {
        let (start, end) = if span.so == NO_MATCH {
            (-1, -1)
        } else {
            let start = jim_character_count(&subject[offset..offset + span.so])?;
            let end = jim_character_count(&subject[offset..offset + span.eo])?;
            (start + character_offset, end + character_offset - 1)
        };
        let start = ops.new_int(start);
        let end = ops.new_int(end);
        Ok(ops.new_list(vec![start, end]))
    } else if span.so == NO_MATCH {
        ops.regex_jim_range(b"", false)
    } else {
        ops.regex_jim_range(&subject[offset + span.so..offset + span.eo], true)
    }
}

fn jim_character_count(bytes: &[u8]) -> Result<i64, ValueError> {
    i64::try_from(
        tcl_syntax::raw_string::RawString::from_bytes(bytes)
            .jim084_characters()
            .count(),
    )
    .map_err(|_| ValueError::CommandProtocolUnavailable("Jim regexp index width"))
}

fn advance_jim_regexp(
    subject: &[u8],
    whole: RegMatch,
    offset: &mut usize,
    character_offset: &mut i64,
) -> Result<(), ValueError> {
    if whole.eo != 0 {
        *character_offset += jim_character_count(&subject[*offset..*offset + whole.eo])?;
        *offset += whole.eo;
    } else {
        *offset += 1;
        *character_offset += 1;
    }
    Ok(())
}

fn jim_whole_match(matches: &[RegMatch], length: usize) -> Result<RegMatch, ValueError> {
    let whole = matches
        .first()
        .copied()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "Jim regexp whole match geometry",
        ))?;
    if whole.so == NO_MATCH || whole.eo < whole.so || whole.eo > length {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim regexp byte match geometry",
        ));
    }
    Ok(whole)
}

fn regexp_jim_original<O: NativeRegexObjects<E>, E: RegexEngine, Err>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
    mut assign: impl FnMut(&mut O, &O::Value, O::Value) -> Result<(), Err>,
) -> Result<RegexpResult<O::Value>, RegexpExecutionError<Err>> {
    let host = |error| RegexpExecutionError::Regex(RegexError::from(crate::CmdError::from(error)));
    let selected = jim_options(ops, args, version, false).map_err(RegexpExecutionError::Regex)?;
    let original = &args[selected.offset..];
    if original.len() < 2 {
        return Err(RegexpExecutionError::Regex(wrong_args(REGEXP_USAGE)));
    }
    let prepared = prepare_pattern_original::<O, E>(ops, &original[0], selected.flags, version)
        .map_err(RegexpExecutionError::Regex)?;
    let PreparedOriginalRegex::Jim { artifact, .. } = prepared else {
        return Err(host(ValueError::CommandProtocolUnavailable(
            "Jim regexp prepared artifact",
        )));
    };
    let pattern = ops.native_string_bytes(&original[0]).map_err(host)?;
    let subject = ops.native_string_bytes(&original[1]).map_err(host)?;
    let vars = original.len() - 2;
    if selected.inline == JimResultDelivery::Inline && vars != 0 {
        return Err(RegexpExecutionError::Regex(RegexError::new(
            b"regexp match variables not allowed when using -inline".to_vec(),
        )));
    }
    let count = if selected.inline == JimResultDelivery::Inline {
        artifact.with_program(|re| E::nsub(re) + 1).map_err(host)?
    } else {
        vars
    };
    let mut offset = jim_byte_offset(&subject, selected.start).map_err(host)?;
    let mut character_offset = if selected.start < 0 {
        i64::from(selected.start)
            + i64::try_from(subject.len()).map_err(|_| {
                host(ValueError::CommandProtocolUnavailable(
                    "Jim regexp source length",
                ))
            })?
            + 1
    } else {
        i64::from(selected.start)
    };
    let mut notbol = selected.start != 0;
    let mut matches_count = 0i64;
    let mut output = Vec::new();
    loop {
        let matches = artifact
            .with_program(|re| E::exec_jim(re, &subject[offset..], count + 1, notbol))
            .map_err(host)?
            .map_err(|reason| host(ValueError::CommandProtocolUnavailable(reason)))?;
        let Some(matches) = matches else {
            break;
        };
        let whole = jim_whole_match(&matches, subject.len() - offset).map_err(host)?;
        matches_count += 1;
        for index in 0..count {
            let span = matches.get(index).copied().unwrap_or(RegMatch {
                so: NO_MATCH,
                eo: NO_MATCH,
            });
            let value = jim_capture_value::<O, E>(
                ops,
                &subject,
                span,
                offset,
                character_offset,
                selected.indices,
            )
            .map_err(host)?;
            if selected.inline == JimResultDelivery::Inline {
                output.push(value);
            } else {
                assign(ops, &original[index + 2], value)
                    .map_err(RegexpExecutionError::Assignment)?;
            }
        }
        if !selected.all
            || (pattern.first() == Some(&b'^') && !selected.flags.lineanchor)
            || subject.get(offset).is_none_or(|&byte| byte == 0)
        {
            break;
        }
        advance_jim_regexp(&subject, whole, &mut offset, &mut character_offset).map_err(host)?;
        if subject.get(offset).is_none_or(|&byte| byte == 0) {
            break;
        }
        notbol = true;
    }
    if selected.inline == JimResultDelivery::Inline {
        Ok(RegexpResult::Inline(ops.new_list(output)))
    } else {
        Ok(RegexpResult::Count {
            assign: None,
            count: matches_count,
        })
    }
}

pub fn regexp<O: ValueOps, E: RegexEngine>(
    ops: &mut O,
    args: &[&[u8]],
    version: TclVersion,
) -> Result<RegexpResult<O::Value>, RegexError> {
    let (c, indices, inline, about, i) =
        regexp_option_scan(&mut ByteOptionArguments(args), version)?;
    regexp_selected::<O, E>(ops, &args[i..], version, c, indices, inline, about)
}

fn regexp_selected<O: ValueOps, E: RegexEngine>(
    ops: &mut O,
    rest: &[&[u8]],
    version: TclVersion,
    c: Common,
    indices: bool,
    inline: bool,
    about: bool,
) -> Result<RegexpResult<O::Value>, RegexError> {
    let mut pairs = Vec::new();
    let outcome = regexp_selected_with_sink::<O, E, std::convert::Infallible>(
        ops,
        rest,
        RegexpOptions {
            version,
            common: c,
            indices,
            inline,
            about,
        },
        |_, index, value| {
            if index == 0 {
                pairs.clear();
            }
            pairs.push((rest[index + 2].to_vec(), value));
            Ok(())
        },
    );
    match outcome {
        Ok(RegexpResult::Count { count, .. }) => Ok(RegexpResult::Count {
            assign: (count != 0).then_some(pairs),
            count,
        }),
        Ok(result) => Ok(result),
        Err(RegexpExecutionError::Regex(error)) => Err(error),
        Err(RegexpExecutionError::Assignment(never)) => match never {},
    }
}

struct RegexpOptions {
    version: TclVersion,
    common: Common,
    indices: bool,
    inline: bool,
    about: bool,
}

fn regexp_selected_with_sink<O: ValueOps, E: RegexEngine, Err>(
    ops: &mut O,
    rest: &[&[u8]],
    options: RegexpOptions,
    mut assign: impl FnMut(&mut O, usize, O::Value) -> Result<(), Err>,
) -> Result<RegexpResult<O::Value>, RegexpExecutionError<Err>> {
    let RegexpOptions {
        version,
        common: c,
        indices,
        inline,
        about,
    } = options;
    // C: `(objc - i) < (2 - about)`. `-about` never looks at a subject, so the
    // pattern alone is enough — tclsh 8.4.20/8.5.19/8.6.18/9.0.4/9.1b0 all
    // answer `regexp -about {a(b)c}` with `1 {}` and give the same answer for
    // `regexp -about {(a)} extraarg`, while bare `regexp -about` is still a
    // wrong-# args.
    if rest.len() + usize::from(about) < 2 {
        return Err(RegexpExecutionError::Regex(wrong_args(REGEXP_USAGE)));
    }
    // C tests `-inline` against the *exact* remaining count and does it before
    // branching to `-about`, so `regexp -about -inline {(a)}` is the mix error
    // rather than an about answer (tclsh 8.4.20–9.1b0). Without `-about` the
    // arity check above has already forced `>= 2`, so `!= 2` is the old `> 2`.
    if inline && rest.len() != 2 {
        return Err(RegexpExecutionError::Regex(RegexError::new(
            b"regexp match variables not allowed when using -inline".to_vec(),
        )));
    }

    let pattern = rest[0];
    if about {
        // `TclRegAbout` (`tclRegexp.c`): a two-element list of the
        // subexpression count and the engine's `re_info` flag names. The
        // compile flags still apply — `regexp -about -expanded {a b}` is
        // `0 REG_UNONPOSIX` on every release — but nothing else about the
        // command runs: no subject decode, no `-start`, no match loop, and
        // `-indices`/`-all` are simply ignored (all tclsh-verified, 8.4.20
        // through 9.1b0).
        let re = E::compile(pattern, c.flags)
            .map_err(|d| RegexpExecutionError::Regex(compile_error(version, &d)))?;
        let nsubs = i64::try_from(E::nsub(&re)).unwrap_or(i64::MAX);
        let count = ops.new_int(nsubs);
        let flags: Vec<O::Value> = E::info_names(&re)
            .into_iter()
            .map(|name| ops.new_str(name))
            .collect();
        let info = ops.new_list(flags);
        // `Inline` is "the command result *is* this value, and no match
        // variable is written", which is exactly `-about`'s contract too — so
        // it carries the answer rather than the result enum gaining a variant
        // every adapter would have to learn.
        return Ok(RegexpResult::Inline(ops.new_list(vec![count, info])));
    }
    let str_bytes = rest[1];
    let (cps, byteoff) = decode_utf8(str_bytes);
    let char_len = cps.len();
    let match_vars = &rest[2..];

    let mut re = E::compile(pattern, c.flags)
        .map_err(|d| RegexpExecutionError::Regex(compile_error(version, &d)))?;
    let nsubs = E::nsub(&re);

    let mut offset = c
        .start
        .as_ref()
        .map_or(Ok(0), |spec| resolve_start_checked(ops, spec, char_len))
        .map_err(RegexpExecutionError::Regex)?;

    // Tcl's `all` doubles as flag + counter: starts 1 if `-all`, else 0.
    let mut all_count: i64 = i64::from(c.all);
    let mut inline_items: Vec<O::Value> = Vec::new();

    loop {
        let notbol = notbol_at(&cps, offset);
        let Some(matches) = E::exec(&mut re, &cps, offset, notbol) else {
            if all_count <= 1 {
                // No match at all (first time through).
                return Ok(if inline {
                    RegexpResult::Inline(ops.new_list(Vec::new()))
                } else {
                    RegexpResult::Count {
                        assign: None,
                        count: 0,
                    }
                });
            }
            break;
        };
        let m0 = matches[0];
        if inline {
            for k in 0..=nsubs {
                inline_items.push(build_match_item(
                    ops, &matches, k, nsubs, indices, str_bytes, &byteoff,
                ));
            }
        } else {
            for index in 0..match_vars.len() {
                let value =
                    build_match_item(ops, &matches, index, nsubs, indices, str_bytes, &byteoff);
                assign(ops, index, value).map_err(RegexpExecutionError::Assignment)?;
            }
        }

        if !c.all {
            break;
        }
        offset = m0.eo;
        if m0.eo == m0.so {
            offset += 1; // zero-length match: always advance to avoid looping
        }
        all_count += 1;
        if offset >= char_len {
            break;
        }
    }

    if inline {
        return Ok(RegexpResult::Inline(ops.new_list(inline_items)));
    }
    Ok(RegexpResult::Count {
        assign: None,
        count: if all_count > 0 { all_count - 1 } else { 1 },
    })
}

/// Slice the original bytes for the character range `[so, eo)` via the char→byte
/// table, **guarding every index**. `byteoff` has one entry per character plus a
/// final `bytes.len()`, so `so`/`eo` must be `<= char_len`; the FFI ARE engine
/// always honours that, but it is foreign code, so an out-of-range or inverted
/// `[so, eo)` must not index-panic here. Anything
/// off the table or backwards yields an empty slice rather than aborting.
fn slice_match<'a>(str_bytes: &'a [u8], byteoff: &[usize], so: usize, eo: usize) -> &'a [u8] {
    match (byteoff.get(so), byteoff.get(eo)) {
        (Some(&a), Some(&b)) if a <= b && b <= str_bytes.len() => &str_bytes[a..b],
        _ => &[],
    }
}

/// Build the value for match item `k`: an `{start end}` index pair (`-indices`)
/// or the matched substring (default). A non-participating group yields
/// `{-1 -1}` / the empty string, per `Tcl_RegexpObjCmd`.
// char offsets into a real subject are far below `i64::MAX`; the `i64` pair is
// Tcl's `-indices` result format.
#[allow(clippy::cast_possible_wrap)]
fn build_match_item<O: ValueOps>(
    ops: &mut O,
    matches: &[RegMatch],
    k: usize,
    nsubs: usize,
    indices: bool,
    str_bytes: &[u8],
    byteoff: &[usize],
) -> O::Value {
    let m = if k <= nsubs {
        matches.get(k).copied()
    } else {
        None
    };
    if indices {
        let (start, end): (i64, i64) = match m {
            Some(rm) if rm.so != NO_MATCH => (rm.so as i64, rm.eo as i64 - 1),
            _ => (-1, -1),
        };
        let a = ops.new_int(start);
        let b = ops.new_int(end);
        ops.new_list(vec![a, b])
    } else {
        match m {
            Some(rm) if rm.so != NO_MATCH && rm.eo > 0 => {
                ops.new_bytes(slice_match(str_bytes, byteoff, rm.so, rm.eo))
            }
            _ => ops.new_bytes(b""),
        }
    }
}

const REGSUB_USAGE: &[u8] = b"regsub ?-option ...? exp string subSpec ?varName?";

// C's `options[]` in `Tcl_RegsubObjCmd` (`tclCmdMZ.c`), matched with
// `TCL_EXACT` like `regexp`'s — and here even tclsh's bytecode compiler
// (`TclCompileRegsubCmd`) only fast-paths a literal `-all`, so `regsub` is
// exact-only in every context.
//
// Unlike `regexp`'s, this table *changed shape* at 9.0: TIP #463 inserted
// `-command` after `-all` and shifted `-nocase` into alphabetical order. The
// error noun moved too — `Tcl_GetIndexFromObj`'s `msg` argument is `"switch"`
// through 8.5 and `"option"` from 8.6 (`tclCmdMZ.c:576` / `:481` / `:521`) —
// and both are visible in the message a pre-9.0 `regsub -command` earns:
//
//   tclsh8.4.20 / 8.5.19: bad switch "-command": must be -all, -nocase,
//       -expanded, -line, -linestop, -lineanchor, -start, or --
//   tclsh8.6.18:          bad option "-command": must be -all, -nocase,
//       -expanded, -line, -linestop, -lineanchor, -start, or --
//   tclsh9.0.4 / 9.1b0:   regsub -command {a} abc {string toupper} -> Abc
static REGSUB_NAMES_8: [&str; 8] = [
    "-all",
    "-nocase",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-start",
    "--",
];
static REGSUB_NAMES_9: [&str; 9] = [
    "-all",
    "-command",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-nocase",
    "-start",
    "--",
];
static REGSUB_OPTIONS_8_4: OptionTable<'static> =
    OptionTable::exact_only("switch", &REGSUB_NAMES_8);
static REGSUB_OPTIONS_8_6: OptionTable<'static> =
    OptionTable::exact_only("option", &REGSUB_NAMES_8);
static REGSUB_OPTIONS_9_0: OptionTable<'static> =
    OptionTable::exact_only("option", &REGSUB_NAMES_9);

/// The `regsub` option table for `version`.
fn regsub_options(version: TclVersion) -> &'static OptionTable<'static> {
    if version >= TclVersion::V9_0 {
        &REGSUB_OPTIONS_9_0
    } else if version >= TclVersion::V8_6 {
        &REGSUB_OPTIONS_8_6
    } else {
        &REGSUB_OPTIONS_8_4
    }
}

/// A `regsub` failure once a `-command` prefix can be evaluated: either
/// `regsub`'s own diagnostic, or the callback's error passed through.
///
/// The evaluation error stays the adapter's own type (as [`crate::lsort`]'s
/// `sort_command` keeps its comparator's): a script failure carries a Tcl
/// return code, an `errorCode` and an `errorInfo` trailer — C appends
/// `\n    (-command substitution computation script)` to the latter — and none
/// of that is expressible as this module's ready-to-report message bytes.
pub enum RegsubError<Err> {
    /// An option, argument, pattern or command-prefix error from `regsub`.
    Regex(RegexError),
    /// The `-command` prefix's evaluation failed.
    Eval(Err),
}

impl<Err> From<RegexError> for RegsubError<Err> {
    fn from(e: RegexError) -> Self {
        RegsubError::Regex(e)
    }
}

/// Split a `-command` prefix into its words (C's `TclListObjGetElements` on
/// `objv[2]`), rejecting an empty one.
///
/// tclsh 9.0.4 / 9.1b0:
///   % regsub -command {a} abc {}
///   command prefix must be a list of at least one element
fn command_prefix(subspec: &[u8]) -> Result<Vec<Vec<u8>>, RegexError> {
    let text = core::str::from_utf8(subspec)
        .map_err(|_| RegexError::new(b"command prefix must be a valid list".to_vec()))?;
    let words = tcl_syntax::list::split_list(text)
        .map_err(|e| RegexError::new(e.message().as_bytes().to_vec()))?;
    if words.is_empty() {
        return Err(RegexError::new(
            b"command prefix must be a list of at least one element".to_vec(),
        ));
    }
    Ok(words
        .into_iter()
        .map(|w| w.into_owned().into_bytes())
        .collect())
}

/// Scan `regsub`'s leading options against `options`, returning the shared
/// compile/`-all`/`-start` state, whether `-command` was given, and the index
/// of the first non-option argument.
///
/// # Errors
/// A bad option, in `options`' own noun and enumeration.
fn regsub_option_scan(
    args: &mut impl OptionArguments,
    version: TclVersion,
) -> Result<(Common, bool, usize), RegexError> {
    let options = regsub_options(version);
    let mut c = Common::for_release(version);
    let mut command = false;
    let mut i = 0;
    while i < args.len() {
        let name = args.bytes(i)?;
        if name.first() != Some(&b'-') {
            break;
        }
        let idx = args.option(i, options)?;
        i += 1;
        // Matched by name, not by table index: the 8.x and 9.x tables list the
        // same options in different orders (see the tables above), so an index
        // means nothing without knowing which one answered.
        match options.names()[idx] {
            "-all" => c.all = true,
            "-command" => command = true,
            "-expanded" => c.flags.expanded = true,
            "-line" => {
                c.flags.linestop = true;
                c.flags.lineanchor = true;
            }
            "-linestop" => c.flags.linestop = true,
            "-lineanchor" => c.flags.lineanchor = true,
            "-nocase" => c.flags.nocase = true,
            "-start" => match (i < args.len()).then(|| args.bytes(i)).transpose()? {
                Some(v) => {
                    c.start = Some(v);
                    i += 1;
                }
                // A trailing `-start` ends the options (C's `goto
                // endOfForLoop`); the caller's arity check then reports it.
                None => break,
            },
            // `--`: explicit end of options.
            _ => break,
        }
    }
    Ok((c, command, i))
}

/// Original-object option preparation retained through callback evaluation.
/// This packet owns no replacement option values and cannot grant Index state.
pub struct OriginalRegsubPreparation<'a, V> {
    version: TclVersion,
    common: Common,
    command: bool,
    rest: Vec<Vec<u8>>,
    target: Option<&'a V>,
    originals: &'a [V],
    start: usize,
    recipe: Option<tcl_syntax::native_regex::NativeRegexpRecipe>,
}

/// Scan actual original options once before borrowing the adapter for callbacks.
pub fn regsub_prepare_original<'a, O: NativeRegexSource>(
    ops: &mut O,
    args: &'a [O::Value],
    version: TclVersion,
) -> Result<OriginalRegsubPreparation<'a, O::Value>, RegexError> {
    let (common, command, offset) =
        regsub_option_scan(&mut OriginalOptionArguments { ops, args }, version)?;
    let originals = &args[offset..];
    if originals.len() < 3 || originals.len() > 4 {
        return Err(wrong_args(REGSUB_USAGE));
    }
    let recipe = ops
        .regex_recipe()
        .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
    let command_start = if recipe.is_some() {
        common
            .start
            .as_ref()
            .map(|spec| {
                let length = ops
                    .native_char_len(&originals[1])
                    .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
                resolve_start_checked(ops, spec, length)
            })
            .transpose()?
    } else {
        None
    };
    let rest = originals[..if recipe.is_some() { 0 } else { 3 }]
        .iter()
        .map(|value| {
            ops.native_string_bytes(value)
                .map(|bytes| bytes.to_vec())
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let start = if recipe.is_some() {
        command_start.unwrap_or(0)
    } else {
        let (characters, _) = decode_utf8(&rest[1]);
        common.start.as_ref().map_or(Ok(0), |spec| {
            resolve_start_checked(ops, spec, characters.len())
        })?
    };
    Ok(OriginalRegsubPreparation {
        version,
        common,
        command,
        rest,
        target: originals.get(3),
        originals,
        start,
        recipe,
    })
}

/// Result text and a borrowed same-original output target. The argv owner
/// keeps the target alive; this transport adds no native object reference.
pub struct OriginalRegsubResult<'a, V> {
    pub text: Vec<u8>,
    pub count: i64,
    pub target: Option<&'a V>,
}

/// Concrete physical edges for C9 command-prefix substitution. Object
/// transports retain lifetime separately from actual native references.
pub trait NativeRegsubObjects: ValueOps {
    type Object;
    type Error;
    /// Construct an actual Jim byte result or fresh callback argument.
    fn regex_jim_bytes(
        &mut self,
        _bytes: &[u8],
        _string_primary: bool,
    ) -> Result<Self::Object, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "original Jim regsub String producer",
        ))
    }
    /// Borrow the current original callback result and reach its Jim getter.
    fn regex_result_bytes(&mut self) -> Result<std::rc::Rc<[u8]>, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "original Jim regsub callback result",
        ))
    }
    fn regex_object<'a>(&self, value: &'a Self::Object) -> &'a Self::Value;
    fn regex_borrow(&self, value: &Self::Value) -> Self::Object;
    fn regex_duplicate(
        &mut self,
        value: &Self::Value,
    ) -> Result<Self::Object, tcl_syntax::value::ValueError>;
    fn regex_members(
        &mut self,
        value: &Self::Value,
    ) -> Result<Vec<Self::Object>, tcl_syntax::value::ValueError>;
    fn regex_unicode(
        &mut self,
        units: &[u32],
    ) -> Result<Self::Object, tcl_syntax::value::ValueError>;
    fn regex_append_unicode(
        &mut self,
        result: &Self::Object,
        units: &[u32],
    ) -> Result<(), tcl_syntax::value::ValueError>;
    fn regex_append_current_result(
        &mut self,
        result: &mut Self::Object,
    ) -> Result<(), tcl_syntax::value::ValueError>;
    fn regex_eval(
        &mut self,
        prefix: &Self::Value,
        arguments: &[Self::Object],
    ) -> Result<(), Self::Error>;
    fn regex_reset_result(&mut self) -> Result<(), tcl_syntax::value::ValueError>;
}

/// Original C cache, Unicode range and actual engine artifact adapters.
/// A missing recipe is only the explicitly selected compatibility path.
pub trait NativeRegexSource: ValueOps {
    /// A distinct actual Jim issuer; compatibility adapters provide none.
    fn jim_regex_recipe(
        &self,
    ) -> Result<Option<tcl_syntax::native_regex::JimRegexpRecipe>, ValueError> {
        Ok(None)
    }
    fn jim_regex_option(
        &mut self,
        _original: &Self::Value,
        _table: &'static [&'static str],
    ) -> Result<usize, crate::CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original Jim regexp Enum").into())
    }
    fn jim_regex_index(&mut self, _original: &Self::Value) -> Result<i32, crate::CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original Jim regexp index").into())
    }

    fn regex_recipe(
        &self,
    ) -> Result<Option<tcl_syntax::native_regex::NativeRegexpRecipe>, tcl_syntax::value::ValueError>;
}
pub trait NativeRegexObjects<E: RegexEngine>: NativeRegexSource {
    fn regex_jim_range(&mut self, bytes: &[u8], matched: bool) -> Result<Self::Value, ValueError> {
        let _ = (bytes, matched);
        Err(ValueError::CommandProtocolUnavailable(
            "original Jim regexp range String",
        ))
    }
    fn regex_cached_jim_pattern(
        &self,
        _original: &Self::Value,
        _flags: u32,
    ) -> Result<Option<tcl_syntax::native_regex::JimRegexpArtifact<E::Regex>>, ValueError> {
        Ok(None)
    }
    fn regex_install_jim_pattern(
        &mut self,
        _original: &Self::Value,
        _flags: u32,
        _program: E::Regex,
    ) -> Result<tcl_syntax::native_regex::JimRegexpArtifact<E::Regex>, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "original Jim regexp primary publication",
        ))
    }

    fn regex_cached_pattern(
        &self,
        original: &Self::Value,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
    ) -> Result<Option<std::rc::Rc<std::cell::RefCell<E::Regex>>>, tcl_syntax::value::ValueError>;
    fn regex_cached_glob(
        &self,
        original: &Self::Value,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
    ) -> Result<Option<std::rc::Rc<[u8]>>, tcl_syntax::value::ValueError>;
    fn regex_install_pattern(
        &mut self,
        original: &Self::Value,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
        compiled: std::rc::Rc<std::cell::RefCell<E::Regex>>,
    ) -> Result<(), tcl_syntax::value::ValueError>;
    fn regex_range_value(
        &mut self,
        range: tcl_syntax::native_regex::NativeRegexpRange,
    ) -> Result<Self::Value, tcl_syntax::value::ValueError>;
}

fn native_compiled_pattern<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    original: &O::Value,
    recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
    flags: RegexFlags,
    version: TclVersion,
) -> Result<std::rc::Rc<std::cell::RefCell<E::Regex>>, RegexError> {
    let host = |error| RegexError::from(crate::CmdError::from(error));
    if let Some(compiled) = ops
        .regex_cached_pattern(original, recipe, flags.cache_key())
        .map_err(host)?
    {
        return Ok(compiled);
    }
    let bytes = ops.native_string_bytes(original).map_err(host)?;
    let units = recipe.pattern_units(&bytes);
    let compiled = E::compile_units(&units, flags)
        .ok_or_else(|| {
            host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regexp character-unit compiler",
            ))
        })?
        .map_err(|detail| compile_error(version, &detail))?;
    let compiled = std::rc::Rc::new(std::cell::RefCell::new(compiled));
    ops.regex_install_pattern(
        original,
        recipe,
        flags.cache_key(),
        std::rc::Rc::clone(&compiled),
    )
    .map_err(host)?;
    Ok(compiled)
}

/// Retained original-pattern engine artifact. A compatibility artifact owns
/// no native `RegExp` primary or object cache authority.
pub enum PreparedOriginalRegex<R> {
    Native {
        compiled: std::rc::Rc<std::cell::RefCell<R>>,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        glob: Option<std::rc::Rc<[u8]>>,
        flags: RegexFlags,
    },
    Jim {
        artifact: tcl_syntax::native_regex::JimRegexpArtifact<R>,
        flags: RegexFlags,
    },
    Compatibility(R),
}
impl<R> PreparedOriginalRegex<R> {
    #[must_use]
    pub const fn native_recipe(&self) -> Option<tcl_syntax::native_regex::NativeRegexpRecipe> {
        match self {
            Self::Native { recipe, .. } => Some(*recipe),
            Self::Jim { .. } | Self::Compatibility(_) => None,
        }
    }
}

/// Compile an original pattern without a compatibility byte projection on C.
pub fn prepare_pattern_original<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &O::Value,
    flags: RegexFlags,
    version: TclVersion,
) -> Result<PreparedOriginalRegex<E::Regex>, RegexError> {
    if let Some(recipe) = ops
        .regex_recipe()
        .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
    {
        let compiled = native_compiled_pattern::<O, E>(ops, pattern, recipe, flags, version)?;
        let glob = ops
            .regex_cached_glob(pattern, recipe, flags.cache_key())
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
        Ok(PreparedOriginalRegex::Native {
            compiled,
            recipe,
            glob,
            flags,
        })
    } else if ops
        .jim_regex_recipe()
        .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
        .is_some()
    {
        let key = jim_flags(flags);
        let artifact = if let Some(artifact) = ops
            .regex_cached_jim_pattern(pattern, key)
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
        {
            artifact
        } else {
            let bytes = ops
                .native_string_bytes(pattern)
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
            let program = E::compile_jim(&bytes, flags)
                .ok_or_else(|| {
                    RegexError::from(crate::CmdError::from(
                        ValueError::CommandProtocolUnavailable("bundled Jim regexp compiler"),
                    ))
                })?
                .map_err(|detail| {
                    RegexError::new(
                        [
                            b"couldn't compile regular expression pattern: ".as_slice(),
                            &detail,
                        ]
                        .concat(),
                    )
                })?;
            ops.regex_install_jim_pattern(pattern, key, program)
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
        };
        // Public Jim regexp consumers deliberately reach this getter after
        // cache selection. A missing native updater is a typed host refusal.
        ops.native_string_bytes(pattern)
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
        Ok(PreparedOriginalRegex::Jim { artifact, flags })
    } else {
        let bytes = ops
            .native_string_bytes(pattern)
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
        E::compile(&bytes, flags)
            .map(PreparedOriginalRegex::Compatibility)
            .map_err(|detail| compile_error(version, &detail))
    }
}

fn jim_flags(flags: RegexFlags) -> u32 {
    u32::from(flags.nocase) * 2
        + u32::from(flags.lineanchor) * 4
        + u32::from(flags.linestop) * 8
        + u32::from(flags.expanded) * 32
}

/// Native LSEARCH tries NOSUB before list conversion and retries genuine
/// compilation failure without NOSUB. Host access refusals never retry.
pub fn prepare_search_pattern_original<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &O::Value,
    mut flags: RegexFlags,
    version: TclVersion,
) -> Result<PreparedOriginalRegex<E::Regex>, RegexError> {
    if ops
        .regex_recipe()
        .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
        .is_none()
    {
        return prepare_pattern_original::<O, E>(ops, pattern, flags, version);
    }
    flags.nosub = true;
    match prepare_pattern_original::<O, E>(ops, pattern, flags, version) {
        Ok(compiled) => Ok(compiled),
        Err(error) => {
            if error.0.native_access_refusal().is_some() || error.0.unicode_refusal().is_some() {
                return Err(error);
            }
            flags.nosub = false;
            prepare_pattern_original::<O, E>(ops, pattern, flags, version)
        }
    }
}

/// Execute against the original subject using the retained artifact's input
/// owner. The executable borrow is released before any variable callback.
pub fn execute_pattern_original<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &mut PreparedOriginalRegex<E::Regex>,
    subject: &O::Value,
    offset: usize,
    notbol: bool,
) -> Result<Option<Vec<RegMatch>>, RegexError> {
    execute_pattern_original_mode::<O, E>(ops, pattern, subject, offset, notbol, true)
}

/// Execute a native boolean match without requesting capture ranges.
pub fn match_pattern_original<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &mut PreparedOriginalRegex<E::Regex>,
    subject: &O::Value,
) -> Result<bool, RegexError> {
    execute_pattern_original_mode::<O, E>(ops, pattern, subject, 0, false, false)
        .map(|result| result.is_some())
}

fn execute_pattern_original_mode<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &mut PreparedOriginalRegex<E::Regex>,
    subject: &O::Value,
    offset: usize,
    notbol: bool,
    captures: bool,
) -> Result<Option<Vec<RegMatch>>, RegexError> {
    match pattern {
        PreparedOriginalRegex::Native {
            compiled,
            recipe,
            glob,
            flags,
        } => {
            if (flags.nosub || !captures)
                && !flags.expanded
                && !flags.linestop
                && !flags.lineanchor
                && offset == 0
                && !notbol
                && let Some(glob) = glob
            {
                return native_glob_exec(ops, *recipe, glob, subject, flags.nocase)
                    .map(|matched| matched.then(Vec::new));
            }
            let units = ops
                .native_unicode_units(subject)
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
            let characters = units
                .iter()
                .map(|&unit| {
                    i32::try_from(unit).map_err(|_| {
                        RegexError::from(crate::CmdError::from(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "native regexp character width",
                            ),
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(E::exec(
                &mut compiled.borrow_mut(),
                &characters,
                offset,
                notbol,
            ))
        }
        PreparedOriginalRegex::Jim { artifact, .. } => {
            let bytes = ops
                .native_string_bytes(subject)
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
            let raw = tcl_syntax::raw_string::RawString::from_bytes(bytes.clone());
            let byte_offset = raw.jim084_byte_offset(offset).map_err(|error| {
                RegexError::from(crate::CmdError::from(ValueError::from(error)))
            })?;
            let count = if captures {
                artifact
                    .with_program(|compiled| E::nsub(compiled) + 1)
                    .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
            } else {
                1
            };
            artifact
                .with_program(|compiled| {
                    E::exec_jim(compiled, &bytes[byte_offset..], count, notbol)
                })
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
                .map(|matches| {
                    matches.map(|mut matches| {
                        for span in &mut matches {
                            if span.so != NO_MATCH {
                                span.so += byte_offset;
                            }
                            if span.eo != NO_MATCH {
                                span.eo += byte_offset;
                            }
                        }
                        matches
                    })
                })
                .map_err(|reason| {
                    RegexError::from(crate::CmdError::from(
                        ValueError::CommandProtocolUnavailable(reason),
                    ))
                })
        }
        PreparedOriginalRegex::Compatibility(compiled) => {
            let bytes = ops
                .native_string_bytes(subject)
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
            let (characters, _) = decode_utf8(&bytes);
            Ok(E::exec(compiled, &characters, offset, notbol))
        }
    }
}

fn native_glob_exec<O: ValueOps>(
    ops: &mut O,
    recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
    pattern: &[u8],
    subject: &O::Value,
    nocase: bool,
) -> Result<bool, RegexError> {
    use tcl_syntax::native_glob::NativeGlobObject as Glob;
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    let host = |error| RegexError::from(crate::CmdError::from(error));
    let snapshot = ops.native_object_snapshot(subject).map_err(host)?;
    let unicode = matches!(&snapshot.cache, Cache::String { .. })
        || (recipe.version() >= TclVersion::V8_6 && matches!(&snapshot.cache, Cache::None));
    let units = unicode
        .then(|| ops.native_unicode_units(subject))
        .transpose()
        .map_err(host)?;
    let binary = recipe.version() == TclVersion::V8_5
        && !nocase
        && snapshot.resident.is_none()
        && matches!(&snapshot.cache, Cache::ByteArray { .. });
    let bytes = if units.is_none() && !binary {
        Some(ops.native_string_bytes(subject).map_err(host)?)
    } else {
        snapshot.resident.clone()
    };
    let original = if let Some(units) = &units {
        Glob::CachedUnicode {
            units,
            resident_bytes: snapshot.resident.as_deref(),
        }
    } else if let Cache::ByteArray { bytes: binary, .. } = &snapshot.cache {
        match &bytes {
            Some(resident) => Glob::ByteArrayWithString {
                bytes: binary,
                resident_bytes: resident,
            },
            None => Glob::PureByteArray(binary),
        }
    } else {
        match snapshot.cache {
            Cache::None => Glob::FreshString(bytes.as_deref().expect("reached string")),
            _ => Glob::OtherString(bytes.as_deref().expect("reached string")),
        }
    };
    tcl_syntax::native_glob::match_native_glob_objects(
        tcl_syntax::naming::NativeNameProtocol::C(recipe.version()),
        Glob::FreshString(pattern),
        original,
        nocase,
    )
    .map_err(|_| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native regexp equivalent-glob storage",
        ))
    })
}

/// Native REGEXP instruction over the same original pattern and subject.
/// It has no option argv and acquires no replacement object references.
pub fn compiled_match_original<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &O::Value,
    subject: &O::Value,
    flags: RegexFlags,
    version: TclVersion,
) -> Result<bool, RegexError> {
    let host = |error| RegexError::from(crate::CmdError::from(error));
    let recipe = ops.regex_recipe().map_err(host)?.ok_or_else(|| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native REGEXP instruction issuer",
        ))
    })?;
    let mut compiled = prepare_pattern_original::<O, E>(ops, pattern, flags, version)?;
    // The selected instruction requests zero ranges, independently of NOSUB.
    // The recipe query above authenticates the physical input owner.
    debug_assert_eq!(compiled.native_recipe(), Some(recipe));
    match_pattern_original::<O, E>(ops, &mut compiled, subject)
}

fn native_capture_value<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    subject: &O::Value,
    units: &[u32],
    span: Option<RegMatch>,
    recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
    indices: bool,
) -> Result<O::Value, ValueError> {
    if indices {
        let (start, end) = match span {
            Some(span) if span.so != NO_MATCH => (
                i64::try_from(span.so).unwrap_or(i64::MAX),
                i64::try_from(span.eo).unwrap_or(i64::MAX) - 1,
            ),
            _ => (-1, -1),
        };
        let start = ops.new_int(start);
        let end = ops.new_int(end);
        Ok(ops.new_list(vec![start, end]))
    } else {
        let range = match span {
            Some(span) if span.so != NO_MATCH && span.eo > 0 => recipe.range(
                &ops.native_object_snapshot(subject)?,
                units,
                span.so,
                span.eo,
            )?,
            _ => tcl_syntax::native_regex::NativeRegexpRange::Empty,
        };
        ops.regex_range_value(range)
    }
}

fn native_regex_characters(units: &[u32], reason: &'static str) -> Result<Vec<i32>, ValueError> {
    units
        .iter()
        .map(|&unit| {
            i32::try_from(unit).map_err(|_| ValueError::CommandProtocolUnavailable(reason))
        })
        .collect()
}

fn native_whole_match(
    matches: &[RegMatch],
    minimum: usize,
    length: usize,
    reason: &'static str,
) -> Result<RegMatch, ValueError> {
    let whole = matches
        .first()
        .copied()
        .ok_or(ValueError::CommandProtocolUnavailable(reason))?;
    if whole.so < minimum || whole.so > whole.eo || whole.eo > length {
        return Err(ValueError::CommandProtocolUnavailable(reason));
    }
    Ok(whole)
}

fn regexp_native_selected<O: NativeRegexObjects<E>, E: RegexEngine, Err>(
    ops: &mut O,
    originals: &[O::Value],
    recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
    options: RegexpOptions,
    mut assign: impl FnMut(&mut O, usize, O::Value) -> Result<(), Err>,
) -> Result<RegexpResult<O::Value>, RegexpExecutionError<Err>> {
    let host = |error| RegexpExecutionError::Regex(RegexError::from(crate::CmdError::from(error)));
    let RegexpOptions {
        version,
        common,
        indices,
        inline,
        about,
    } = options;
    let (length, mut offset) = if about {
        (0, 0)
    } else {
        let length = ops.native_char_len(&originals[1]).map_err(host)?;
        let offset = common
            .start
            .as_ref()
            .map_or(Ok(0), |spec| resolve_start_checked(ops, spec, length))
            .map_err(RegexpExecutionError::Regex)?;
        (length, offset)
    };
    let compiled =
        native_compiled_pattern::<O, E>(ops, &originals[0], recipe, common.flags, version)
            .map_err(RegexpExecutionError::Regex)?;
    let nsubs = E::nsub(&compiled.borrow());
    if about {
        let count = ops.new_int(i64::try_from(nsubs).unwrap_or(i64::MAX));
        let names = E::info_names(&compiled.borrow())
            .into_iter()
            .map(|name| ops.new_str(name))
            .collect();
        let names = ops.new_list(names);
        return Ok(RegexpResult::Inline(ops.new_list(vec![count, names])));
    }
    let mut count = 0_i64;
    let mut items = Vec::new();
    loop {
        let units = ops.native_unicode_units(&originals[1]).map_err(host)?;
        let characters =
            native_regex_characters(&units, "native regexp character width").map_err(host)?;
        let matches = E::exec(
            &mut compiled.borrow_mut(),
            &characters,
            offset,
            notbol_at(&characters, offset),
        );
        let Some(matches) = matches else {
            break;
        };
        let whole = native_whole_match(
            &matches,
            0,
            units.len(),
            "native regexp whole-match geometry",
        )
        .map_err(host)?;
        let total = if inline {
            nsubs + 1
        } else {
            originals.len() - 2
        };
        for index in 0..total {
            let span = (index <= nsubs)
                .then(|| matches.get(index).copied())
                .flatten();
            let value =
                native_capture_value::<O, E>(ops, &originals[1], &units, span, recipe, indices)
                    .map_err(host)?;
            if inline {
                items.push(value);
            } else {
                assign(ops, index, value).map_err(RegexpExecutionError::Assignment)?;
            }
        }
        count += 1;
        if !common.all {
            break;
        }
        offset = whole.eo;
        if whole.eo == whole.so {
            offset += 1;
        }
        if offset >= length {
            break;
        }
    }
    if inline {
        Ok(RegexpResult::Inline(ops.new_list(items)))
    } else {
        Ok(RegexpResult::Count {
            assign: None,
            count,
        })
    }
}

/// A real command callback completion is transported separately from parser errors.
pub enum OriginalRegexConsumerError<E, D = crate::CmdError> {
    Command(D),
    Callback(E),
}

fn jim_substitution_bytes(
    output: &mut Vec<u8>,
    replacement: &[u8],
    subject: &[u8],
    offset: usize,
    matched: &[RegMatch],
) {
    let mut cursor = 0;
    while cursor < replacement.len() {
        let byte = replacement[cursor];
        cursor += 1;
        let capture = if byte == b'&' {
            Some(0)
        } else if byte == b'\\' && replacement.get(cursor).is_some_and(u8::is_ascii_digit) {
            let group = usize::from(replacement[cursor] - b'0');
            cursor += 1;
            Some(group)
        } else {
            None
        };
        if let Some(group) = capture {
            if let Some(span) = matched.get(group).filter(|span| span.so != NO_MATCH) {
                output.extend_from_slice(&subject[offset + span.so..offset + span.eo]);
            }
        } else if byte == b'\\'
            && replacement
                .get(cursor)
                .is_some_and(|&byte| matches!(byte, b'&' | b'\\'))
        {
            output.push(replacement[cursor]);
            cursor += 1;
        } else {
            output.push(byte);
        }
    }
}

fn jim_regsub_callback<O: NativeRegsubObjects>(
    ops: &mut O,
    prefix: &O::Object,
    subject: &[u8],
    offset: usize,
    matched: &[RegMatch],
    output: &mut Vec<u8>,
) -> Result<(), RegsubError<O::Error>> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let mut arguments = Vec::new();
    for span in matched {
        if span.so == NO_MATCH {
            break;
        }
        arguments.push(
            ops.regex_jim_bytes(&subject[offset + span.so..offset + span.eo], false)
                .map_err(host)?,
        );
    }
    let call = ops
        .regex_duplicate(ops.regex_object(prefix))
        .map_err(host)?;
    ops.regex_eval(ops.regex_object(&call), &arguments)
        .map_err(RegsubError::Eval)?;
    let current = ops.regex_result_bytes().map_err(host)?;
    output.extend_from_slice(tcl_core_types::c_string_extent(&current));
    Ok(())
}

/// Bundled Jim regsub uses a shallow original-pattern duplicate, `CString`
/// matching, counted replacement bytes and original callback result getters.
pub fn regsub_jim_original<'a, O: NativeRegsubObjects + NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    args: &'a [O::Value],
    version: TclVersion,
) -> NativeRegsubOutcome<'a, O> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let selected = jim_options(ops, args, version, true).map_err(RegsubError::Regex)?;
    let originals = &args[selected.offset..];
    if !(3..=4).contains(&originals.len()) {
        return Err(RegsubError::Regex(wrong_args(REGSUB_USAGE)));
    }
    let duplicate = ops.regex_duplicate(&originals[0]).map_err(host)?;
    let original_duplicate = ops.regex_object(&duplicate);
    let prepared =
        prepare_pattern_original::<O, E>(ops, original_duplicate, selected.flags, version)
            .map_err(RegsubError::Regex)?;
    let PreparedOriginalRegex::Jim { artifact, .. } = prepared else {
        return Err(host(ValueError::CommandProtocolUnavailable(
            "Jim regsub prepared artifact",
        )));
    };
    let pattern = ops.native_string_bytes(&originals[0]).map_err(host)?;
    let subject = ops.native_string_bytes(&originals[1]).map_err(host)?;
    let prefix = if selected.command == JimReplacement::Command {
        let members = ops.regex_members(&originals[2]).map_err(host)?;
        if members.is_empty() {
            return Err(RegsubError::Regex(RegexError::new(
                b"command prefix must be a list of at least one element".to_vec(),
            )));
        }
        Some(ops.regex_borrow(&originals[2]))
    } else {
        None
    };
    let replacement = if selected.command == JimReplacement::Command {
        None
    } else {
        Some(ops.native_string_bytes(&originals[2]).map_err(host)?)
    };
    let mut offset = jim_byte_offset(&subject, selected.start).map_err(host)?;
    let mut output = subject[..offset].to_vec();
    let mut count = 0i64;
    let mut notbol = false;
    while offset < subject.len() || !tcl_core_types::c_string_extent(&pattern).is_empty() {
        let matched = artifact
            .with_program(|program| E::exec_jim(program, &subject[offset..], 50, notbol))
            .map_err(host)?
            .map_err(|reason| host(ValueError::CommandProtocolUnavailable(reason)))?;
        let Some(matched) = matched else {
            break;
        };
        let whole = *matched.first().ok_or_else(|| {
            host(ValueError::CommandProtocolUnavailable(
                "Jim regsub whole match",
            ))
        })?;
        for span in &matched {
            if span.so != NO_MATCH && (span.eo < span.so || span.eo > subject.len() - offset) {
                return Err(host(ValueError::CommandProtocolUnavailable(
                    "Jim regsub original range geometry",
                )));
            }
        }
        if whole.so == NO_MATCH {
            return Err(host(ValueError::CommandProtocolUnavailable(
                "Jim regsub missing whole match",
            )));
        }
        output.extend_from_slice(&subject[offset..offset + whole.so]);
        if let Some(prefix) = &prefix {
            jim_regsub_callback(ops, prefix, &subject, offset, &matched, &mut output)?;
        } else if let Some(replacement) = &replacement {
            jim_substitution_bytes(&mut output, replacement, &subject, offset, &matched);
        }
        count += 1;
        offset += whole.eo;
        notbol = false;
        if !selected.all || offset == subject.len() {
            break;
        }
        if whole.eo == whole.so {
            if pattern.first() == Some(&b'^') {
                notbol = true;
            } else {
                let width = tcl_syntax::raw_string::RawString::from_bytes(&subject[offset..])
                    .jim084_byte_offset(1)
                    .map_err(ValueError::from)
                    .map_err(host)?;
                output.extend_from_slice(&subject[offset..offset + width]);
                offset += width;
            }
        }
    }
    output.extend_from_slice(tcl_core_types::c_string_extent(&subject[offset..]));
    let result = ops.regex_jim_bytes(&output, true).map_err(host)?;
    Ok(NativeRegsubResult {
        result,
        count,
        target: originals.get(3),
    })
}

/// Native result retains the genuine object producer and the same output target.
pub struct NativeRegsubResult<'a, V, T> {
    pub result: T,
    pub count: i64,
    pub target: Option<&'a V>,
}

/// Original substitution result and guest callback completion for one adapter.
pub type NativeRegsubOutcome<'a, O> = Result<
    NativeRegsubResult<'a, <O as ValueOps>::Value, <O as NativeRegsubObjects>::Object>,
    RegsubError<<O as NativeRegsubObjects>::Error>,
>;

impl<V> OriginalRegsubPreparation<'_, V> {
    /// Whether the actual selected option table entered command-prefix mode.
    #[must_use]
    pub fn is_command(&self) -> bool {
        self.command
    }
    #[must_use]
    pub fn is_native(&self) -> bool {
        self.recipe.is_some()
    }
}

type RegsubAccumulation<T, E> = Result<(Option<T>, i64), RegsubError<E>>;

fn native_literal_substitution<O: NativeRegsubObjects + NativeRegexSource>(
    ops: &mut O,
    prepared: &OriginalRegsubPreparation<'_, O::Value>,
    recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
) -> RegsubAccumulation<O::Object, O::Error> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let original = prepared.originals;
    let mut count = 0;
    let mut result = None;
    let pattern = ops.native_unicode_units(&original[0]).map_err(host)?;
    let subject = ops.native_unicode_units(&original[1]).map_err(host)?;
    let substitution = ops.native_unicode_units(&original[2]).map_err(host)?;
    let mut output = Vec::new();
    if pattern.is_empty() {
        for &unit in subject.iter() {
            output.extend_from_slice(&substitution);
            output.push(unit);
            count += 1;
        }
    } else {
        let mut cursor = 0;
        let mut copied = 0;
        while cursor <= subject.len().saturating_sub(pattern.len())
            && pattern.len() <= subject.len()
        {
            if recipe.equal_units(
                &subject[cursor..cursor + pattern.len()],
                &pattern,
                prepared.common.flags.nocase,
            ) {
                output.extend_from_slice(&subject[copied..cursor]);
                output.extend_from_slice(&substitution);
                cursor += pattern.len();
                copied = cursor;
                count += 1;
            } else {
                cursor += 1;
            }
        }
        if count != 0 {
            output.extend_from_slice(&subject[copied..]);
        }
    }
    if count != 0 {
        let accumulator = ops.regex_unicode(&[]).map_err(host)?;
        ops.regex_append_unicode(&accumulator, &output)
            .map_err(host)?;
        result = Some(accumulator);
    }
    Ok((result, count))
}

fn native_regsub_matches<O: NativeRegsubObjects, E: RegexEngine>(
    ops: &mut O,
    prepared: &OriginalRegsubPreparation<'_, O::Value>,
    compiled: &std::rc::Rc<std::cell::RefCell<E::Regex>>,
    units: &[u32],
    replacement: &[u32],
    characters: &[i32],
) -> RegsubAccumulation<O::Object, O::Error> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let mut count = 0;
    let mut result = None;
    let mut offset = prepared.start;
    while offset <= units.len() {
        let matches = E::exec(
            &mut compiled.borrow_mut(),
            characters,
            offset,
            notbol_at(characters, offset),
        );
        let Some(matches) = matches else {
            break;
        };
        let whole = matches.first().copied().ok_or_else(|| {
            host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regsub match geometry",
            ))
        })?;
        if whole.so < offset || whole.so > whole.eo || whole.eo > units.len() {
            return Err(host(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native regsub match geometry",
                ),
            ));
        }
        if result.is_none() {
            let accumulator = ops.regex_unicode(&[]).map_err(host)?;
            ops.regex_append_unicode(&accumulator, &units[..offset])
                .map_err(host)?;
            result = Some(accumulator);
        }
        let accumulator = result.as_ref().expect("matched native result");
        ops.regex_append_unicode(accumulator, &units[offset..whole.so])
            .map_err(host)?;
        let expanded = native_substitution_units(replacement, units, &matches).map_err(host)?;
        ops.regex_append_unicode(accumulator, &expanded)
            .map_err(host)?;
        count += 1;
        offset = whole.eo;
        if whole.eo == whole.so {
            if offset < units.len() {
                ops.regex_append_unicode(accumulator, &units[offset..=offset])
                    .map_err(host)?;
            }
            offset += 1;
        }
        if !prepared.common.all {
            break;
        }
    }
    if let Some(result) = &result
        && offset < units.len()
    {
        ops.regex_append_unicode(result, &units[offset..])
            .map_err(host)?;
    }
    Ok((result, count))
}

/// Original C regsub instruction and command result over native Unicode.
/// The literal mapping route preserves its distinct original String primaries.
pub fn regsub_native_original<
    'a,
    O: NativeRegsubObjects + NativeRegexObjects<E>,
    E: RegexEngine,
>(
    ops: &mut O,
    prepared: &OriginalRegsubPreparation<'a, O::Value>,
) -> NativeRegsubOutcome<'a, O> {
    if prepared.command {
        return regsub_command_original::<O, E>(ops, prepared);
    }
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let recipe = prepared.recipe.ok_or_else(|| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native regsub source issuer",
        ))
    })?;
    let original = prepared.originals;
    let mapping = if prepared.common.all && prepared.start == 0 {
        let substitution = ops.native_string_bytes(&original[2]).map_err(host)?;
        if recipe.literal_mapping_substitution(&substitution) {
            let pattern = ops.native_string_bytes(&original[0]).map_err(host)?;
            recipe.literal_mapping(true, 0, &pattern, &substitution)
        } else {
            false
        }
    } else {
        false
    };
    let (result, count) = if mapping {
        native_literal_substitution(ops, prepared, recipe)?
    } else {
        let compiled = native_compiled_pattern::<O, E>(
            ops,
            &original[0],
            recipe,
            prepared.common.flags,
            prepared.version,
        )
        .map_err(RegsubError::Regex)?;
        let subject = if ops.same_object(&original[1], &original[0]).ok_or_else(|| {
            host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regsub subject identity",
            ))
        })? {
            ops.regex_duplicate(&original[1]).map_err(host)?
        } else {
            ops.regex_borrow(&original[1])
        };
        let units = ops
            .native_unicode_units(ops.regex_object(&subject))
            .map_err(host)?;
        let substitution = if ops.same_object(&original[2], &original[0]).ok_or_else(|| {
            host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regsub substitution identity",
            ))
        })? {
            ops.regex_duplicate(&original[2]).map_err(host)?
        } else {
            ops.regex_borrow(&original[2])
        };
        let replacement = ops
            .native_unicode_units(ops.regex_object(&substitution))
            .map_err(host)?;
        let characters =
            native_regex_characters(&units, "native regsub character width").map_err(host)?;
        native_regsub_matches::<O, E>(ops, prepared, &compiled, &units, &replacement, &characters)?
    };
    Ok(NativeRegsubResult {
        result: match result {
            Some(result) => result,
            None => ops.regex_borrow(&original[1]),
        },
        count,
        target: prepared.target,
    })
}

fn native_substitution_units(
    specification: &[u32],
    subject: &[u32],
    matches: &[RegMatch],
) -> Result<Vec<u32>, tcl_syntax::value::ValueError> {
    let mut output = Vec::new();
    let mut cursor = 0;
    while cursor < specification.len() {
        let unit = specification[cursor];
        let mut group = (unit == u32::from(b'&')).then_some(0);
        if unit == u32::from(b'\\')
            && let Some(&next) = specification.get(cursor + 1)
        {
            if (u32::from(b'0')..=u32::from(b'9')).contains(&next) {
                group = Some((next - u32::from(b'0')) as usize);
                cursor += 1;
            } else if matches!(next, 38 | 92) {
                output.push(next);
                cursor += 2;
                continue;
            }
        }
        if let Some(group) = group {
            if let Some(span) = matches.get(group)
                && span.so != NO_MATCH
            {
                let range = subject.get(span.so..span.eo).ok_or(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native regsub submatch geometry",
                    ),
                )?;
                output.extend_from_slice(range);
            }
        } else {
            output.push(unit);
        }
        cursor += 1;
    }
    Ok(output)
}

struct RegsubCommandInputs<T, R> {
    prefix: T,
    subject: T,
    compiled: std::rc::Rc<std::cell::RefCell<R>>,
}

type RegsubCommandPreparation<O, E> = Result<
    RegsubCommandInputs<<O as NativeRegsubObjects>::Object, <E as RegexEngine>::Regex>,
    RegsubError<<O as NativeRegsubObjects>::Error>,
>;

fn prepare_regsub_command<O: NativeRegsubObjects + NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    prepared: &OriginalRegsubPreparation<'_, O::Value>,
) -> RegsubCommandPreparation<O, E> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let original = prepared.originals;
    let recipe = ops.regex_recipe().map_err(host)?.ok_or_else(|| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native regsub compiled pattern issuer",
        ))
    })?;
    let compiled = native_compiled_pattern::<O, E>(
        ops,
        &original[0],
        recipe,
        prepared.common.flags,
        prepared.version,
    )
    .map_err(RegsubError::Regex)?;
    let members = ops.regex_members(&original[2]).map_err(host)?;
    if members.is_empty() {
        return Err(RegsubError::Regex(RegexError::from(
            crate::CmdError::with_error_code_bytes(
                b"command prefix must be a list of at least one element",
                b"TCL OPERATION REGSUB CMDEMPTY",
            ),
        )));
    }
    drop(members);
    // Prefix conversion can replace the original pattern representation.
    drop(compiled);
    let compiled = native_compiled_pattern::<O, E>(
        ops,
        &original[0],
        recipe,
        prepared.common.flags,
        prepared.version,
    )
    .map_err(RegsubError::Regex)?;
    let subject = if ops.same_object(&original[1], &original[0]).ok_or_else(|| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original regsub subject identity",
        ))
    })? {
        ops.regex_duplicate(&original[1]).map_err(host)?
    } else {
        ops.regex_borrow(&original[1])
    };
    let prefix = if ops.same_object(&original[2], &original[0]).ok_or_else(|| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original regsub prefix identity",
        ))
    })? {
        ops.regex_duplicate(&original[2]).map_err(host)?
    } else {
        ops.regex_borrow(&original[2])
    };
    Ok(RegsubCommandInputs {
        prefix,
        subject,
        compiled,
    })
}

fn native_callback_arguments<O: NativeRegsubObjects>(
    ops: &mut O,
    nsubs: usize,
    matches: &[RegMatch],
    units: &[u32],
) -> Result<Vec<O::Object>, ValueError> {
    (0..=nsubs)
        .map(|index| {
            let capture_units = match matches.get(index) {
                Some(span)
                    if span.so != NO_MATCH && span.so <= span.eo && span.eo <= units.len() =>
                {
                    &units[span.so..span.eo]
                }
                Some(span) if span.so != NO_MATCH => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native regex submatch geometry",
                    ));
                }
                _ => &[],
            };
            ops.regex_unicode(capture_units)
        })
        .collect()
}

/// C9 prefix substitution over original Unicode storage and List members.
/// Subject/prefix aliasing to the pattern selects genuine duplicates. Callback
/// result append precedes result reset and original subject Unicode refetch.
pub fn regsub_command_original<
    'a,
    O: NativeRegsubObjects + NativeRegexObjects<E>,
    E: RegexEngine,
>(
    ops: &mut O,
    prepared: &OriginalRegsubPreparation<'a, O::Value>,
) -> NativeRegsubOutcome<'a, O> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    if !prepared.command || prepared.version < TclVersion::V9_0 {
        return Err(host(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native C9 regsub command prefix",
            ),
        ));
    }
    let original = prepared.originals;
    let RegsubCommandInputs {
        compiled,
        subject,
        prefix,
    } = prepare_regsub_command::<O, E>(ops, prepared)?;
    let mut units = ops
        .native_unicode_units(ops.regex_object(&subject))
        .map_err(host)?;
    let mut offset = prepared.start;
    let nsubs = E::nsub(&compiled.borrow());
    let mut result = None;
    let mut count = 0;
    while offset <= units.len() {
        let characters =
            native_regex_characters(&units, "native regex character width").map_err(host)?;
        let notbol = offset > 0 && units[offset - 1] != u32::from(b'\n');
        let Some(matches) = E::exec(&mut compiled.borrow_mut(), &characters, offset, notbol) else {
            break;
        };
        let whole = matches.first().copied().ok_or_else(|| {
            host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regex match geometry",
            ))
        })?;
        if whole.so < offset || whole.eo < whole.so || whole.eo > units.len() {
            return Err(host(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native regex match geometry",
                ),
            ));
        }
        if result.is_none() {
            let accumulator = ops.regex_unicode(&[]).map_err(host)?;
            ops.regex_append_unicode(&accumulator, &units[..offset])
                .map_err(host)?;
            result = Some(accumulator);
        }
        let accumulator = result.as_mut().expect("matched result owner");
        ops.regex_append_unicode(accumulator, &units[offset..whole.so])
            .map_err(host)?;
        let arguments = native_callback_arguments(ops, nsubs, &matches, &units).map_err(host)?;
        ops.regex_eval(ops.regex_object(&prefix), &arguments)
            .map_err(RegsubError::Eval)?;
        drop(arguments);
        ops.regex_append_current_result(accumulator).map_err(host)?;
        ops.regex_reset_result().map_err(host)?;
        units = ops
            .native_unicode_units(ops.regex_object(&subject))
            .map_err(host)?;
        count += 1;
        offset = whole.eo;
        if whole.eo == whole.so {
            if offset < units.len() {
                ops.regex_append_unicode(accumulator, &units[offset..=offset])
                    .map_err(host)?;
            }
            offset += 1;
        }
        if !prepared.common.all {
            break;
        }
    }
    let result = match result {
        Some(result) => {
            if offset < units.len() {
                ops.regex_append_unicode(&result, &units[offset..])
                    .map_err(host)?;
            }
            result
        }
        None => ops.regex_borrow(&original[1]),
    };
    Ok(NativeRegsubResult {
        result,
        count,
        target: prepared.target,
    })
}

/// Execute the already selected original option plan without another scan.
pub fn regsub_eval_original<'a, E: RegexEngine, Err, V>(
    prepared: &OriginalRegsubPreparation<'a, V>,
    eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
) -> Result<OriginalRegsubResult<'a, V>, RegsubError<Err>> {
    if prepared.command {
        return Err(RegsubError::Regex(RegexError::from(crate::CmdError::from(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original native regsub callback owner",
            ),
        ))));
    }
    let rest = prepared.rest.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let result = regsub_selected::<E, Err>(
        &rest,
        prepared.version,
        prepared.start,
        &prepared.common,
        prepared.command,
        eval,
    )?;
    Ok(OriginalRegsubResult {
        text: result.text,
        count: result.count,
        target: prepared.target,
    })
}

/// Resolve a regsub start offset before a runtime command callback borrows
/// the adapter. Native engines preserve their exact selected index policy.
///
/// # Errors
/// Invalid options, arguments, or start index.
pub fn regsub_start<O: ValueOps>(
    ops: &mut O,
    args: &[&[u8]],
    version: TclVersion,
) -> Result<usize, RegexError> {
    let (common, _, offset) = regsub_option_scan(&mut ByteOptionArguments(args), version)?;
    let rest = &args[offset..];
    if rest.len() < 3 || rest.len() > 4 {
        return Err(wrong_args(REGSUB_USAGE));
    }
    let (characters, _) = decode_utf8(rest[1]);
    common.start.as_ref().map_or(Ok(0), |spec| {
        resolve_start_checked(ops, spec, characters.len())
    })
}

/// C-release compatibility adapter. Native engines use [`regsub_start`] and
/// [`regsub_eval_at`] so Jim never inherits a C-version offset parser.
///
/// # Errors
/// Invalid arguments, regex failures, or callback completion.
pub fn regsub_eval<E: RegexEngine, Err>(
    args: &[&[u8]],
    version: TclVersion,
    eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
) -> Result<RegsubResult, RegsubError<Err>> {
    let (common, _, offset) = regsub_option_scan(&mut ByteOptionArguments(args), version)?;
    let rest = &args[offset..];
    if rest.len() < 3 || rest.len() > 4 {
        return Err(wrong_args(REGSUB_USAGE).into());
    }
    let (characters, _) = decode_utf8(rest[1]);
    let start = regsub_start_offset(common.start.as_deref(), characters.len(), version)?;
    regsub_eval_at::<E, Err>(args, version, start, eval)
}

/// Drive `regsub` over the engine `E`, **without** a script evaluator. `args`
/// is the command's arguments without the command name; the result string +
/// count + optional var name are returned for the adapter to apply.
///
/// The option table is 9.0's (this crate's baseline), so `-command` parses —
/// but serving it means running a Tcl command prefix, which this entry point
/// has no way to do. It therefore refuses, and, like C, only once a
/// substitution is actually due: a `-command` call whose pattern never matches
/// still returns the subject unchanged, and an unusable command prefix is still
/// rejected up front, both as tclsh 9.0.4 does. A caller that *can* evaluate —
/// a runtime rather than the registry's const-folder — calls [`regsub_eval`]
/// instead and gets `-command` for real.
///
/// # Errors
/// Option/arg/compile errors as ready-to-report [`RegexError`] messages.
pub fn regsub<E: RegexEngine>(args: &[&[u8]]) -> Result<RegsubResult, RegexError> {
    regsub_eval::<E, RegexError>(args, TclVersion::V9_0, |_| {
        Err(RegexError::new(
            b"regsub -command is not yet supported".to_vec(),
        ))
    })
    .map_err(|e| match e {
        RegsubError::Regex(e) | RegsubError::Eval(e) => e,
    })
}

fn regsub_start_offset(
    spec: Option<&[u8]>,
    char_len: usize,
    version: TclVersion,
) -> Result<usize, RegexError> {
    spec.map_or(Ok(0), |spec| {
        let text = String::from_utf8_lossy(spec);
        let syntax = tcl_dialect::IndexSyntax::for_version(version);
        if syntax.regex_start_grammar() == tcl_dialect::RegexStartGrammar::Integer {
            let flags = tcl_syntax::number::ParseFlags {
                integer_only: true,
                ..tcl_syntax::number::ParseFlags::for_syntax(syntax.numbers)
            };
            if tcl_syntax::number::parse_whole_with(&text, flags).is_none() {
                return Err(RegexError::new(
                    format!("expected integer but got \"{text}\"").into_bytes(),
                ));
            }
        }
        crate::index::resolve_in(&text, char_len, syntax)
            .map(|value| usize::try_from(value).unwrap_or(0))
            .map_err(RegexError::from)
    })
}

/// Drive `regsub` over the engine `E` for `version`, evaluating a `-command`
/// prefix through `eval`.
///
/// `eval` receives the whole command word list — the prefix's own words
/// followed by the matched text and each submatch (a non-participating
/// submatch is the empty string) — exactly as C hands it to `Tcl_EvalObjv`,
/// and returns the replacement text. It is called once per substitution, so a
/// `-all` run calls it per match; a pattern that never matches never calls it.
///
/// `version` selects C's option table and the noun its errors use, so
/// `-command` is refused before 9.0 in that release's own words rather than
/// served or refused generically.
///
/// # Errors
/// [`RegsubError::Regex`] for `regsub`'s own diagnostics, [`RegsubError::Eval`]
/// for a failing command prefix.
pub fn regsub_eval_at<E: RegexEngine, Err>(
    args: &[&[u8]],
    version: TclVersion,
    start: usize,
    eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
) -> Result<RegsubResult, RegsubError<Err>> {
    let (c, command, i) = regsub_option_scan(&mut ByteOptionArguments(args), version)?;
    regsub_selected::<E, Err>(&args[i..], version, start, &c, command, eval)
}

fn regsub_selected<E: RegexEngine, Err>(
    rest: &[&[u8]],
    version: TclVersion,
    start: usize,
    c: &Common,
    command: bool,
    mut eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
) -> Result<RegsubResult, RegsubError<Err>> {
    if rest.len() < 3 || rest.len() > 4 {
        return Err(wrong_args(REGSUB_USAGE).into());
    }
    let pattern = rest[0];
    let str_bytes = rest[1];
    let subspec = rest[2];
    let var = rest.get(3).map(|&v| v.to_vec());

    // C splits and checks the command prefix *before* the match loop, so an
    // unusable prefix is an error even when the pattern never matches (tclsh
    // 9.0.4: `regsub -command {z} abc {}` is still `command prefix must be a
    // list of at least one element`).
    let prefix = if command {
        Some(command_prefix(subspec)?)
    } else {
        None
    };

    let (cps, byteoff) = decode_utf8(str_bytes);
    let char_len = cps.len();

    let mut re = E::compile(pattern, c.flags).map_err(|d| compile_error(version, &d))?;
    let nsubs = E::nsub(&re);

    let mut offset = start;

    // The **literal empty pattern** is not the general empty-match case. C
    // diverts a metacharacter-free pattern away from the RE engine into a
    // literal string map (`Tcl_RegsubObjCmd`'s "simple one pair string map
    // situation") and spells the empty pattern out as its own branch of it
    // (`if (slen == 0)`): the replacement goes before each character and never
    // at end-of-string, so the count is exactly the subject's character length
    // and an empty subject substitutes nothing at all.
    //
    // That is a `regsub` rule, not an engine rule, which is why it sits in
    // `regsub`'s own loop rather than in the `RegexEngine` the two commands
    // share. An RE that merely *can* match empty keeps the general rule
    // (`regsub -all {(?:)} abc X` is still 4 / `XaXbXcX`), and `regexp -all`
    // never sees this branch at all — it counts 3 for both patterns and still
    // reports one match on an empty subject where `regsub` reports none. The
    // two commands disagree at end-of-string on purpose.
    //
    // The guard is C's, term for term: `-all`, a resolved start of 0, no
    // `-command`, and a subspec free of `&` and `\` — either of those sends C
    // down the general path instead, which is why `regsub -all {} abc &`
    // counts 4. C's remaining term, "the pattern holds none of
    // `*+?{}()[].\|^$`", is implied by the pattern being empty. Measured
    // identical on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0.
    // …and only where one replacement per scalar is the release's own answer.
    // The substitution count tracks `string length`, so it follows the
    // release's `StringCharacterModel`, not the Unicode-scalar count: for
    // U+1D11E, `regsub -all {} $s X` counts 4 on tclsh 8.4.20/8.5.19 (the
    // scalar is never assembled, so each UTF-8 byte is a position), 2 on
    // 8.6.18 (UTF-16 code units) and 1 on 9.0.4/9.1b0 (scalars).
    //
    // The emit below walks scalars, so it can only be right where the model
    // counts scalars too — always under 9.x, and under 8.x exactly when the
    // subject holds no supplementary scalar. Where it would not be, this
    // declines the fast path rather than assert one release's count under
    // another. The general loop's answer is also wrong there, differently;
    // correcting it needs the model's *units* threaded through the emit, not
    // just its count (#2170).
    let scalars_are_the_models_units = std::str::from_utf8(str_bytes)
        .is_ok_and(|text| version.string_character_model().count(text) == char_len);

    if c.all
        && offset == 0
        && !command
        && pattern.is_empty()
        && scalars_are_the_models_units
        && !subspec.iter().any(|&b| b == b'&' || b == b'\\')
    {
        // Saturating: on a 32-bit target (the WASM runtime) a long subject
        // times a long replacement can overflow `usize`, and a capacity hint
        // must never be the thing that aborts. Too small only costs a regrow.
        let hint = subspec
            .len()
            .saturating_mul(char_len)
            .saturating_add(str_bytes.len());
        let mut text = Vec::with_capacity(hint);
        for i in 0..char_len {
            text.extend_from_slice(subspec);
            text.extend_from_slice(&str_bytes[byteoff[i]..byteoff[i + 1]]);
        }
        return Ok(RegsubResult {
            text,
            count: i64::try_from(char_len).unwrap_or(i64::MAX),
            var,
        });
    }

    let mut result: Vec<u8> = Vec::new();
    let mut count: i64 = 0;

    while offset <= char_len {
        let notbol = offset > 0 && cps[offset - 1] != i32::from(b'\n');
        let Some(matches) = E::exec(&mut re, &cps, offset, notbol) else {
            break;
        };
        if count == 0 && offset > 0 {
            // Copy the skipped prefix when a `-start` offset was given.
            result.extend_from_slice(&str_bytes[..byteoff[offset]]);
        }
        count += 1;

        let m0 = matches[0];
        // Text before this match.
        result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[m0.so]]);
        if let Some(prefix) = prefix.as_ref() {
            // `-command`: the prefix's words, then the whole match and each
            // submatch as further words, evaluated as one command whose result
            // is the replacement (C builds the same word list for
            // `Tcl_EvalObjv`).
            let mut words = prefix.clone();
            words.extend((0..=nsubs).map(|k| match matches.get(k) {
                Some(rm) if rm.so != NO_MATCH => {
                    slice_match(str_bytes, &byteoff, rm.so, rm.eo).to_vec()
                }
                // A submatch that did not participate is an empty word, not a
                // missing one (tclsh 9.0.4: `regsub -command {(a)|(b)} ab cap`
                // passes `a a {}`).
                _ => Vec::new(),
            }));
            result.extend_from_slice(&eval(&words).map_err(RegsubError::Eval)?);
        } else {
            // The substitution spec, with `&`/`\N` expanded.
            apply_subspec(&mut result, subspec, &matches, nsubs, str_bytes, &byteoff);
        }

        // Advance, always consuming at least one char on an empty match.
        if m0.eo == offset {
            if offset < char_len {
                result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[offset + 1]]);
            }
            offset += 1;
        } else {
            offset = m0.eo;
            if m0.so == m0.eo {
                if offset < char_len {
                    result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[offset + 1]]);
                }
                offset += 1;
            }
        }
        if !c.all {
            break;
        }
    }

    let text = if count == 0 {
        str_bytes.to_vec()
    } else {
        if offset < char_len {
            result.extend_from_slice(&str_bytes[byteoff[offset]..]);
        }
        result
    };

    Ok(RegsubResult { text, count, var })
}

/// Expand a `regsub` substitution spec into `out`: `&` / `\0` → whole match,
/// `\N` → capture group N, `\\` → `\`, `\&` → `&`; any other run is copied
/// verbatim. Mirrors the `wsubspec` scan in `Tcl_RegsubObjCmd`.
fn apply_subspec(
    out: &mut Vec<u8>,
    sub: &[u8],
    matches: &[RegMatch],
    nsubs: usize,
    str_bytes: &[u8],
    byteoff: &[usize],
) {
    let mut k = 0;
    let mut run = 0; // start of the current verbatim run
    while k < sub.len() {
        let ch = sub[k];
        let idx: usize;
        if ch == b'&' {
            idx = 0;
        } else if ch == b'\\' {
            match sub.get(k + 1) {
                Some(&d) if d.is_ascii_digit() => idx = (d - b'0') as usize,
                Some(&d) if d == b'\\' || d == b'&' => {
                    // Literal `\` or `&`: flush the run, emit the bare char.
                    out.extend_from_slice(&sub[run..k]);
                    out.push(d);
                    k += 2;
                    run = k;
                    continue;
                }
                _ => {
                    // Backslash before any other char: keep both verbatim.
                    k += 1;
                    continue;
                }
            }
        } else {
            k += 1;
            continue;
        }
        // Reached for `&` or `\N`: flush the verbatim run, then the group.
        out.extend_from_slice(&sub[run..k]);
        if idx <= nsubs
            && let Some(rm) = matches.get(idx)
            && rm.so != NO_MATCH
        {
            // Guard the engine-supplied offsets (see `slice_match`): a foreign ARE
            // engine returning an out-of-range `[so, eo)` must not index-panic.
            out.extend_from_slice(slice_match(str_bytes, byteoff, rm.so, rm.eo));
        }
        k += if ch == b'\\' { 2 } else { 1 };
        run = k;
    }
    out.extend_from_slice(&sub[run..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_error_wrapper_retains_arity_and_primitive_state_actions() {
        let usage = wrong_args(b"regexp native\0\xff").into_cmd_error();
        assert_eq!(
            usage.error_code_update(),
            &crate::CmdErrorCodeUpdate::WrongArguments
        );
        let details = crate::CmdErrorDetails {
            string_result: None,
            message: b"RAW\0\xc0\x80\xff".to_vec(),
            error_code: crate::CmdErrorCodeUpdate::Unchanged,
            error_info: Some(b"INFO\0\xff".to_vec()),
            error_line: Some(17),
            primitive_getter: None,
        };
        assert_eq!(
            RegexError::from(crate::CmdError::from_byte_details(details.clone()))
                .into_cmd_error()
                .into_byte_details(),
            details
        );
    }

    #[test]
    fn original_match_targets_are_borrowed_and_assigned_before_the_next_match() {
        let args = ["-all", "a", "aba", "first", "second"].map(str::to_owned);
        let mut writes = Vec::new();
        let result = regexp_original::<ListOps, LiteralEngine, &'static str>(
            &mut ListOps,
            &args,
            TclVersion::V9_0,
            |_, name, value| {
                let original = if name == "first" { &args[3] } else { &args[4] };
                assert!(std::ptr::eq(name, original));
                writes.push((name.clone(), value));
                if writes.len() == 3 {
                    Err("setter failed")
                } else {
                    Ok(())
                }
            },
        );
        assert!(matches!(
            result,
            Err(RegexpExecutionError::Assignment("setter failed"))
        ));
        assert_eq!(
            writes,
            [
                ("first".into(), "a".into()),
                ("second".into(), "".into()),
                ("first".into(), "a".into())
            ]
        );
    }

    #[test]
    fn resolve_start_handles_integer_and_end_forms() {
        // regexp `-start` index: integer / end / end±N against the char
        // length, clamped to 0.
        assert_eq!(resolve_start_checked(&mut ListOps, b"5", 10).unwrap(), 5);
        assert_eq!(resolve_start_checked(&mut ListOps, b"0", 10).unwrap(), 0);
        assert_eq!(resolve_start_checked(&mut ListOps, b"end", 10).unwrap(), 9);
        assert_eq!(
            resolve_start_checked(&mut ListOps, b"end-2", 10).unwrap(),
            7
        );
        assert_eq!(
            resolve_start_checked(&mut ListOps, b"end+1", 10).unwrap(),
            10
        );
        assert_eq!(resolve_start_checked(&mut ListOps, b"1+1", 10).unwrap(), 2);
        assert_eq!(resolve_start_checked(&mut ListOps, b"0x2", 10).unwrap(), 2);
        assert_eq!(resolve_start_checked(&mut ListOps, b"+5", 10).unwrap(), 5);
        assert_eq!(resolve_start_checked(&mut ListOps, b"-3", 10).unwrap(), 0); // Tcl clamps negatives.
        assert!(resolve_start_checked(&mut ListOps, b"bad", 10).is_err());
        assert!(resolve_start_checked(&mut ListOps, b"end - 2", 10).is_err());
        assert_eq!(resolve_start_checked(&mut ListOps, b"end", 0).unwrap(), 0);
    }

    #[test]
    fn resolve_start_end_offset_saturates_without_overflow() {
        // A giant `end±N` must not overflow the isize add/sub. An `N` near
        // `isize::MAX` still parses, so `(len-1) ± N` is where the wrap would
        // happen — `saturating_*` pins it instead: `end+N` to "past the end" (the
        // match loop then finds nothing), `end-N` back to the start.
        let big = b"end+9223372036854775800"; // close to isize::MAX, parses fine
        assert_eq!(
            resolve_start_checked(&mut ListOps, big, 10).unwrap(),
            usize::try_from(i64::MAX).unwrap_or(usize::MAX)
        );
        assert_eq!(
            resolve_start_checked(&mut ListOps, b"end-9223372036854775800", 10).unwrap(),
            0
        );
        // A bignum operand is a bad index, not a silent reset to zero.
        assert!(resolve_start_checked(&mut ListOps, b"end+99999999999999999999999", 10).is_err());
    }

    #[test]
    fn slice_match_guards_out_of_range_engine_offsets() {
        // The byte-offset slice must never index-panic on a (foreign)
        // engine offset past the char→byte table or with `eo < so`.
        let bytes = b"hello";
        let byteoff = [0usize, 1, 2, 3, 4, 5]; // 5 chars + final len
        assert_eq!(slice_match(bytes, &byteoff, 1, 4), b"ell");
        // `eo` past the table → empty, not a panic.
        assert_eq!(slice_match(bytes, &byteoff, 0, 99), b"");
        // `so` past the table → empty.
        assert_eq!(slice_match(bytes, &byteoff, 99, 100), b"");
        // Inverted range → empty.
        assert_eq!(slice_match(bytes, &byteoff, 4, 1), b"");
        // The `NO_MATCH` sentinel as an index → empty (never indexes).
        assert_eq!(slice_match(bytes, &byteoff, NO_MATCH, NO_MATCH), b"");
    }

    /// A throwaway `ValueOps` whose `new_list` renders real Tcl list syntax, so
    /// an assertion reads exactly as tclsh prints the answer.
    #[derive(Default)]
    struct ListOps;

    impl ValueOps for ListOps {
        fn index_syntax(&self) -> Option<tcl_dialect::IndexSyntax> {
            Some(tcl_dialect::IndexSyntax::for_version(TclVersion::V9_0))
        }
        type Value = String;
        fn new_str(&mut self, s: &str) -> String {
            s.to_owned()
        }
        fn new_int(&mut self, n: i64) -> String {
            n.to_string()
        }
        fn new_double(&mut self, f: f64) -> String {
            tcl_syntax::number::format_double(f)
        }
        fn new_bool(&mut self, b: bool) -> String {
            (if b { "1" } else { "0" }).to_owned()
        }
        fn new_list(&mut self, items: Vec<String>) -> String {
            tcl_syntax::list::join_list(items)
        }
        fn as_bytes(&mut self, v: &String) -> std::rc::Rc<[u8]> {
            std::rc::Rc::from(v.as_bytes())
        }
        fn new_bytes(&mut self, bytes: &[u8]) -> Self::Value {
            self.new_str(std::str::from_utf8(bytes).expect("Unicode-only fixture input"))
        }

        fn as_int(&mut self, v: &String) -> Result<i64, tcl_syntax::value::ValueError> {
            v.parse()
                .map_err(|_| tcl_syntax::value::ValueError::NotInteger(v.clone()))
        }
        fn as_double(&mut self, _v: &String) -> Result<f64, tcl_syntax::value::ValueError> {
            Ok(0.0)
        }
        fn as_bool(&mut self, _v: &String) -> Result<bool, tcl_syntax::value::ValueError> {
            Ok(false)
        }
        fn list_elements(
            &mut self,
            v: &String,
        ) -> Result<Vec<String>, tcl_syntax::value::ValueError> {
            Ok(v.split_whitespace().map(str::to_owned).collect())
        }
    }

    impl NativeRegexSource for ListOps {
        fn regex_recipe(
            &self,
        ) -> Result<
            Option<tcl_syntax::native_regex::NativeRegexpRecipe>,
            tcl_syntax::value::ValueError,
        > {
            Ok(None)
        }
    }
    impl NativeRegexObjects<LiteralEngine> for ListOps {
        fn regex_cached_pattern(
            &self,
            _original: &String,
            _recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
            _flags: u32,
        ) -> Result<Option<std::rc::Rc<std::cell::RefCell<LiteralRe>>>, tcl_syntax::value::ValueError>
        {
            unreachable!("compatibility fixture")
        }
        fn regex_cached_glob(
            &self,
            _original: &String,
            _recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
            _flags: u32,
        ) -> Result<Option<std::rc::Rc<[u8]>>, tcl_syntax::value::ValueError> {
            Ok(None)
        }
        fn regex_install_pattern(
            &mut self,
            _original: &String,
            _recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
            _flags: u32,
            _compiled: std::rc::Rc<std::cell::RefCell<LiteralRe>>,
        ) -> Result<(), tcl_syntax::value::ValueError> {
            unreachable!("compatibility fixture")
        }
        fn regex_range_value(
            &mut self,
            _range: tcl_syntax::native_regex::NativeRegexpRange,
        ) -> Result<String, tcl_syntax::value::ValueError> {
            unreachable!("compatibility fixture")
        }
    }

    /// A stand-in engine for the *plumbing* tests. `regexp -about` and `regsub
    /// -command` are pure plumbing over `nsub` / `info_names` / `exec`, so a
    /// deliberately trivial provider exercises them without dragging the real
    /// ARE engine (a different crate) into this one's unit tests.
    ///
    /// Its pattern language: the text matches **literally** once `(` and `)`
    /// are dropped, `nsub` is the number of `(`, and every participating
    /// subexpression reports the whole match. A pattern of `!bad` fails to
    /// compile, so the compile-error path is reachable.
    struct LiteralEngine;

    struct LiteralRe {
        text: Vec<i32>,
        nsub: usize,
    }

    impl RegexEngine for LiteralEngine {
        type Regex = LiteralRe;

        fn compile(pattern: &[u8], _flags: RegexFlags) -> Result<LiteralRe, Vec<u8>> {
            if pattern == b"!bad" {
                return Err(b"brackets [] not balanced".to_vec());
            }
            let (cps, _) = decode_utf8(pattern);
            Ok(LiteralRe {
                nsub: cps.iter().filter(|&&c| c == i32::from(b'(')).count(),
                text: cps
                    .into_iter()
                    .filter(|&c| c != i32::from(b'(') && c != i32::from(b')'))
                    .collect(),
            })
        }

        fn nsub(re: &LiteralRe) -> usize {
            re.nsub
        }

        fn exec(
            re: &mut LiteralRe,
            cps: &[i32],
            offset: usize,
            _notbol: bool,
        ) -> Option<Vec<RegMatch>> {
            let n = re.text.len();
            let at =
                (offset..=cps.len().checked_sub(n)?).find(|&i| cps[i..i + n] == re.text[..])?;
            let whole = RegMatch { so: at, eo: at + n };
            Some(core::iter::repeat_n(whole, re.nsub + 1).collect())
        }
    }

    /// The same engine with `re_info` flags, to prove `-about` renders the
    /// engine's list in the engine's order.
    struct FlaggyEngine;

    impl RegexEngine for FlaggyEngine {
        type Regex = LiteralRe;
        fn compile(pattern: &[u8], flags: RegexFlags) -> Result<LiteralRe, Vec<u8>> {
            LiteralEngine::compile(pattern, flags)
        }
        fn nsub(re: &LiteralRe) -> usize {
            re.nsub
        }
        fn info_names(_re: &LiteralRe) -> Vec<&'static str> {
            vec!["REG_UNONPOSIX", "REG_ULOCALE"]
        }
        fn exec(
            re: &mut LiteralRe,
            cps: &[i32],
            offset: usize,
            notbol: bool,
        ) -> Option<Vec<RegMatch>> {
            LiteralEngine::exec(re, cps, offset, notbol)
        }
    }

    fn about(args: &[&[u8]]) -> Result<String, String> {
        let mut ops = ListOps;
        match regexp::<ListOps, LiteralEngine>(&mut ops, args, TclVersion::V9_0) {
            Ok(RegexpResult::Inline(v)) => Ok(v),
            Ok(RegexpResult::Count { count, .. }) => Ok(count.to_string()),
            Err(error) => Err(String::from_utf8_lossy(error.message_bytes()).into_owned()),
        }
    }

    #[test]
    fn regexp_about_reports_the_subexpression_count_and_info_list() {
        // Regression (#2124): `-about` was refused outright with `regexp -about
        // is not yet supported`, on every release. tclsh 8.4.20, 8.5.19,
        // 8.6.18, 9.0.4 and 9.1b0 all answer:
        //   % regexp -about {a(b)c}        ;# 1 {}
        //   % regexp -about abc            ;# 0 {}
        //   % regexp -about {(a)(b)}       ;# 2 {}
        //   % regexp -about {(a)} extraarg ;# 1 {}   (the subject is ignored)
        assert_eq!(about(&[b"-about", b"a(b)c"]).unwrap(), "1 {}");
        assert_eq!(about(&[b"-about", b"abc"]).unwrap(), "0 {}");
        assert_eq!(about(&[b"-about", b"(a)(b)"]).unwrap(), "2 {}");
        assert_eq!(about(&[b"-about", b"(a)", b"extraarg"]).unwrap(), "1 {}");
        assert_eq!(about(&[b"-about", b"--", b"(a)"]).unwrap(), "1 {}");
        // `-all`/`-indices`/`-start` are ignored in about mode, and `-nocase`
        // only reaches the compile.
        assert_eq!(about(&[b"-all", b"-about", b"(a)"]).unwrap(), "1 {}");
        assert_eq!(about(&[b"-about", b"-indices", b"(a)"]).unwrap(), "1 {}");
        assert_eq!(
            about(&[b"-about", b"-start", b"3", b"(a)"]).unwrap(),
            "1 {}"
        );
        // The engine's `re_info` names become the second element, in order.
        let mut ops = ListOps;
        let Ok(RegexpResult::Inline(v)) =
            regexp::<ListOps, FlaggyEngine>(&mut ops, &[b"-about", b"(a)"], TclVersion::V9_0)
        else {
            panic!("-about must answer")
        };
        assert_eq!(v, "1 {REG_UNONPOSIX REG_ULOCALE}");
    }

    #[test]
    fn regexp_about_keeps_cs_arity_and_inline_rules() {
        // C's arity bound is `(objc - i) < (2 - about)`, so `-about` needs the
        // pattern and nothing more, but a bare `regexp -about` is still a
        // wrong-# args (tclsh 8.6.18/9.0.4/9.1b0 word it with `?-option ...?`).
        assert_eq!(
            about(&[b"-about"]).unwrap_err(),
            "wrong # args: should be \"regexp ?-option ...? exp string \
             ?matchVar? ?subMatchVar ...?\""
        );
        assert_eq!(
            about(&[b"-about", b"--"]).unwrap_err(),
            "wrong # args: should be \"regexp ?-option ...? exp string \
             ?matchVar? ?subMatchVar ...?\""
        );
        // C checks `-inline` before branching to `-about`, so the mix error
        // wins — tclsh 8.4.20 through 9.1b0:
        //   % regexp -about -inline {(a)}
        //   regexp match variables not allowed when using -inline
        assert_eq!(
            about(&[b"-about", b"-inline", b"(a)"]).unwrap_err(),
            "regexp match variables not allowed when using -inline"
        );
        // A bad pattern is still a compile error, not an about answer.
        assert_eq!(
            about(&[b"-about", b"!bad"]).unwrap_err(),
            "cannot compile regular expression pattern: brackets [] not balanced"
        );
        // Without `-about`, the ordinary two-argument minimum still applies.
        assert_eq!(
            about(&[b"(a)"]).unwrap_err(),
            "wrong # args: should be \"regexp ?-option ...? exp string \
             ?matchVar? ?subMatchVar ...?\""
        );
    }

    #[test]
    fn compile_error_prefix_follows_the_release() {
        // tclsh 8.4.20 / 8.5.19 / 8.6.18 vs 9.0.4 / 9.1.0:
        //   % regexp {[a} b
        //   couldn't compile regular expression pattern: brackets [] not balanced
        //   cannot compile regular expression pattern: brackets [] not balanced
        for (version, verb) in [
            (TclVersion::V8_4, "couldn't"),
            (TclVersion::V8_5, "couldn't"),
            (TclVersion::V8_6, "couldn't"),
            (TclVersion::V9_0, "cannot"),
            (TclVersion::V9_1, "cannot"),
        ] {
            let want =
                format!("{verb} compile regular expression pattern: brackets [] not balanced");
            for args in [&[b"!bad".as_slice(), b"x"][..], &[b"-about", b"!bad"]] {
                let mut ops = ListOps;
                let Err(error) = regexp::<ListOps, LiteralEngine>(&mut ops, args, version) else {
                    panic!("{version:?}: a bad pattern must not compile")
                };
                assert_eq!(
                    String::from_utf8_lossy(error.message_bytes()),
                    want,
                    "{version:?}"
                );
            }
            assert_eq!(
                regsub_at(version, &[b"!bad", b"x", b"y"]).unwrap_err(),
                want,
                "{version:?}"
            );
        }
    }

    fn regsub_at(version: TclVersion, args: &[&[u8]]) -> Result<(String, i64), String> {
        let joined = |argv: &[Vec<u8>]| {
            let words: Vec<String> = argv
                .iter()
                .map(|w| String::from_utf8_lossy(w).into_owned())
                .collect();
            Ok::<Vec<u8>, String>(format!("<{}>", words.join("|")).into_bytes())
        };
        match regsub_eval::<LiteralEngine, String>(args, version, joined) {
            Ok(r) => Ok((String::from_utf8_lossy(&r.text).into_owned(), r.count)),
            Err(RegsubError::Regex(error)) => {
                Err(String::from_utf8_lossy(error.message_bytes()).into_owned())
            }
            Err(RegsubError::Eval(e)) => Err(e),
        }
    }

    #[test]
    fn regsub_command_is_refused_before_9_0_in_cs_own_words() {
        // Regression (#2124): `-command` was refused with `regsub -command is
        // not yet supported` on every release, where C has three answers.
        //
        // tclsh8.4.20 and tclsh8.5.19 (`Tcl_GetIndexFromObj`'s noun is
        // "switch" until 8.6, and the table has no `-command`):
        //   % regsub -command {a} abc {string toupper}
        //   bad switch "-command": must be -all, -nocase, -expanded, -line,
        //   -linestop, -lineanchor, -start, or --
        for v in [TclVersion::V8_4, TclVersion::V8_5] {
            assert_eq!(
                regsub_at(v, &[b"-command", b"a", b"abc", b"up"]).unwrap_err(),
                "bad switch \"-command\": must be -all, -nocase, -expanded, \
                 -line, -linestop, -lineanchor, -start, or --",
                "{v:?}"
            );
        }
        // tclsh8.6.18 — same table, the noun becomes "option":
        //   % regsub -command {a} abc {string toupper}
        //   bad option "-command": must be -all, -nocase, -expanded, -line,
        //   -linestop, -lineanchor, -start, or --
        assert_eq!(
            regsub_at(TclVersion::V8_6, &[b"-command", b"a", b"abc", b"up"]).unwrap_err(),
            "bad option \"-command\": must be -all, -nocase, -expanded, -line, \
             -linestop, -lineanchor, -start, or --"
        );
        // The rest of the 8.x enumeration is the 8.x table too, not 9.0's —
        // tclsh8.6.18 `regsub -bogus a b c` prints exactly the same list.
        assert_eq!(
            regsub_at(TclVersion::V8_6, &[b"-bogus", b"a", b"abc", b"x"]).unwrap_err(),
            "bad option \"-bogus\": must be -all, -nocase, -expanded, -line, \
             -linestop, -lineanchor, -start, or --"
        );
        // …while 9.0's own enumeration carries `-command` and puts `-nocase`
        // last but one (tclsh9.0.4 / 9.1b0 `regsub -bogus a b c`).
        assert_eq!(
            regsub_at(TclVersion::V9_0, &[b"-bogus", b"a", b"abc", b"x"]).unwrap_err(),
            "bad option \"-bogus\": must be -all, -command, -expanded, -line, \
             -linestop, -lineanchor, -nocase, -start, or --"
        );
    }

    #[test]
    fn regsub_command_evaluates_the_prefix_from_9_0() {
        // tclsh9.0.4 / 9.1b0 serve `-command` by appending the match and its
        // submatches to the prefix and substituting the result:
        //   % proc cap args {return "<[join $args |]>"}
        //   % regsub -command {(a)(b)} xabcy cap   ;# x<ab|a|b>cy
        //   % regsub -all -command {b} abc {list X} ;# aX bc
        for v in [TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                regsub_at(v, &[b"-command", b"(a)(b)", b"xabcy", b"cap"]).unwrap(),
                ("x<cap|ab|ab|ab>cy".to_owned(), 1),
                "{v:?}"
            );
            // The prefix is a *list*, so its words arrive as separate words.
            assert_eq!(
                regsub_at(v, &[b"-command", b"b", b"abc", b"list X"]).unwrap(),
                ("a<list|X|b>c".to_owned(), 1),
                "{v:?}"
            );
            // `-all` evaluates once per match.
            assert_eq!(
                regsub_at(v, &[b"-all", b"-command", b"b", b"abcb", b"f"]).unwrap(),
                ("a<f|b>c<f|b>".to_owned(), 2),
                "{v:?}"
            );
            // A pattern that never matches returns the subject and never
            // evaluates (tclsh9.0.4: `regsub -command {z} abc {string toupper}`
            // → `abc`).
            assert_eq!(
                regsub_at(v, &[b"-command", b"z", b"abc", b"f"]).unwrap(),
                ("abc".to_owned(), 0),
                "{v:?}"
            );
            // …but an unusable prefix is still rejected, match or no match
            // (tclsh9.0.4: `regsub -command {z} abc {}`).
            assert_eq!(
                regsub_at(v, &[b"-command", b"z", b"abc", b""]).unwrap_err(),
                "command prefix must be a list of at least one element",
                "{v:?}"
            );
            // Without `-command` the subspec is still expanded, not evaluated.
            assert_eq!(
                regsub_at(v, &[b"b", b"abc", b"[&]"]).unwrap(),
                ("a[b]c".to_owned(), 1),
                "{v:?}"
            );
        }
    }

    #[test]
    fn regsub_without_an_evaluator_still_refuses_command() {
        // The evaluator-free `regsub` keeps 9.0's option table (the VM's own
        // tests pin that enumeration) and refuses only when a substitution is
        // actually due — so a non-matching `-command` call still answers.
        let refused = regsub::<LiteralEngine>(&[b"-command", b"a", b"abc", b"f"])
            .err()
            .map(|error| String::from_utf8_lossy(error.message_bytes()).into_owned());
        assert_eq!(
            refused.as_deref(),
            Some("regsub -command is not yet supported")
        );
        let Ok(r) = regsub::<LiteralEngine>(&[b"-command", b"z", b"abc", b"f"]) else {
            panic!("a non-matching -command needs no evaluator")
        };
        assert_eq!(
            (String::from_utf8_lossy(&r.text).into_owned(), r.count),
            ("abc".to_owned(), 0)
        );
    }

    #[test]
    fn regsub_all_with_the_literal_empty_pattern_substitutes_once_per_character() {
        // Regression (#2147): `regsub -all {} abc X` counted 4 and produced
        // `XaXbXcX` — the general empty-match rule, one substitution before
        // each character *and* one at end-of-string. C never runs the engine
        // here: a metacharacter-free pattern goes down `Tcl_RegsubObjCmd`'s
        // literal string-map path, whose empty-pattern branch walks the
        // subject's characters and stops at the last one.
        //
        //   % regsub -all {} abc X r ;# 3, r is XaXbXc
        //
        // (tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1b0 all agree.)
        for v in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            assert_eq!(
                regsub_at(v, &[b"-all", b"", b"abc", b"X"]).unwrap(),
                ("XaXbXc".to_owned(), 3)
            );
            // One character, and a multi-character replacement, so a fix that
            // merely subtracted one from the old count is not enough: the
            // *text* has to lose its trailing replacement too.
            assert_eq!(
                regsub_at(v, &[b"-all", b"", b"a", b"YZ"]).unwrap(),
                ("YZa".to_owned(), 1)
            );
            // An empty subject has no characters, so nothing is substituted —
            // where the general rule (and `regexp -all`) still reports one
            // empty match at offset 0.
            assert_eq!(
                regsub_at(v, &[b"-all", b"", b"", b"X"]).unwrap(),
                (String::new(), 0)
            );
        }
    }

    #[test]
    fn regsub_empty_pattern_special_case_keeps_cs_guard_terms() {
        // Each row here is a way C *declines* its literal string-map path and
        // falls back to the engine's general empty-match rule, which counts
        // one more (a substitution at end-of-string). They are the guard's
        // reason for existing: drop a term and its row swings to the
        // once-per-character answer.
        let v = TclVersion::V9_0;
        // `&` in the substitution spec (C: `strpbrk(subSpec, "&\\")`).
        assert_eq!(
            regsub_at(v, &[b"-all", b"", b"abc", b"&"]).unwrap(),
            ("abc".to_owned(), 4)
        );
        // A backslash in the substitution spec, same C term.
        assert_eq!(
            regsub_at(v, &[b"-all", b"", b"abc", br"\0"]).unwrap(),
            ("abc".to_owned(), 4)
        );
        // A non-zero resolved `-start` (C: `offset == 0`).
        assert_eq!(
            regsub_at(v, &[b"-all", b"-start", b"1", b"", b"abc", b"X"]).unwrap(),
            ("aXbXcX".to_owned(), 3)
        );
        // `-start 0` resolves to 0, so the special case *does* apply.
        assert_eq!(
            regsub_at(v, &[b"-all", b"-start", b"0", b"", b"abc", b"X"]).unwrap(),
            ("XaXbXc".to_owned(), 3)
        );
        // `-command` (C: `command == 0`, 9.0's added term).
        assert_eq!(
            regsub_at(v, &[b"-all", b"-command", b"", b"abc", b"f"]).unwrap(),
            ("<f|>a<f|>b<f|>c<f|>".to_owned(), 4)
        );
        // No `-all` at all: a single substitution at offset 0, untouched.
        assert_eq!(
            regsub_at(v, &[b"", b"abc", b"X"]).unwrap(),
            ("Xabc".to_owned(), 1)
        );
    }
}
