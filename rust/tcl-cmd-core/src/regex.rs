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
//! Tcl's ARE matching has two separable halves: the **engine** (compile a
//! pattern, find matches in a codepoint string) and the **command plumbing**
//! (option parsing, the match/advance loop, `-indices`/`-inline`/`-start`/`-all`
//! handling, submatch-variable assignment, the `regsub` substitution-spec
//! expansion, and the match-count return semantics). The two runtimes share the
//! same engine *contract* but **not the same engine**: `runtime/rust` links the
//! real Tcl 9 Henry-Spencer ARE engine (so it is byte-for-byte tclsh), while the
//! bytecode VM drives the Rust `regex` crate (approximate — it does not
//! implement full ARE syntax such as `\m`/`\M`/`[[:<:]]`). This module is
//! everything *except* the engine: written once, run by both, so both runtimes
//! share one option set and the same char-offset semantics.
//!
//! All offsets here are **character** (codepoint) offsets, matching Tcl's index
//! model; [`decode_utf8`] maps the subject to codepoints plus a char→byte table
//! so the plumbing can slice the original bytes. The engine presents matches in
//! char offsets too (the FFI engine is natively codepoint-based; the VM's
//! crate-based engine translates byte↔char behind the seam).
//!
//! Semantics follow tclsh 9.0, which `runtime/rust`'s linked ARE engine
//! reproduces exactly.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;

use tcl_dialect::{ByteStringEncoding, StringCharacterModel, TclVersion};
use tcl_syntax::value::ValueOps;

use crate::prefix::OptionTable;

/// The "did not participate" sentinel for a subexpression's offset (mirrors the
/// engine's `(size_t)-1`).
pub const NO_MATCH: usize = usize::MAX;

/// One reported (sub-)match: half-open `[so, eo)` in **character** offsets.
/// `so == NO_MATCH` means the subexpression did not participate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegMatch {
    pub so: usize,
    pub eo: usize,
}

/// What a bounded match establishes (`docs/design/compiler/value-evaluation.md`
/// § *The typed precision result*). The only variant a compile-time fact may
/// be built from is `Exact`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegexpPrecision<V> {
    /// The search ran to completion and matched. Spans are half-open
    /// `[so, eo)` character offsets, exactly as [`RegMatch`] carries them;
    /// a non-participating subexpression is `None`.
    Exact {
        whole: V,
        groups: Vec<Option<V>>,
        /// Every span was produced by the exact path, never by an
        /// approximation.
        captures_exact: bool,
    },
    /// The search ran to completion and did not match. This is the only
    /// answer that proves a negative.
    NoMatch,
    /// The search did not establish either, for a recorded reason.
    Declined(PrecisionDecline),
}

impl RegexpPrecision<RegMatch> {
    /// A completed match as the plumbing's match vector: the whole match,
    /// then each subexpression, a non-participating one as [`NO_MATCH`].
    #[must_use]
    pub fn match_vector(&self) -> Option<Vec<RegMatch>> {
        match self {
            Self::Exact { whole, groups, .. } => Some(
                std::iter::once(*whole)
                    .chain(groups.iter().map(|group| {
                        group.unwrap_or(RegMatch {
                            so: NO_MATCH,
                            eo: NO_MATCH,
                        })
                    }))
                    .collect(),
            ),
            Self::NoMatch | Self::Declined(_) => None,
        }
    }

    /// A match vector as an exact answer: index 0 the whole match, a
    /// [`NO_MATCH`] entry a non-participating subexpression.
    #[must_use]
    pub fn exact(matches: &[RegMatch]) -> Self {
        let Some((&whole, groups)) = matches.split_first() else {
            return Self::NoMatch;
        };
        Self::Exact {
            whole,
            groups: groups
                .iter()
                .map(|group| (group.so != NO_MATCH).then_some(*group))
                .collect(),
            captures_exact: true,
        }
    }
}

/// Why a bounded match established neither a match nor a no-match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrecisionDecline {
    /// The engine's work budget ran out.
    FuelExhausted { spent: u64 },
    /// A recursion limit was reached.
    DepthExhausted { limit: u32 },
    /// A span came from an approximation.
    ApproximateCapture { group: usize },
    /// The pattern did not compile; carries the engine's detail bytes as
    /// [`RegexError`] already does.
    PatternError(RegexError),
    /// The option or form is outside what the core implements.
    FormUnsupported { option: &'static str },
    /// The request budget or the cancellation token stopped the match.
    Cancelled,
}

