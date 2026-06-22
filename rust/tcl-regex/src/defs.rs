//! Core constants and type aliases — the Rust analogue of `regex.h` and the
//! top of `regguts.h`. Faithful to Tcl 9's `generic/regcustom.h` parameters
//! (`chr` = 32-bit codepoint, `CHR_MAX = 0x10FFFF`, `color` = a small signed
//! integer, `DUPMAX = 255`).

/// A character: a Unicode codepoint. (`Tcl_UniChar` / `chr` in the C engine,
/// built with `CHRBITS == 32`.)
pub type Chr = u32;

/// What a `Chr` promotes to in arithmetic (`pchr`). We use `i64` so range
/// arithmetic such as `c + 1` past `CHR_MAX` cannot wrap.
pub type Pchr = i64;

/// A colour: an equivalence class of characters. The C engine uses `short`
/// with `COLORLESS == -1` and `WHITE == 0`; we widen to `i32` to index arenas
/// comfortably while preserving the sentinels.
pub type Color = i32;

/// `celt` — holds a `Chr`, or `NOCELT`.
pub type Celt = i64;

pub const CHR_MIN: Chr = 0x0000_0000;
pub const CHR_MAX: Chr = 0x0010_FFFF;
pub const NOCELT: Celt = -1;

pub const COLORLESS: Color = -1;
pub const WHITE: Color = 0;
/// Max colour value (`SHRT_MAX` upstream — we keep the same ceiling so the
/// `REG_ECOLORS` "too many colors" path triggers identically).
pub const MAX_COLOR: Color = 32767;

/// `_POSIX2_RE_DUP_MAX` — the largest bound in a `{m,n}` quantifier.
pub const DUPMAX: i32 = 255;
/// One past `DUPMAX`; the engine's "infinity" sentinel for `max`.
pub const DUPINF: i32 = DUPMAX + 1;

// -- compile flags (cflags) — regex.h ----------------------------------------

pub const REG_BASIC: i32 = 0o000000; // BREs
pub const REG_EXTENDED: i32 = 0o000001; // EREs
pub const REG_ADVF: i32 = 0o000002; // advanced features in EREs
pub const REG_ADVANCED: i32 = 0o000003; // AREs (= EXTENDED|ADVF)
pub const REG_QUOTE: i32 = 0o000004; // no special characters
pub const REG_NOSPEC: i32 = REG_QUOTE;
pub const REG_ICASE: i32 = 0o000010; // ignore case
pub const REG_NOSUB: i32 = 0o000020; // don't care about subexpressions
pub const REG_EXPANDED: i32 = 0o000040; // expanded format, whitespace & comments
pub const REG_NLSTOP: i32 = 0o000100; // \n doesn't match . or [^ ]
pub const REG_NLANCH: i32 = 0o000200; // ^ matches after \n, $ before
pub const REG_NEWLINE: i32 = 0o000300; // newlines are line terminators
pub const REG_PEND: i32 = 0o000400; // backward-compatibility hack
pub const REG_EXPECT: i32 = 0o001000; // report details on partial/limited matches
pub const REG_BOSONLY: i32 = 0o002000; // temporary kludge for BOS-only matches
pub const REG_DUMP: i32 = 0o004000;
pub const REG_FAKE: i32 = 0o010000;
pub const REG_PROGRESS: i32 = 0o020000;

// -- execution flags (eflags) — regex.h --------------------------------------

pub const REG_NOTBOL: i32 = 0o0001; // BOS is not BOL
pub const REG_NOTEOL: i32 = 0o0002; // EOS is not EOL
pub const REG_STARTEND: i32 = 0o0004;
pub const REG_FTRACE: i32 = 0o0010;
pub const REG_MTRACE: i32 = 0o0020;
pub const REG_SMALL: i32 = 0o0040;

// -- re_info bits — regex.h --------------------------------------------------

