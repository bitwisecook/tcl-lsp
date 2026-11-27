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

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;

use tcl_dialect::{ByteStringEncoding, StringCharacterModel, TclVersion};

use crate::index::NativeIndexOps;

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

/// A subexpression that did not participate in the match.
const UNMATCHED: RegMatch = RegMatch {
    so: NO_MATCH,
    eo: NO_MATCH,
};

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
                    .chain(groups.iter().map(|group| group.unwrap_or(UNMATCHED)))
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
                return RegexError::new(format!("{option} is not yet supported").into_bytes());
            }
            Self::FuelExhausted { .. } => "the search exceeded its work budget",
            Self::DepthExhausted { .. } => "the search exceeded its recursion limit",
            Self::ApproximateCapture { .. } => "a subexpression's span is only approximate",
            Self::Cancelled => "operation cancelled",
        };
        let mut message = b"error while matching regular expression: ".to_vec();
        message.extend_from_slice(reason.as_bytes());
        RegexError::new(message)
    }
}

pub(crate) fn completed_matches(
    answer: RegexpPrecision<RegMatch>,
) -> Result<Option<Vec<RegMatch>>, RegexError> {
    match answer {
        RegexpPrecision::Declined(decline) => Err(decline.into_error()),
        RegexpPrecision::Exact {
            captures_exact: false,
            ..
        } => Err(PrecisionDecline::ApproximateCapture { group: 0 }.into_error()),
        answer => Ok(answer.match_vector()),
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
    /// `\z` is an end-of-string anchor ([`TclVersion::regex_z_anchor`]).
    pub z_anchor: bool,
    /// Native LSEARCH suppresses subexpression storage on its first compilation.
    pub nosub: bool,
}

impl RegexFlags {
    /// Apply one canonical option from an already selected command table.
    /// Occurrence, availability, abbreviation and value widths belong to the
    /// caller's option grammar. A non-compilation option leaves the flags alone.
    pub fn apply_compile_option(&mut self, name: &str) -> bool {
        match name {
            "-expanded" => self.expanded = true,
            "-nocase" => self.nocase = true,
            "-line" => {
                self.linestop = true;
                self.lineanchor = true;
            }
            "-linestop" => self.linestop = true,
            "-lineanchor" => self.lineanchor = true,
            _ => return false,
        }
        true
    }

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
    /// after character `offset`, matching as C Tcl does against the subject
    /// from `offset` on: `\A` and word-boundary context start at `offset`, and
    /// `notbol` (`REG_NOTBOL`) stops `^` matching there. The answer is
    /// three-way: the match in **absolute** character offsets, a completed
    /// no-match, or why the search established neither — which is never a
    /// no-match.
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

/// A regex command failure retaining its full portable command-error receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegexError(crate::CmdError);

impl RegexError {
    /// Neutral regex result bytes, without an inferred semantic error identity.
    #[must_use]
    pub fn new(message: impl Into<Vec<u8>>) -> Self {
        Self(crate::CmdError::new_bytes(message))
    }

    /// A failure carrying its `-errorcode` list.
    #[must_use]
    pub fn coded(message: impl Into<Vec<u8>>, code: impl Into<String>) -> Self {
        Self(crate::CmdError::with_error_code_bytes(
            message.into(),
            code.into().into_bytes(),
        ))
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

/// The `REG_*` name `regerror` gives each compile failure (`regerrs.h`),
/// keyed by its message.
const REG_ERROR_NAMES: &[(&str, &str)] = &[
    ("invalid regexp (reg version 0.8)", "REG_BADPAT"),
    ("invalid collating element", "REG_ECOLLATE"),
    ("invalid character class", "REG_ECTYPE"),
    ("invalid escape \\ sequence", "REG_EESCAPE"),
    ("invalid backreference number", "REG_ESUBREG"),
    ("brackets [] not balanced", "REG_EBRACK"),
    ("parentheses () not balanced", "REG_EPAREN"),
    ("braces {} not balanced", "REG_EBRACE"),
    ("invalid repetition count(s)", "REG_BADBR"),
    ("invalid character range", "REG_ERANGE"),
    ("out of memory", "REG_ESPACE"),
    ("invalid quantifier operand", "REG_BADRPT"),
    ("\"cannot happen\" -- you found a bug", "REG_ASSERT"),
    ("invalid argument to regex function", "REG_INVARG"),
    ("character widths of regex and string differ", "REG_MIXED"),
    ("invalid embedded option", "REG_BADOPT"),
    ("regular expression is too complex", "REG_ETOOBIG"),
    ("too many colors", "REG_ECOLORS"),
];

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

fn discard_regexp_result<O: NativeIndexOps>(ops: &mut O, result: RegexpResult<O::Value>) {
    match result {
        RegexpResult::Inline(created) => ops.discard_owned_index_result(created),
        RegexpResult::Count {
            assign: Some(assignments),
            ..
        } => {
            for (_, created) in assignments {
                ops.discard_owned_index_result(created);
            }
        }
        RegexpResult::Count { assign: None, .. } => {}
    }
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
    /// All compile flags, including original no-capture compilation, packed for
    /// the pattern cache key.
    #[must_use]
    pub const fn packed(self) -> u8 {
        (self.nocase as u8)
            | ((self.expanded as u8) << 1)
            | ((self.linestop as u8) << 2)
            | ((self.lineanchor as u8) << 3)
            | ((self.z_anchor as u8) << 4)
            | ((self.nosub as u8) << 5)
    }
}

/// The compiled-pattern cache key (`docs/design/compiler/value-evaluation.md`
/// § *The pattern cache, its bound, and the cancellation point*). Every field
/// is part of the identity a compiled pattern is only valid under.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PatternCacheKey {
    /// The pattern's exact bytes, never a normalised or trimmed form.
    pub pattern: Rc<[u8]>,
    /// Every [`RegexFlags`] compile flag, packed ([`RegexFlags::packed`]).
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

mod jim;
mod original_pattern;
mod regexp_command;
mod regsub_command;
mod regsub_original;
mod replacement;

pub use jim::regsub_jim_original;
pub use original_pattern::{
    NativeRegexObjects, NativeRegexSource, PreparedOriginalRegex, compiled_match_original,
    execute_pattern_original, match_pattern_original, prepare_pattern_original,
    prepare_search_pattern_original,
};
pub use regexp_command::{RegexpExecutionError, regexp, regexp_analysis, regexp_original};
pub use regsub_command::{
    RegsubError, regsub, regsub_analysis, regsub_eval, regsub_eval_at, regsub_start,
};
pub use regsub_original::{
    NativeRegsubObjects, NativeRegsubOutcome, NativeRegsubResult, OriginalRegexConsumerError,
    OriginalRegsubPreparation, OriginalRegsubResult, regsub_command_original, regsub_eval_original,
    regsub_native_original, regsub_prepare_original,
};
pub use replacement::{ReplacementPart, replacement_parts};

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
    pub(crate) fn into_error(self) -> RegexError {
        match self {
            Self::Error(error) => error,
            Self::Declined(decline) => decline.into_error(),
        }
    }
}

/// How one `regexp` / `regsub` / `switch -regexp` run compiles its pattern
/// and matches.
pub(crate) enum Run<'a, 'c> {
    /// The runtime: a fresh compile, the engine's own budget.
    Runtime,
    /// The analysis path: the thread's bounded cache, the caller's charge
    /// and limits.
    Analysis(&'a mut AnalysisMatch<'c>),
}

impl Run<'_, '_> {
    /// Compile `pattern` under `flags`, a failure worded as `version` words
    /// it. On the analysis path the thread's
    /// cache answers first; a miss charges the pattern's length squared —
    /// the parser's worst case — before it compiles, and a refused charge is
    /// `Cancelled`, never a partial entry.
    pub(crate) fn compile<E: RegexEngine>(
        &mut self,
        pattern: &[u8],
        flags: RegexFlags,
        version: TclVersion,
    ) -> Result<Rc<RefCell<E::Regex>>, RegexFailure>
    where
        E::Regex: 'static,
    {
        let Self::Analysis(analysis) = self else {
            let re =
                E::compile(pattern, flags).map_err(|detail| compile_error(version, &detail))?;
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
            RegexFailure::Declined(PrecisionDecline::PatternError(compile_error(
                version, &detail,
            )))
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
    pub(crate) fn exec<E: RegexEngine>(
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
/// C hands `TclGetIntForIndexM` the string length as the value of `end`, so
/// `end` is one past the last character, not the last character.
///
/// `N` in `end±N` is a user-supplied integer, so `len - 1 ± N` is done with
/// `saturating_*`: `end+9999999999999999999` would otherwise overflow `isize`
/// and wrap to a bogus (possibly in-range) offset. Saturating pins it to the
/// `isize` extremes — a too-large `+N` becomes "past the end" (the match loop's
/// `offset >= char_len` check then yields no match) and a too-large `-N` becomes
/// the start, both the intended clamp behaviour.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
fn resolve_start_checked<O: NativeIndexOps>(
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
        crate::index::resolve_in(text, end_basis(char_len), syntax)
    } else {
        crate::index::resolve_for_ops(ops, text, end_basis(char_len))
    }
    .map_err(RegexError::from)?;
    Ok(usize::try_from(idx).unwrap_or(0))
}

/// Resolve the original `-start` word as C's `TclGetIntForIndexM` reads it:
/// the same object, so a bignum offset saturates under 9.x as `lindex` does.
fn resolve_start_original<O: NativeIndexOps>(
    ops: &mut O,
    args: &[O::Value],
    common: &Common,
    char_len: usize,
) -> Result<usize, RegexError> {
    let (Some(spec), Some(at)) = (common.start.as_deref(), common.start_at) else {
        return Ok(0);
    };
    let syntax = ops
        .index_syntax()
        .ok_or_else(|| RegexError::new(b"regex offset dialect is not selected".to_vec()))?;
    if syntax.regex_start_grammar() == tcl_dialect::RegexStartGrammar::Integer {
        return resolve_start_checked(ops, spec, char_len);
    }
    let idx = crate::index::resolve_value(ops, &args[at], end_basis(char_len))
        .map_err(RegexError::from)?;
    Ok(usize::try_from(idx).unwrap_or(0))
}

/// The length whose `end` is `char_len`, as `-start` reads it.
const fn end_basis(char_len: usize) -> usize {
    char_len.saturating_add(1)
}

/// The compile flags + `-all`/`-start` shared by both commands' option sets.
struct Common {
    all: bool,
    flags: RegexFlags,
    start: Option<Vec<u8>>,
    /// The `-start` word's position among the command's arguments.
    start_at: Option<usize>,
}

impl Common {
    const fn for_release(version: TclVersion) -> Self {
        Self {
            all: false,
            flags: RegexFlags::for_release(version),
            start: None,
            start_at: None,
        }
    }
}

fn wrong_args(usage: &[u8]) -> RegexError {
    crate::CmdError::wrong_args_bytes(usage).into()
}

/// Wrap a provider compile-error detail in `version`'s standard prefix, with
/// `TclRegError`'s `REGEXP <name> <detail>` code when the detail is one of
/// `regerror`'s.
#[must_use]
pub fn compile_error(version: TclVersion, detail: &[u8]) -> RegexError {
    let mut m = version.regex_compile_error_prefix().as_bytes().to_vec();
    m.extend_from_slice(detail);
    let text = String::from_utf8_lossy(detail);
    match REG_ERROR_NAMES.iter().find(|(message, _)| *message == text) {
        Some((_, name)) => {
            RegexError::coded(m, tcl_syntax::list::join_list(["REGEXP", name, &text]))
        }
        None => RegexError::new(m),
    }
}

#[cfg(test)]
mod tests;