impl PrecisionDecline {
    /// The error the runtime raises for a match that established nothing:
    /// the search's own reason under C Tcl's `error while matching regular
    /// expression: ` prefix (`TclRegError`), a pattern's compile error as it
    /// stands, and an unsupported form as the refusal it is.
    #[must_use]
    pub fn into_error(self) -> RegexError {
        let reason = match self {
            Self::PatternError(error) => return error,
            Self::FormUnsupported { option } => {
                return RegexError(format!("{option} is not yet supported").into_bytes());
            }
            Self::FuelExhausted { .. } => "the search exceeded its work budget",
            Self::DepthExhausted { .. } => "the search exceeded its recursion limit",
            Self::ApproximateCapture { .. } => "a subexpression's span is only approximate",
            Self::Cancelled => "operation cancelled",
        };
        let mut message = b"error while matching regular expression: ".to_vec();
        message.extend_from_slice(reason.as_bytes());
        RegexError(message)
    }
}

/// An engine's identity and revision: part of every compiled-pattern cache
/// key, so a change to the engine cannot serve a stale compilation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EngineIdentity {
    /// The engine's name.
    pub name: &'static str,
    /// Bumped whenever what a pattern compiles to, or what it matches, may
    /// change.
    pub revision: u32,
}

/// The limits one analysis-path match runs under: the engine's work
/// budget, when the caller narrows it, the token that stops it, and the
/// counter its work is charged to.
#[derive(Clone, Copy, Debug, Default)]
pub struct MatchLimits<'c> {
    /// The engine's work budget; `None` for its own default. With a
    /// [`Self::spent`] counter it bounds every search charged to that
    /// counter together: each runs under what the counter leaves of it, so
    /// a `-all` loop cannot spend it once per match.
    pub fuel: Option<u64>,
    /// Set by the caller to stop the match.
    pub cancel: Option<&'c AtomicBool>,
    /// The work each search spent, in the engine's elementary steps, added
    /// here by an engine that meters its work — what the caller charges to
    /// a budget of its own (`docs/design/compiler/value-evaluation.md`
    /// § *Units and charges*). An engine that does not meter leaves it.
    pub spent: Option<&'c Cell<u64>>,
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
    /// ignore it). The answer is three-way: the match in **absolute**
    /// character offsets, a completed no-match, or why the search established
    /// neither — which is never a no-match.
    fn exec(
        re: &mut Self::Regex,
        cps: &[i32],
        offset: usize,
        notbol: bool,
    ) -> RegexpPrecision<RegMatch>;

    /// [`Self::exec`] under `limits`. An engine without a budget of its own
    /// to narrow, or a token to read, answers as [`Self::exec`] does.
    fn exec_within(
        re: &mut Self::Regex,
        cps: &[i32],
        offset: usize,
        notbol: bool,
        limits: MatchLimits<'_>,
    ) -> RegexpPrecision<RegMatch> {
        let _ = limits;
        Self::exec(re, cps, offset, notbol)
    }

    /// The engine's identity and revision (the pattern cache's key).
    const IDENTITY: EngineIdentity;

    /// The bytes a compiled pattern keeps on the heap — what the pattern
    /// cache charges against its bound. The default charges nothing beyond
    /// the handle.
    fn retained_bytes(re: &Self::Regex) -> usize {
        let _ = re;
        std::mem::size_of::<Self::Regex>()
    }
}

/// A `regexp`/`regsub` failure: the full, ready-to-report message bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegexError(pub Vec<u8>);

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

impl RegexFlags {
    /// The four booleans packed, for the pattern cache's key: nocase,
    /// expanded, linestop, lineanchor.
    #[must_use]
    pub const fn packed(self) -> u8 {
        (self.nocase as u8)
            | ((self.expanded as u8) << 1)
            | ((self.linestop as u8) << 2)
            | ((self.lineanchor as u8) << 3)
    }
}

/// The compiled-pattern cache key (`docs/design/compiler/value-evaluation.md`
/// § *The pattern cache, its bound, and the cancellation point*). Every field
/// is part of the identity a compiled pattern is only valid under.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PatternCacheKey {
    /// The pattern's exact bytes, never a normalised or trimmed form.
    pub pattern: Rc<[u8]>,
    /// [`RegexFlags`]' four booleans, packed ([`RegexFlags::packed`]).
    pub flags: u8,
    /// The engine's identity and revision, so a change to the engine cannot
    /// serve a stale compilation.
    pub engine: EngineIdentity,
    /// The target's character model and byte-string encoding, the two axes
    /// that change what a pattern matches.
    pub target: (Option<StringCharacterModel>, Option<ByteStringEncoding>),
}

/// The most compiled-pattern bytes one thread's cache retains. The bound is
/// on retained bytes, not entries: one pathological pattern compiles to far
/// more than an average one, and an entry count would not limit memory.
pub const PATTERN_CACHE_BYTES: usize = 4 * 1024 * 1024;

/// One cached compilation.
struct CachedPattern {
    compiled: Rc<dyn Any>,
    bytes: usize,
    /// The cache clock at the last use; the coldest entry goes first.
    used: u64,
}