pub const REG_UBACKREF: i32 = 0o000001;
pub const REG_ULOOKAHEAD: i32 = 0o000002;
pub const REG_UBOUNDS: i32 = 0o000004;
pub const REG_UBRACES: i32 = 0o000010;
pub const REG_UBSALNUM: i32 = 0o000020;
pub const REG_UPBOTCH: i32 = 0o000040;
pub const REG_UBBS: i32 = 0o000100;
pub const REG_UNONPOSIX: i32 = 0o000200;
pub const REG_UUNSPEC: i32 = 0o000400;
pub const REG_UUNPORT: i32 = 0o001000;
pub const REG_ULOCALE: i32 = 0o002000;
pub const REG_UEMPTYMATCH: i32 = 0o004000;
pub const REG_UIMPOSSIBLE: i32 = 0o010000;
pub const REG_USHORTEST: i32 = 0o020000;

/// Error codes (`regex.h`). `REG_OKAY` is the success value; the rest are the
/// `REG_*` compile / match failures whose *names* `reg.test` checks against
/// `errorCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Err {
    Okay = 0,
    Nomatch = 1,
    Badpat = 2,
    Ecollate = 3,
    Ectype = 4,
    Eescape = 5,
    Esubreg = 6,
    Ebrack = 7,
    Eparen = 8,
    Ebrace = 9,
    Badbr = 10,
    Erange = 11,
    Espace = 12,
    Badrpt = 13,
    Assert = 15,
    Invarg = 16,
    Mixed = 17,
    Badopt = 18,
    Etoobig = 19,
    Ecolors = 20,
}

impl Err {
    /// The bare error name as it appears in Tcl's `errorCode` — e.g.
    /// `REG_BADRPT` is reported as `BADRPT` (reg.test strips `REG_`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Err::Okay => "OKAY",
            Err::Nomatch => "NOMATCH",
            Err::Badpat => "BADPAT",
            Err::Ecollate => "ECOLLATE",
            Err::Ectype => "ECTYPE",
            Err::Eescape => "EESCAPE",
            Err::Esubreg => "ESUBREG",
            Err::Ebrack => "EBRACK",
            Err::Eparen => "EPAREN",
            Err::Ebrace => "EBRACE",
            Err::Badbr => "BADBR",
            Err::Erange => "ERANGE",
            Err::Espace => "ESPACE",
            Err::Badrpt => "BADRPT",
            Err::Assert => "ASSERT",
            Err::Invarg => "INVARG",
            Err::Mixed => "MIXED",
            Err::Badopt => "BADOPT",
            Err::Etoobig => "ETOOBIG",
            Err::Ecolors => "ECOLORS",
        }
    }

    /// The human-readable message (`regerror.c` / `regerrs.h`), used as the
    /// detail in `cannot compile regular expression pattern: <msg>`.
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Err::Okay => "no errors detected",
            Err::Nomatch => "failed to match",
            Err::Badpat => "invalid regular expression",
            Err::Ecollate => "invalid collating element",
            Err::Ectype => "invalid character class",
            Err::Eescape => "invalid escape \\ sequence",
            Err::Esubreg => "invalid backreference number",
            Err::Ebrack => "brackets [] not balanced",
            Err::Eparen => "parentheses () not balanced",
            Err::Ebrace => "braces {} not balanced",
            Err::Badbr => "invalid repetition count(s)",
            Err::Erange => "invalid character range",
            Err::Espace => "out of memory",
            Err::Badrpt => "invalid quantifier operand",
            Err::Assert => "\"can't happen\" -- you found a bug",
            Err::Invarg => "invalid argument to regex routine",
            Err::Mixed => "character widths of regex and string differ",
            Err::Badopt => "invalid embedded option",
            Err::Etoobig => "nfa has too many states",
            Err::Ecolors => "too many colors",
        }
    }
}

/// One reported (sub-)match: half-open `[so, eo)` in **character** offsets.
/// `so == NO_MATCH` means the subexpression did not participate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegMatch {
    pub so: usize,
    pub eo: usize,
}

/// `(size_t)-1` — the engine's "did not participate" sentinel.
pub const NO_MATCH: usize = usize::MAX;
