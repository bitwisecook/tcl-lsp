//! A pure-Rust port of Tcl 9's Henry-Spencer Advanced Regular Expression (ARE)
//! engine. The goal is behavioural fidelity with `tclsh` 9.0 — the same match
//! positions, submatch participation, [`Regex::nsub`], [`Regex::info`] bits,
//! and [`ErrorCode`] compile errors — verified against the engine itself
//! through the `reg.test` corpus (see `tests/`).
//!
//! The implementation is idiomatic Rust, not a transliteration of the C: the
//! pattern compiles to a typed [`ast::Node`] tree ([`parser`]) and matches via
//! a reachable-set simulator plus a submatch dissector ([`exec`]). See those
//! modules for the algorithmic mapping back to `regcomp.c` / `regexec.c`.

// Codepoints and octal flag-bit constants are clearer without digit
// separators; the engine carries codepoints as i64 internally but every
// value is provably within u32 range, so the codepoint casts are sound.
#![allow(
    clippy::unreadable_literal,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]

mod ast;
pub mod defs;
mod exec;
mod parser;

pub use defs::Err as ErrorCode;
pub use exec::Span;

use ast::Node;

/// One of the `re_info` bits the engine reports about a compiled pattern
/// (`testregexp -about`). The discriminants are the Tcl `REG_U*` bit values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InfoFlag {
    /// `REG_UBACKREF` — a back reference was used.
    Backref = defs::REG_UBACKREF,
    /// `REG_ULOOKAHEAD` — a lookahead constraint was used.
    Lookahead = defs::REG_ULOOKAHEAD,
    /// `REG_UBOUNDS` — a `{m,n}` bound was used.
    Bounds = defs::REG_UBOUNDS,
    /// `REG_UBRACES` — a literal `{` was treated as ordinary.
    Braces = defs::REG_UBRACES,
    /// `REG_UBSALNUM` — a `\<alphanumeric>` escape was seen.
    BsAlnum = defs::REG_UBSALNUM,
    /// `REG_UPBOTCH` — the POSIX unmatched-`)` botch was exercised.
    PBotch = defs::REG_UPBOTCH,
    /// `REG_UBBS` — a backslash was seen inside brackets.
    Bbs = defs::REG_UBBS,
    /// `REG_UNONPOSIX` — a non-POSIX construct was used.
    NonPosix = defs::REG_UNONPOSIX,
    /// `REG_UUNSPEC` — POSIX-unspecified syntax was used.
    Unspec = defs::REG_UUNSPEC,
    /// `REG_UUNPORT` — an unportable (machine-specific) construct was used.
    Unport = defs::REG_UUNPORT,
    /// `REG_ULOCALE` — a locale-dependent construct was used.
    Locale = defs::REG_ULOCALE,
    /// `REG_UEMPTYMATCH` — the RE can match the empty string.
    EmptyMatch = defs::REG_UEMPTYMATCH,
    /// `REG_UIMPOSSIBLE` — the RE cannot match anything.
    Impossible = defs::REG_UIMPOSSIBLE,
    /// `REG_USHORTEST` — the RE prefers the shortest match.
    Shortest = defs::REG_USHORTEST,
}

impl InfoFlag {
    /// All flags in `re_info` bit order (LSB first) — the order
    /// `testregexp -about` reports them.
    pub const ALL: [InfoFlag; 14] = [
        InfoFlag::Backref,
        InfoFlag::Lookahead,
        InfoFlag::Bounds,
        InfoFlag::Braces,
        InfoFlag::BsAlnum,
        InfoFlag::PBotch,
        InfoFlag::Bbs,
        InfoFlag::NonPosix,
        InfoFlag::Unspec,
        InfoFlag::Unport,
        InfoFlag::Locale,
        InfoFlag::EmptyMatch,
        InfoFlag::Impossible,
        InfoFlag::Shortest,
    ];