/// A thread's compiled patterns, bounded by [`PATTERN_CACHE_BYTES`].
#[derive(Default)]
struct PatternCache {
    entries: HashMap<PatternCacheKey, CachedPattern>,
    retained: usize,
    clock: u64,
}

impl PatternCache {
    fn get(&mut self, key: &PatternCacheKey) -> Option<Rc<dyn Any>> {
        self.clock += 1;
        let clock = self.clock;
        self.entries.get_mut(key).map(|entry| {
            entry.used = clock;
            Rc::clone(&entry.compiled)
        })
    }

    /// Keep `compiled` under `key`, evicting the coldest entries until it
    /// fits. A compilation larger than the whole bound is not kept.
    fn insert(&mut self, key: PatternCacheKey, compiled: Rc<dyn Any>, bytes: usize) {
        if bytes > PATTERN_CACHE_BYTES {
            return;
        }
        while self.retained + bytes > PATTERN_CACHE_BYTES {
            let Some(coldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            if let Some(evicted) = self.entries.remove(&coldest) {
                self.retained -= evicted.bytes;
            }
        }
        self.clock += 1;
        let used = self.clock;
        if let Some(replaced) = self.entries.insert(
            key,
            CachedPattern {
                compiled,
                bytes,
                used,
            },
        ) {
            self.retained -= replaced.bytes;
        }
        self.retained += bytes;
    }
}

thread_local! {
    /// The thread's compiled-pattern cache.
    static PATTERN_CACHE: RefCell<PatternCache> = RefCell::new(PatternCache::default());
}

/// The bytes the calling thread's pattern cache retains now.
#[must_use]
pub fn pattern_cache_retained_bytes() -> usize {
    PATTERN_CACHE.with(|cache| cache.borrow().retained)
}

/// What an analysis-path run compiles and matches under: the target its
/// pattern's meaning depends on, the charge a compile pays before it runs,
/// and the limits every search runs under.
pub struct AnalysisMatch<'c> {
    /// The target's character model and byte-string encoding.
    pub target: (Option<StringCharacterModel>, Option<ByteStringEncoding>),
    /// Charge `units` of work; `false` once the caller's budget is spent,
    /// which cancels the compile rather than cache a partial entry.
    pub charge: &'c mut dyn FnMut(u64) -> bool,
    /// The engine budget and cancellation token each search runs under.
    pub limits: MatchLimits<'c>,
}

/// A `regexp` / `regsub` run on the analysis path that did not complete
/// normally: the error the command raises, or the decline of a match that
/// established nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegexFailure {
    /// The command raises this error.
    Error(RegexError),
    /// The match established neither a match nor a no-match.
    Declined(PrecisionDecline),
}

impl From<RegexError> for RegexFailure {
    fn from(error: RegexError) -> Self {
        Self::Error(error)
    }
}

impl RegexFailure {
    /// The runtime's reading: a decline is raised as the error it is.
    fn into_error(self) -> RegexError {
        match self {
            Self::Error(error) => error,
            Self::Declined(decline) => decline.into_error(),
        }
    }
}

/// How one `regexp` / `regsub` run compiles its pattern and matches.
enum Run<'a, 'c> {
    /// The runtime: a fresh compile, the engine's own budget.
    Runtime,
    /// The analysis path: the thread's bounded cache, the caller's charge
    /// and limits.
    Analysis(&'a mut AnalysisMatch<'c>),
}

impl Run<'_, '_> {
    /// Compile `pattern` under `flags`. On the analysis path the thread's
    /// cache answers first; a miss charges the pattern's length squared —
    /// the parser's worst case — before it compiles, and a refused charge is
    /// `Cancelled`, never a partial entry.
    fn compile<E: RegexEngine>(
        &mut self,
        pattern: &[u8],
        flags: RegexFlags,
    ) -> Result<Rc<RefCell<E::Regex>>, RegexFailure>
    where
        E::Regex: 'static,
    {
        let Self::Analysis(analysis) = self else {
            let re = E::compile(pattern, flags).map_err(|detail| compile_error(&detail))?;
            return Ok(Rc::new(RefCell::new(re)));
        };
        let key = PatternCacheKey {
            pattern: Rc::from(pattern),
            flags: flags.packed(),
            engine: E::IDENTITY,
            target: analysis.target,
        };
        if let Some(hit) = PATTERN_CACHE.with(|cache| cache.borrow_mut().get(&key))
            && let Ok(re) = hit.downcast::<RefCell<E::Regex>>()
        {
            return Ok(re);
        }
        let length = u64::try_from(pattern.len()).unwrap_or(u64::MAX);
        if !(analysis.charge)(length.saturating_mul(length)) {
            return Err(RegexFailure::Declined(PrecisionDecline::Cancelled));
        }
        let re = E::compile(pattern, flags).map_err(|detail| {
            RegexFailure::Declined(PrecisionDecline::PatternError(compile_error(&detail)))
        })?;
        let bytes = E::retained_bytes(&re).saturating_add(pattern.len());
        let re = Rc::new(RefCell::new(re));
        let kept: Rc<dyn Any> = Rc::clone(&re) as Rc<dyn Any>;
        PATTERN_CACHE.with(|cache| cache.borrow_mut().insert(key, kept, bytes));
        Ok(re)
    }

    /// One search. On the analysis path it runs under the caller's limits,
    /// and a match with an approximate span declines: a capture consumer
    /// needs exact captures.
    fn exec<E: RegexEngine>(
        &self,
        re: &RefCell<E::Regex>,
        cps: &[i32],
        offset: usize,
        notbol: bool,
    ) -> RegexpPrecision<RegMatch> {
        let mut re = re.borrow_mut();
        match self {
            Self::Runtime => E::exec(&mut re, cps, offset, notbol),
            Self::Analysis(analysis) => {
                match E::exec_within(&mut re, cps, offset, notbol, analysis.limits) {
                    RegexpPrecision::Exact {
                        groups,
                        captures_exact: false,
                        ..
                    } => RegexpPrecision::Declined(PrecisionDecline::ApproximateCapture {
                        // The engine says only that some span is approximate:
                        // the first participating subexpression is named.
                        group: groups
                            .iter()
                            .position(Option::is_some)
                            .map_or(0, |at| at + 1),
                    }),
                    answer => answer,
                }
            }
        }
    }
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
fn resolve_start_checked(spec: &[u8], char_len: usize) -> Result<usize, RegexError> {
    let text = core::str::from_utf8(spec).map_err(|_| RegexError(b"bad index".to_vec()))?;
    let idx = crate::index::resolve(text, char_len)
        .map_err(|e| RegexError(e.into_message().into_bytes()))?;
    Ok(usize::try_from(idx).unwrap_or(0))
}

/// The compile flags + `-all`/`-start` shared by both commands' option sets.
#[derive(Default)]
struct Common {
    all: bool,
    flags: RegexFlags,
    start: Option<Vec<u8>>,
}

fn wrong_args(usage: &[u8]) -> RegexError {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    RegexError(m)
}

/// Wrap a provider compile-error detail in Tcl's standard prefix.
fn compile_error(detail: &[u8]) -> RegexError {
    let mut m = b"cannot compile regular expression pattern: ".to_vec();
    m.extend_from_slice(detail);
    RegexError(m)
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
const REGEXP_NAMES: [&str; 11] = [
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
const REGEXP_OPTIONS: OptionTable<'static> = OptionTable::exact_only("option", &REGEXP_NAMES);

/// Drive `regexp` over the engine `E` and value-ops `O`. `args` is the
/// command's arguments **without** the command name.
///
/// # Errors
/// Option/arg/compile errors as ready-to-report [`RegexError`] messages, and
/// a search that established neither a match nor a no-match as the error it
/// raises ([`PrecisionDecline::into_error`]) — never a count of 0.
pub fn regexp<O: ValueOps, E: RegexEngine>(
    ops: &mut O,
    args: &[&[u8]],
) -> Result<RegexpResult<O::Value>, RegexError>
where
    E::Regex: 'static,
{
    regexp_run::<O, E>(ops, args, &mut Run::Runtime).map_err(RegexFailure::into_error)
}

/// `regexp` on the analysis path: compiled through the thread's bounded
/// pattern cache, each search under `analysis`' limits, and a match that
/// established nothing kept typed ([`RegexFailure::Declined`]) rather than
/// raised — the answer a compile-time fact may be built from, or its reason.
///
/// # Errors
/// [`RegexFailure::Error`] for the error the command raises;
/// [`RegexFailure::Declined`] for a pattern that does not compile, a refused
/// compile charge, an exhausted or cancelled search, or an approximate span.
pub fn regexp_analysis<O: ValueOps, E: RegexEngine>(
    ops: &mut O,
    args: &[&[u8]],
    analysis: &mut AnalysisMatch<'_>,
) -> Result<RegexpResult<O::Value>, RegexFailure>
where
    E::Regex: 'static,
{
    regexp_run::<O, E>(ops, args, &mut Run::Analysis(analysis))
}

/// The one `regexp` algorithm both paths run.
#[allow(clippy::too_many_lines)] // option scan + match loop + result build, read top-to-bottom
fn regexp_run<O: ValueOps, E: RegexEngine>(
    ops: &mut O,
    args: &[&[u8]],
    run: &mut Run<'_, '_>,
) -> Result<RegexpResult<O::Value>, RegexFailure>
where
    E::Regex: 'static,
{
    let mut c = Common::default();
    let mut indices = false;
    let mut inline = false;
    let mut about = false;

    let mut i = 0;
    while i < args.len() {
        let name = args[i];
        if name.first() != Some(&b'-') {
            break;
        }
        let idx = REGEXP_OPTIONS.index_of(name).map_err(RegexError)?;
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
            RE_START => match args.get(i) {
                Some(v) => {
                    c.start = Some(v.to_vec());
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

    let rest = &args[i..];
    // C: `(objc - i) < (2 - about)`. `-about` never looks at a subject, so the
    // pattern alone is enough — tclsh 8.4.20/8.5.19/8.6.18/9.0.4/9.1b0 all
    // answer `regexp -about {a(b)c}` with `1 {}` and give the same answer for
    // `regexp -about {(a)} extraarg`, while bare `regexp -about` is still a
    // wrong-# args.
    if rest.len() + usize::from(about) < 2 {
        return Err(wrong_args(REGEXP_USAGE).into());
    }
    // C tests `-inline` against the *exact* remaining count and does it before
    // branching to `-about`, so `regexp -about -inline {(a)}` is the mix error
    // rather than an about answer (tclsh 8.4.20–9.1b0). Without `-about` the
    // arity check above has already forced `>= 2`, so `!= 2` is the old `> 2`.
    if inline && rest.len() != 2 {
        return Err(
            RegexError(b"regexp match variables not allowed when using -inline".to_vec()).into(),
        );
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
        let re = run.compile::<E>(pattern, c.flags)?;
        let re = re.borrow();
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

    let re = run.compile::<E>(pattern, c.flags)?;
    let nsubs = E::nsub(&re.borrow());

    let mut offset = c
        .start
        .as_ref()
        .map_or(Ok(0), |spec| resolve_start_checked(spec, char_len))?;

    // Tcl's `all` doubles as flag + counter: starts 1 if `-all`, else 0.
    let mut all_count: i64 = i64::from(c.all);
    let mut inline_items: Vec<O::Value> = Vec::new();
    let mut last_matches: Option<Vec<RegMatch>> = None;

    loop {
        let notbol = notbol_at(&cps, offset);
        let answer = run.exec::<E>(&re, &cps, offset, notbol);
        if let RegexpPrecision::Declined(decline) = answer {
            return Err(RegexFailure::Declined(decline));
        }
        let Some(matches) = answer.match_vector() else {
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
            last_matches = Some(matches);
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
    // Non-inline: build the (var, value) pairs from the final match. `-all`'s
    // match variables hold the *last* match, so building once after the loop is
    // both correct and leak-free (no intermediate values are constructed).
    let lm = last_matches.expect("a match occurred");
    let assign = match_vars
        .iter()
        .enumerate()
        .map(|(k, &name)| {
            let v = build_match_item(ops, &lm, k, nsubs, indices, str_bytes, &byteoff);
            (name.to_vec(), v)
        })
        .collect();
    Ok(RegexpResult::Count {
        assign: Some(assign),
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
const REGSUB_NAMES_8: [&str; 8] = [
    "-all",
    "-nocase",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-start",
    "--",
];
const REGSUB_NAMES_9: [&str; 9] = [
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
const REGSUB_OPTIONS_8_4: OptionTable<'static> = OptionTable::exact_only("switch", &REGSUB_NAMES_8);
const REGSUB_OPTIONS_8_6: OptionTable<'static> = OptionTable::exact_only("option", &REGSUB_NAMES_8);
const REGSUB_OPTIONS_9_0: OptionTable<'static> = OptionTable::exact_only("option", &REGSUB_NAMES_9);

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
        .map_err(|_| RegexError(b"command prefix must be a valid list".to_vec()))?;
    let words = tcl_syntax::list::split_list(text)
        .map_err(|e| RegexError(e.message().as_bytes().to_vec()))?;
    if words.is_empty() {
        return Err(RegexError(
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
    args: &[&[u8]],
    options: &OptionTable<'static>,
) -> Result<(Common, bool, usize), RegexError> {
    let mut c = Common::default();
    let mut command = false;
    let mut i = 0;
    while i < args.len() {
        let name = args[i];
        if name.first() != Some(&b'-') {
            break;
        }
        let idx = options.index_of(name).map_err(RegexError)?;
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
            "-start" => match args.get(i) {
                Some(v) => {
                    c.start = Some(v.to_vec());
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
pub fn regsub<E: RegexEngine>(args: &[&[u8]]) -> Result<RegsubResult, RegexError>
where
    E::Regex: 'static,
{
    regsub_eval::<E, RegexError>(args, TclVersion::V9_0, |_| {
        Err(RegexError(b"regsub -command is not yet supported".to_vec()))
    })
    .map_err(|e| match e {
        RegsubError::Regex(e) | RegsubError::Eval(e) => e,
    })
}

/// `regsub` on the analysis path for `version`: compiled through the
/// thread's bounded pattern cache, each search under `analysis`' limits, and
/// a match that established nothing kept typed. A `-command` substitution
/// runs a script this path cannot, so one that is due declines
/// `FormUnsupported` — never the unsubstituted text, which would claim that
/// nothing matched.
///
/// # Errors
/// [`RegexFailure::Error`] for the error the command raises;
/// [`RegexFailure::Declined`] as for [`regexp_analysis`], and for a due
/// `-command` substitution.
pub fn regsub_analysis<E: RegexEngine>(
    args: &[&[u8]],
    version: TclVersion,
    analysis: &mut AnalysisMatch<'_>,
) -> Result<RegsubResult, RegexFailure>
where
    E::Regex: 'static,
{
    regsub_run::<E, RegexFailure>(
        args,
        version,
        |_| {
            Err(RegexFailure::Declined(PrecisionDecline::FormUnsupported {
                option: "regsub -command",
            }))
        },
        &mut Run::Analysis(analysis),
    )
    .map_err(|failure| match failure {
        RunFailure::Regex(error) => RegexFailure::Error(error),
        RunFailure::Eval(failure) => failure,
        RunFailure::Declined(decline) => RegexFailure::Declined(decline),
    })
}

/// A `regsub` run's failure on either path.
enum RunFailure<Err> {
    /// `regsub`'s own diagnostic.
    Regex(RegexError),
    /// The `-command` prefix's evaluation failed.
    Eval(Err),
    /// A match established neither a match nor a no-match.
    Declined(PrecisionDecline),
}

impl<Err> From<RegexError> for RunFailure<Err> {
    fn from(error: RegexError) -> Self {
        Self::Regex(error)
    }
}

impl<Err> From<RegexFailure> for RunFailure<Err> {
    fn from(failure: RegexFailure) -> Self {
        match failure {
            RegexFailure::Error(error) => Self::Regex(error),
            RegexFailure::Declined(decline) => Self::Declined(decline),
        }
    }
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
pub fn regsub_eval<E: RegexEngine, Err>(
    args: &[&[u8]],
    version: TclVersion,
    eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
) -> Result<RegsubResult, RegsubError<Err>>
where
    E::Regex: 'static,
{
    regsub_run::<E, Err>(args, version, eval, &mut Run::Runtime).map_err(|failure| match failure {
        RunFailure::Regex(error) => RegsubError::Regex(error),
        RunFailure::Eval(error) => RegsubError::Eval(error),
        RunFailure::Declined(decline) => RegsubError::Regex(decline.into_error()),
    })
}

/// The one `regsub` algorithm both paths run.
fn regsub_run<E: RegexEngine, Err>(
    args: &[&[u8]],
    version: TclVersion,
    mut eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
    run: &mut Run<'_, '_>,
) -> Result<RegsubResult, RunFailure<Err>>
where
    E::Regex: 'static,
{
    let (c, command, i) = regsub_option_scan(args, regsub_options(version))?;

    let rest = &args[i..];
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

    let re = run.compile::<E>(pattern, c.flags)?;
    let nsubs = E::nsub(&re.borrow());

    let mut offset = c
        .start
        .as_ref()
        .map_or(Ok(0), |spec| resolve_start_checked(spec, char_len))?;

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
        return Ok(RegsubResult {
            text: substitute_before_each_character(subspec, str_bytes, &byteoff),
            count: i64::try_from(char_len).unwrap_or(i64::MAX),
            var,
        });
    }

    let mut result: Vec<u8> = Vec::new();
    let mut count: i64 = 0;

    while offset <= char_len {
        let notbol = offset > 0 && cps[offset - 1] != i32::from(b'\n');
        let answer = run.exec::<E>(&re, &cps, offset, notbol);
        if let RegexpPrecision::Declined(decline) = answer {
            // A search cut short is not a no-match: the unsubstituted text
            // would claim nothing matched.
            return Err(RunFailure::Declined(decline));
        }
        let Some(matches) = answer.match_vector() else {
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
            result.extend_from_slice(&eval(&words).map_err(RunFailure::Eval)?);
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

/// The literal empty pattern's substitution: `subspec` before each
/// character of `str_bytes`, never at end-of-string (see `regsub_run`).
fn substitute_before_each_character(
    subspec: &[u8],
    str_bytes: &[u8],
    byteoff: &[usize],
) -> Vec<u8> {
    let char_len = byteoff.len().saturating_sub(1);
    // Saturating: on a 32-bit target (the WASM runtime) a long subject times
    // a long replacement can overflow `usize`, and a capacity hint must never
    // be the thing that aborts. Too small only costs a regrow.
    let hint = subspec
        .len()
        .saturating_mul(char_len)
        .saturating_add(str_bytes.len());
    let mut text = Vec::with_capacity(hint);
    for i in 0..char_len {
        text.extend_from_slice(subspec);
        text.extend_from_slice(&str_bytes[byteoff[i]..byteoff[i + 1]]);
    }
    text
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
    fn resolve_start_handles_integer_and_end_forms() {
        // regexp `-start` index: integer / end / end±N against the char
        // length, clamped to 0.
        assert_eq!(resolve_start_checked(b"5", 10).unwrap(), 5);
        assert_eq!(resolve_start_checked(b"0", 10).unwrap(), 0);
        assert_eq!(resolve_start_checked(b"end", 10).unwrap(), 9);
        assert_eq!(resolve_start_checked(b"end-2", 10).unwrap(), 7);
        assert_eq!(resolve_start_checked(b"end+1", 10).unwrap(), 10);
        assert_eq!(resolve_start_checked(b"1+1", 10).unwrap(), 2);
        assert_eq!(resolve_start_checked(b"0x2", 10).unwrap(), 2);
        assert_eq!(resolve_start_checked(b"+5", 10).unwrap(), 5);
        assert_eq!(resolve_start_checked(b"-3", 10).unwrap(), 0); // Tcl clamps negatives.
        assert!(resolve_start_checked(b"bad", 10).is_err());
        assert!(resolve_start_checked(b"end - 2", 10).is_err());
        assert_eq!(resolve_start_checked(b"end", 0).unwrap(), 0);
    }

    #[test]
    fn resolve_start_end_offset_saturates_without_overflow() {
        // A giant `end±N` must not overflow the isize add/sub. An `N` near
        // `isize::MAX` still parses, so `(len-1) ± N` is where the wrap would
        // happen — `saturating_*` pins it instead: `end+N` to "past the end" (the
        // match loop then finds nothing), `end-N` back to the start.
        let big = b"end+9223372036854775800"; // close to isize::MAX, parses fine
        assert_eq!(
            resolve_start_checked(big, 10).unwrap(),
            usize::try_from(i64::MAX).unwrap_or(usize::MAX)
        );
        assert_eq!(
            resolve_start_checked(b"end-9223372036854775800", 10).unwrap(),
            0
        );
        // A bignum operand is a bad index, not a silent reset to zero.
        assert!(resolve_start_checked(b"end+99999999999999999999999", 10).is_err());
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
        fn as_str(&mut self, v: &String) -> std::rc::Rc<str> {
            std::rc::Rc::from(v.as_str())
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
        ) -> RegexpPrecision<RegMatch> {
            let n = re.text.len();
            let at = cps
                .len()
                .checked_sub(n)
                .and_then(|last| (offset..=last).find(|&i| cps[i..i + n] == re.text[..]));
            let Some(at) = at else {
                return RegexpPrecision::NoMatch;
            };
            let whole = RegMatch { so: at, eo: at + n };
            RegexpPrecision::exact(&core::iter::repeat_n(whole, re.nsub + 1).collect::<Vec<_>>())
        }

        const IDENTITY: EngineIdentity = EngineIdentity {
            name: "literal",
            revision: 1,
        };
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
        ) -> RegexpPrecision<RegMatch> {
            LiteralEngine::exec(re, cps, offset, notbol)
        }

        const IDENTITY: EngineIdentity = EngineIdentity {
            name: "flaggy",
            revision: 1,
        };
    }

    /// An engine whose every search stops: what a search that ran out of
    /// budget looks like to the plumbing.
    struct StoppingEngine;

    impl RegexEngine for StoppingEngine {
        type Regex = LiteralRe;
        fn compile(pattern: &[u8], flags: RegexFlags) -> Result<LiteralRe, Vec<u8>> {
            LiteralEngine::compile(pattern, flags)
        }
        fn nsub(re: &LiteralRe) -> usize {
            re.nsub
        }
        fn exec(
            _re: &mut LiteralRe,
            _cps: &[i32],
            _offset: usize,
            _notbol: bool,
        ) -> RegexpPrecision<RegMatch> {
            RegexpPrecision::Declined(PrecisionDecline::FuelExhausted { spent: 7 })
        }
        const IDENTITY: EngineIdentity = EngineIdentity {
            name: "stopping",
            revision: 1,
        };
    }

    /// A search that established neither a match nor a no-match is raised on
    /// the runtime path — never a count of 0, never the unsubstituted text —
    /// and kept typed on the analysis path.
    #[test]
    fn a_stopped_search_is_raised_at_run_time_and_typed_in_analysis() {
        let raised =
            b"error while matching regular expression: the search exceeded its work budget";
        let mut ops = ListOps;
        assert!(matches!(
            regexp::<ListOps, StoppingEngine>(&mut ops, &[b"abc", b"xabcx"]),
            Err(RegexError(message)) if message == raised
        ));
        assert!(matches!(
            regsub::<StoppingEngine>(&[b"abc", b"xabcx", b"-"]),
            Err(RegexError(message)) if message == raised
        ));
        let mut charge = |_: u64| true;
        let mut analysis = AnalysisMatch {
            target: (None, None),
            charge: &mut charge,
            limits: MatchLimits::default(),
        };
        assert!(matches!(
            regexp_analysis::<ListOps, StoppingEngine>(
                &mut ops,
                &[b"abc", b"xabcx"],
                &mut analysis
            ),
            Err(RegexFailure::Declined(PrecisionDecline::FuelExhausted {
                spent: 7
            }))
        ));
        assert!(matches!(
            regsub_analysis::<StoppingEngine>(
                &[b"abc", b"xabcx", b"-"],
                TclVersion::V9_0,
                &mut analysis
            ),
            Err(RegexFailure::Declined(PrecisionDecline::FuelExhausted {
                spent: 7
            }))
        ));
    }

    /// The analysis path compiles through the thread's cache: a pattern is
    /// charged its length squared once, then served from the cache; a
    /// different target is a different key; a refused charge is `Cancelled`
    /// and caches nothing; and a `-command` substitution that is due
    /// declines rather than answer with the unsubstituted text.
    #[test]
    fn the_pattern_cache_charges_a_compile_once_per_key() {
        let mut ops = ListOps;
        let mut ledger: Vec<u64> = Vec::new();
        let mut charge = |units: u64| {
            ledger.push(units);
            true
        };
        {
            let mut analysis = AnalysisMatch {
                target: (None, None),
                charge: &mut charge,
                limits: MatchLimits::default(),
            };
            for _ in 0..2 {
                let answer = regexp_analysis::<ListOps, LiteralEngine>(
                    &mut ops,
                    &[b"cached-abc", b"xcached-abcx"],
                    &mut analysis,
                );
                assert!(matches!(answer, Ok(RegexpResult::Count { count: 1, .. })));
            }
            // Another target is another key.
            analysis.target = (Some(StringCharacterModel::Utf16CodeUnits), None);
            let answer = regexp_analysis::<ListOps, LiteralEngine>(
                &mut ops,
                &[b"cached-abc", b"xcached-abcx"],
                &mut analysis,
            );
            assert!(answer.is_ok());
            // A `-command` substitution that is due declines.
            assert!(matches!(
                regsub_analysis::<LiteralEngine>(
                    &[b"-command", b"b", b"abc", b"string toupper"],
                    TclVersion::V9_0,
                    &mut analysis
                ),
                Err(RegexFailure::Declined(
                    PrecisionDecline::FormUnsupported { .. }
                ))
            ));
        }
        assert_eq!(
            ledger,
            [100, 100, 1],
            "one compile per key, charged its length squared"
        );
        let mut refuse = |_: u64| false;
        let mut refused = AnalysisMatch {
            target: (None, None),
            charge: &mut refuse,
            limits: MatchLimits::default(),
        };
        assert!(matches!(
            regexp_analysis::<ListOps, LiteralEngine>(&mut ops, &[b"uncached", b"x"], &mut refused),
            Err(RegexFailure::Declined(PrecisionDecline::Cancelled))
        ));
    }

    /// The cache's bound is retained bytes: filling it past
    /// [`PATTERN_CACHE_BYTES`] evicts the coldest entries, and it never
    /// holds more.
    #[test]
    fn the_pattern_cache_stays_within_its_byte_bound() {
        let mut ops = ListOps;
        let mut charge = |_: u64| true;
        let mut analysis = AnalysisMatch {
            target: (None, None),
            charge: &mut charge,
            limits: MatchLimits::default(),
        };
        for n in 0..6u8 {
            let mut pattern = vec![b'q'; 1024 * 1024];
            pattern[0] = b'a' + n;
            let answer = regexp_analysis::<ListOps, LiteralEngine>(
                &mut ops,
                &[&pattern, b"z"],
                &mut analysis,
            );
            assert!(matches!(answer, Ok(RegexpResult::Count { count: 0, .. })));
            assert!(pattern_cache_retained_bytes() <= PATTERN_CACHE_BYTES);
        }
        assert!(
            pattern_cache_retained_bytes() >= 3 * 1024 * 1024,
            "the cache keeps what fits"
        );
    }

    fn about(args: &[&[u8]]) -> Result<String, String> {
        let mut ops = ListOps;
        match regexp::<ListOps, LiteralEngine>(&mut ops, args) {
            Ok(RegexpResult::Inline(v)) => Ok(v),
            Ok(RegexpResult::Count { count, .. }) => Ok(count.to_string()),
            Err(RegexError(m)) => Err(String::from_utf8_lossy(&m).into_owned()),
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
            regexp::<ListOps, FlaggyEngine>(&mut ops, &[b"-about", b"(a)"])
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
            Err(RegsubError::Regex(RegexError(m))) => Err(String::from_utf8_lossy(&m).into_owned()),
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
            .map(|RegexError(m)| String::from_utf8_lossy(&m).into_owned());
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