    /// The Tcl name, e.g. `REG_UNONPOSIX`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            InfoFlag::Backref => "REG_UBACKREF",
            InfoFlag::Lookahead => "REG_ULOOKAHEAD",
            InfoFlag::Bounds => "REG_UBOUNDS",
            InfoFlag::Braces => "REG_UBRACES",
            InfoFlag::BsAlnum => "REG_UBSALNUM",
            InfoFlag::PBotch => "REG_UPBOTCH",
            InfoFlag::Bbs => "REG_UBBS",
            InfoFlag::NonPosix => "REG_UNONPOSIX",
            InfoFlag::Unspec => "REG_UUNSPEC",
            InfoFlag::Unport => "REG_UUNPORT",
            InfoFlag::Locale => "REG_ULOCALE",
            InfoFlag::EmptyMatch => "REG_UEMPTYMATCH",
            InfoFlag::Impossible => "REG_UIMPOSSIBLE",
            InfoFlag::Shortest => "REG_USHORTEST",
        }
    }
}

/// The set of `re_info` bits for a compiled pattern.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Info(i32);

impl Info {
    #[must_use]
    pub fn bits(self) -> i32 {
        self.0
    }
    #[must_use]
    pub fn contains(self, f: InfoFlag) -> bool {
        self.0 & (f as i32) != 0
    }
    /// The flags present, in `re_info` bit order.
    #[must_use]
    pub fn flags(self) -> Vec<InfoFlag> {
        InfoFlag::ALL
            .iter()
            .copied()
            .filter(|f| self.contains(*f))
            .collect()
    }
}

/// A compiled regular expression.
pub struct Regex {
    root: Node,
    nsub: usize,
    info: i32,
    cflags: i32,
    has_backref: bool,
}

impl Regex {
    /// Compile `pattern` (codepoints) under the engine compile flags `cflags`
    /// (`defs::REG_*`). Use [`defs::REG_ADVANCED`] for the Tcl default ARE.
    ///
    /// # Errors
    /// The first [`ErrorCode`] encountered, matching Tcl's behaviour.
    pub fn compile(pattern: &[u32], cflags: i32) -> Result<Regex, ErrorCode> {
        let c = parser::compile(pattern, cflags)?;
        let has_backref = contains_backref(&c.root);
        Ok(Regex {
            root: c.root,
            nsub: c.nsub,
            info: c.info,
            cflags: c.cflags,
            has_backref,
        })
    }

    /// Compile from a `&str` pattern.
    ///
    /// # Errors
    /// As [`Regex::compile`].
    pub fn compile_str(pattern: &str, cflags: i32) -> Result<Regex, ErrorCode> {
        let cps: Vec<u32> = pattern.chars().map(|c| c as u32).collect();
        Regex::compile(&cps, cflags)
    }

    /// The number of capturing subexpressions (`re_nsub`).
    #[must_use]
    pub fn nsub(&self) -> usize {
        self.nsub
    }

    /// The `re_info` bits.
    #[must_use]
    pub fn info(&self) -> Info {
        Info(self.info)
    }

    /// Match against `subject` (codepoints), searching from character `from`,
    /// under exec flags `eflags` (`REG_NOTBOL` / `REG_NOTEOL`). Returns the
    /// capture spans (index 0 = whole match; `None` = non-participating
    /// subexpression), or `None` if there is no match.
    #[must_use]
    pub fn exec(&self, subject: &[u32], from: usize, eflags: i32) -> Option<Vec<Option<Span>>> {
        let prefer_shortest = self.info & defs::REG_USHORTEST != 0;
        let mut m = exec::Matcher::new(subject, self.cflags, eflags, prefer_shortest);
        if self.has_backref {
            m.search_backref(&self.root, self.nsub, from)
        } else {
            m.search(&self.root, self.nsub, from)
        }
    }
}

fn contains_backref(node: &Node) -> bool {
    match node {
        Node::Backref { .. } => true,
        Node::Empty | Node::Set(_) | Node::Anchor(_) => false,
        Node::Look { sub, .. } | Node::Capture { sub, .. } | Node::Repeat { sub, .. } => {
            contains_backref(sub)
        }
        Node::Concat(items) | Node::Alt(items) => items.iter().any(contains_backref),
    }
}
