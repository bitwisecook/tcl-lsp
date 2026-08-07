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

//! Lexer + recursive-descent parser, an idiomatic re-expression of the C
//! engine's `regc_lex.c` + the `parse`/`parsebranch`/`parseqatom` machinery in
//! `regcomp.c`. It consumes a `&[Chr]` and produces a [`Node`] AST, while
//! accumulating the `re_info` bits and the subexpression count, and reporting
//! the exact `REG_*` compile errors Tcl does.
//!
//! Fidelity targets (verified against `tests/data/reg.test`): the same token
//! decisions (BRE vs ERE/ARE, embedded options, `***=`/`***:` prefixes), the
//! same `NOTE(REG_U*)` info bits, the same backreference-vs-octal heuristic,
//! the same `BADRPT`/`BADBR`/`EBRACE`/`EPAREN`/`EBRACK`/`ESUBREG`/`EESCAPE`
//! error points, and the same capture numbering.

use crate::ast::{Anchor, CharSet, ClassKind, Node, Pref};
use crate::defs::{
    Chr, DUPINF, DUPMAX, Err, REG_ADVANCED, REG_ADVF, REG_EXPANDED, REG_EXTENDED, REG_ICASE,
    REG_NEWLINE, REG_NLANCH, REG_NLSTOP, REG_NOSUB, REG_QUOTE, REG_UBACKREF, REG_UBBS, REG_UBOUNDS,
    REG_UBRACES, REG_UBSALNUM, REG_ULOCALE, REG_ULOOKAHEAD, REG_UNONPOSIX, REG_UPBOTCH,
    REG_UUNPORT, REG_UUNSPEC,
};

/// Lexical token kinds, mirroring the C `nexttype` codes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tok {
    Empty,
    Eos,
    Plain,
    Digit,
    Backref,
    Collel,
    Eclass,
    Cclass,
    End,
    Range,
    /// `\d \D \s \S \w \W` shorthand; value holds the ASCII letter.
    Shorthand,
    Lacon,
    Wbdry,
    NWbdry,
    Sbegin,
    Send,
    Caret,
    Dollar,
    Lt,
    Gt,
    OpenParen,
    CloseParen,
    OpenBrack,
    CloseBrack,
    Dot,
    Bar,
    Star,
    Plus,
    Quest,
    OpenBrace,
    CloseBrace,
    Comma,
}

/// Lexical contexts (`v->lexcon`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Con {
    Ere,
    Bre,
    Quote,
    Ebnd,
    Bbnd,
    Brack,
    Cel,
    Ecl,
    Ccl,
}

pub struct Compiled {
    pub root: Node,
    pub nsub: usize,
    pub info: i32,
    /// The compile flags *after* prefix/embedded-option processing — what the
    /// matcher must use for line-anchoring etc.
    pub cflags: i32,
}

pub struct Parser<'a> {
    input: &'a [Chr],
    now: usize,
    cflags: i32,
    info: i32,
    lexcon: Con,
    nexttype: Tok,
    nextvalue: i64,
    lasttype: Tok,
    nsubexp: usize,
    /// `subs_defined[n]` is set once capture group `n` has been fully parsed
    /// (the backref-validity gate from `parseqatom`'s `BACKREF` case).
    subs_defined: Vec<bool>,
    /// Current recursive-descent nesting depth. Bumped on entry to [`parse`]
    /// and dropped on exit; guards against stack overflow from pathologically
    /// nested groups (see [`MAX_PARSE_DEPTH`]).
    depth: usize,
    err: Option<Err>,
}

/// Recursion budget for the `parse` → `parsebranch` → `parseqatom` → `parse`
/// cycle (one extra level per `(`). A hostile pattern such as `(`×4000 would
/// otherwise drive the recursive descent into a stack overflow (SIGABRT) before
/// any error could be reported. Real patterns never nest groups this deeply, so
/// capping at 1000 — comfortably within an 8 MB stack at this frame size — turns
/// the abort into a clean `REG_ETOOBIG` compile error.
const MAX_PARSE_DEPTH: usize = 1000;

/// Compile `input` (codepoints) under `cflags`, returning the AST + metadata.
///
/// # Errors
/// The first `REG_*` failure encountered (matching Tcl's behaviour of reporting
/// only the first compile error).
pub fn compile(input: &[Chr], cflags: i32) -> Result<Compiled, Err> {
    // Sanity checks on flag combinations (regcomp.c `compile`).
    if cflags & REG_QUOTE != 0 && cflags & (REG_ADVANCED | REG_EXPANDED | REG_NEWLINE) != 0 {
        return Err(Err::Invarg);
    }
    if cflags & REG_EXTENDED == 0 && cflags & REG_ADVF != 0 {
        return Err(Err::Invarg);
    }
    let mut p = Parser {
        input,
        now: 0,
        cflags,
        info: 0,
        lexcon: Con::Ere,
        nexttype: Tok::Empty,
        nextvalue: 0,
        lasttype: Tok::Empty,
        nsubexp: 0,
        subs_defined: vec![false],
        depth: 0,
        err: None,
    };
    p.lexstart();
    if let Some(e) = p.err {
        return Err(e);
    }
    let root = p.parse(true, false);
    if let Some(e) = p.err {
        return Err(e);
    }
    // Trailing `)` with nothing to close, or other leftover, is caught inside
    // `parse` via the stopper logic. A successful parse leaves us at EOS.
    if p.nexttype != Tok::Eos {
        // Unbalanced close paren at top level.
        return Err(p.err.unwrap_or(Err::Eparen));
    }
    let final_cflags = p.cflags;
    // REG_BOSONLY behaves as if the pattern were anchored with `\A`.
    let mut root = root;
    if final_cflags & crate::defs::REG_BOSONLY != 0 {
        root = Node::Concat(vec![Node::Anchor(Anchor::Bos), root]);
    }
    let mut info = p.info;
    // The whole RE preferring the shortest match (regcomp.c, top-tree SHORTER).
    if crate::ast::leading_pref(&root) == Some(Pref::Shorter) {
        info |= crate::defs::REG_USHORTEST;
    }
    // Structural info bits (regc_nfa.c `analyze`): mutually exclusive — the RE
    // either cannot match at all, or it can match the empty string.
    if is_impossible(&root, final_cflags) {
        info |= crate::defs::REG_UIMPOSSIBLE;
    } else if can_match_empty(&root) {
        info |= crate::defs::REG_UEMPTYMATCH;
    }
    Ok(Compiled {
        root,
        nsub: p.nsubexp,
        info,
        cflags: final_cflags,
    })
}

/// Can `node` match the empty string somewhere (`REG_UEMPTYMATCH`)?
fn can_match_empty(node: &Node) -> bool {
    match node {
        Node::Empty | Node::Anchor(_) | Node::Look { .. } => true,
        Node::Set(_) => false,
        Node::Capture { sub, .. } => can_match_empty(sub),
        Node::Concat(items) => items.iter().all(can_match_empty),
        Node::Alt(branches) => branches.iter().any(can_match_empty),
        Node::Repeat { sub, min, .. } => *min == 0 || can_match_empty(sub),
        Node::Backref { min, .. } => *min == 0,
    }
}

/// Minimum number of characters `node` must consume.
fn min_len(node: &Node) -> usize {
    match node {
        Node::Empty | Node::Anchor(_) | Node::Look { .. } | Node::Backref { .. } => 0,
        Node::Set(_) => 1,
        Node::Capture { sub, .. } => min_len(sub),
        Node::Concat(items) => items.iter().map(min_len).sum(),
        Node::Alt(branches) => branches.iter().map(min_len).min().unwrap_or(0),
        Node::Repeat { sub, min, .. } => (*min as usize) * min_len(sub),
    }
}

/// Detect REs that can never match (`REG_UIMPOSSIBLE`) due to a constraint that
/// no string can satisfy in context — e.g. `x^` or `x$y` outside newline mode.
/// Conservative: only flags clear contradictions (matches the cases `reg.test`
/// exercises); never a false positive.
fn is_impossible(node: &Node, cflags: i32) -> bool {
    let lineanchor = cflags & REG_NLANCH != 0;
    impossible_walk(node, lineanchor)
}

fn impossible_walk(node: &Node, lineanchor: bool) -> bool {
    match node {
        Node::Concat(items) => {
            if concat_impossible(items, lineanchor) {
                return true;
            }
            items.iter().any(|n| impossible_walk(n, lineanchor))
        }
        Node::Alt(branches) => {
            !branches.is_empty() && branches.iter().all(|b| impossible_walk(b, lineanchor))
        }
        Node::Capture { sub, .. } => impossible_walk(sub, lineanchor),
        Node::Repeat { sub, min, .. } => *min >= 1 && impossible_walk(sub, lineanchor),
        _ => false,
    }
}

/// Within a concatenation, a `^`/`\A` preceded by something that must consume a
/// character (no intervening newline possible) can never fire; symmetrically a
/// `$`/`\Z` followed by a mandatory character.
fn concat_impossible(items: &[Node], lineanchor: bool) -> bool {
    for (i, it) in items.iter().enumerate() {
        match it {
            Node::Anchor(Anchor::Bos) => {
                if items[..i].iter().map(min_len).sum::<usize>() > 0 {
                    return true;
                }
            }
            Node::Anchor(Anchor::Bol) => {
                if !lineanchor && items[..i].iter().map(min_len).sum::<usize>() > 0 {
                    return true;
                }
            }
            Node::Anchor(Anchor::Eos) => {
                if items[i + 1..].iter().map(min_len).sum::<usize>() > 0 {
                    return true;
                }
            }
            Node::Anchor(Anchor::Eol) => {
                if !lineanchor && items[i + 1..].iter().map(min_len).sum::<usize>() > 0 {
                    return true;
                }
            }
            // A word-begin needs a non-word char (or BOS) to its left and a word
            // char to its right; a word-end is the mirror. A neighbouring atom
            // that forces the opposite word-ness makes the RE impossible.
            Node::Anchor(Anchor::WordBegin) => {
                if last_char_word(&items[..i]) == Some(true)
                    || first_char_word(&items[i + 1..]) == Some(false)
                {
                    return true;
                }
            }
            Node::Anchor(Anchor::WordEnd)
                if last_char_word(&items[..i]) == Some(false)
                    || first_char_word(&items[i + 1..]) == Some(true) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Is the first character a sequence of nodes must consume known to be a word
/// char (`Some(true)`), a non-word char (`Some(false)`), or undetermined?
fn first_char_word(items: &[Node]) -> Option<bool> {
    items.iter().find_map(node_first_word)
}

/// As [`first_char_word`] but for the last consumed character.
fn last_char_word(items: &[Node]) -> Option<bool> {
    items.iter().rev().find_map(node_last_word)
}

fn node_first_word(node: &Node) -> Option<bool> {
    match node {
        Node::Set(s) => simple_set_word(s),
        Node::Capture { sub, .. } => node_first_word(sub),
        Node::Concat(items) => first_char_word(items),
        Node::Repeat { sub, min, .. } if *min >= 1 => node_first_word(sub),
        _ => None,
    }
}

fn node_last_word(node: &Node) -> Option<bool> {
    match node {
        Node::Set(s) => simple_set_word(s),
        Node::Capture { sub, .. } => node_last_word(sub),
        Node::Concat(items) => last_char_word(items),
        Node::Repeat { sub, min, .. } if *min >= 1 => node_last_word(sub),
        _ => None,
    }
}

/// Word-ness of a single-literal positive set, else undetermined.
fn simple_set_word(set: &crate::ast::CharSet) -> Option<bool> {
    if set.negated || !set.classes.is_empty() || set.ranges.len() != 1 {
        return None;
    }
    let (lo, hi) = set.ranges[0];
    if lo != hi {
        return None;
    }
    Some(is_word_char(lo))
}

fn is_word_char(c: Chr) -> bool {
    char::from_u32(c).is_some_and(|c| c.is_alphanumeric() || c == '_')
}

impl Parser<'_> {
    // low-level scanning helpers (the C ATEOS/HAVE/NEXTn macros)

    fn ateos(&self) -> bool {
        self.now >= self.input.len()
    }
    fn have(&self, n: usize) -> bool {
        self.input.len() - self.now >= n
    }
    fn peek(&self, off: usize) -> Option<Chr> {
        self.input.get(self.now + off).copied()
    }
    fn next1(&self, c: u8) -> bool {
        self.peek(0) == Some(Chr::from(c))
    }
    fn next2(&self, a: u8, b: u8) -> bool {
        self.have(2) && self.next1(a) && self.peek(1) == Some(Chr::from(b))
    }
    fn next3(&self, a: u8, b: u8, c: u8) -> bool {
        self.have(3) && self.next2(a, b) && self.peek(2) == Some(Chr::from(c))
    }

    fn note(&mut self, bit: i32) {
        self.info |= bit;
    }
    fn fail(&mut self, e: Err) {
        if self.err.is_none() {
            self.err = Some(e);
        }
        self.nexttype = Tok::Eos;
    }
    fn iserr(&self) -> bool {
        self.err.is_some()
    }

    fn set(&mut self, t: Tok) {
        self.nexttype = t;
    }
    fn setv(&mut self, t: Tok, v: i64) {
        self.nexttype = t;
        self.nextvalue = v;
    }

    // lexer

    fn lexstart(&mut self) {
        self.prefixes();
        if self.iserr() {
            return;
        }
        if self.cflags & REG_QUOTE != 0 {
            self.lexcon = Con::Quote;
        } else if self.cflags & REG_EXTENDED != 0 {
            self.lexcon = Con::Ere;
        } else {
            self.lexcon = Con::Bre;
        }
        self.nexttype = Tok::Empty;
        self.next();
    }

    fn prefixes(&mut self) {
        if self.cflags & REG_QUOTE != 0 {
            return;
        }
        if self.have(4) && self.next3(b'*', b'*', b'*') {
            match self.peek(3) {
                Some(c) if c == Chr::from(b'?') => {
                    self.fail(Err::Badpat);
                    return;
                }
                Some(c) if c == Chr::from(b'=') => {
                    self.note(REG_UNONPOSIX);
                    self.cflags |= REG_QUOTE;
                    self.cflags &= !(REG_ADVANCED | REG_EXPANDED | REG_NEWLINE);
                    self.now += 4;
                    return;
                }
                Some(c) if c == Chr::from(b':') => {
                    self.note(REG_UNONPOSIX);
                    self.cflags |= REG_ADVANCED;
                    self.now += 4;
                }
                _ => {
                    self.fail(Err::Badrpt);
                    return;
                }
            }
        }
        if self.cflags & REG_ADVANCED != REG_ADVANCED {
            return;
        }
        // Embedded options: (?xyz)
        if self.have(3) && self.next2(b'(', b'?') && self.peek(2).is_some_and(is_alpha) {
            self.note(REG_UNONPOSIX);
            self.now += 2;
            while !self.ateos() && self.peek(0).is_some_and(is_alpha) {
                let c = self.input[self.now];
                self.now += 1;
                match u8::try_from(c).ok() {
                    Some(b'b') => self.cflags &= !(REG_ADVANCED | REG_QUOTE),
                    Some(b'c') => self.cflags &= !REG_ICASE,
                    Some(b'e') => {
                        self.cflags |= REG_EXTENDED;
                        self.cflags &= !(REG_ADVF | REG_QUOTE);
                    }
                    Some(b'i') => self.cflags |= REG_ICASE,
                    Some(b'm' | b'n') => self.cflags |= REG_NEWLINE,
                    Some(b'p') => {
                        self.cflags |= REG_NLSTOP;
                        self.cflags &= !REG_NLANCH;
                    }
                    Some(b'q') => {
                        self.cflags |= REG_QUOTE;
                        self.cflags &= !REG_ADVANCED;
                    }
                    Some(b's') => self.cflags &= !REG_NEWLINE,
                    Some(b't') => self.cflags &= !REG_EXPANDED,
                    Some(b'w') => {
                        self.cflags &= !REG_NLSTOP;
                        self.cflags |= REG_NLANCH;
                    }
                    Some(b'x') => self.cflags |= REG_EXPANDED,
                    _ => {
                        self.fail(Err::Badopt);
                        return;
                    }
                }
            }
            if !self.next1(b')') {
                self.fail(Err::Badopt);
                return;
            }
            self.now += 1;
            if self.cflags & REG_QUOTE != 0 {
                self.cflags &= !(REG_EXPANDED | REG_NEWLINE);
            }
        }
    }

    /// Skip whitespace + `#` comments in expanded mode.
    fn skip(&mut self) {
        let start = self.now;
        loop {
            while !self.ateos() && is_ascii_ws(self.input[self.now]) {
                self.now += 1;
            }
            if self.ateos() || !self.next1(b'#') {
                break;
            }
            while !self.ateos() && self.input[self.now] != Chr::from(b'\n') {
                self.now += 1;
            }
        }
        if self.now != start {
            self.note(REG_UNONPOSIX);
        }
    }

    /// Fetch the next token into `nexttype`/`nextvalue`.
    fn next(&mut self) {
        if self.iserr() {
            return;
        }
        self.lasttype = self.nexttype;

        // Expanded-mode whitespace skipping in mainline / bound contexts.
        if self.cflags & REG_EXPANDED != 0
            && matches!(self.lexcon, Con::Ere | Con::Bre | Con::Ebnd | Con::Bbnd)
        {
            self.skip();
        }

        if self.ateos() {
            match self.lexcon {
                Con::Ere | Con::Bre | Con::Quote => self.set(Tok::Eos),
                Con::Ebnd | Con::Bbnd => self.fail(Err::Ebrace),
                Con::Brack | Con::Cel | Con::Ecl | Con::Ccl => self.fail(Err::Ebrack),
            }
            return;
        }

        let c = self.input[self.now];
        self.now += 1;

        match self.lexcon {
            Con::Bre => self.brenext(c),
            Con::Quote => self.setv(Tok::Plain, i64::from(c)),
            Con::Ebnd | Con::Bbnd => self.bound_next(c),
            Con::Brack => self.brack_next(c),
            Con::Cel => {
                if c == Chr::from(b'.') && self.next1(b']') {
                    self.now += 1;
                    self.lexcon = Con::Brack;
                    self.setv(Tok::End, i64::from(b'.'));
                } else {
                    self.setv(Tok::Plain, i64::from(c));
                }
            }
            Con::Ecl => {
                if c == Chr::from(b'=') && self.next1(b']') {
                    self.now += 1;
                    self.lexcon = Con::Brack;
                    self.setv(Tok::End, i64::from(b'='));
                } else {
                    self.setv(Tok::Plain, i64::from(c));
                }
            }
            Con::Ccl => {
                if c == Chr::from(b':') && self.next1(b']') {
                    self.now += 1;
                    self.lexcon = Con::Brack;
                    self.setv(Tok::End, i64::from(b':'));
                } else {
                    self.setv(Tok::Plain, i64::from(c));
                }
            }
            Con::Ere => self.erenext(c),
        }
    }

    fn bound_next(&mut self, c: Chr) {
        if let Some(d) = digit_val(c) {
            self.setv(Tok::Digit, i64::from(d));
            return;
        }
        if c == Chr::from(b',') {
            self.set(Tok::Comma);
            return;
        }
        if c == Chr::from(b'}') {
            if self.lexcon == Con::Ebnd {
                self.lexcon = Con::Ere;
                if self.cflags & REG_ADVF != 0 && self.next1(b'?') {
                    self.now += 1;
                    self.note(REG_UNONPOSIX);
                    self.setv(Tok::CloseBrace, 0);
                    return;
                }
                self.setv(Tok::CloseBrace, 1);
                return;
            }
            self.fail(Err::Badbr);
            return;
        }
        if c == Chr::from(b'\\') && self.lexcon == Con::Bbnd && self.next1(b'}') {
            self.now += 1;
            self.lexcon = Con::Bre;
            self.setv(Tok::CloseBrace, 1);
            return;
        }
        self.fail(Err::Badbr);
    }

    fn brack_next(&mut self, c: Chr) {
        if c == Chr::from(b']') {
            if self.lasttype == Tok::OpenBrack {
                self.setv(Tok::Plain, i64::from(c));
            } else {
                self.lexcon = if self.cflags & REG_EXTENDED != 0 {
                    Con::Ere
                } else {
                    Con::Bre
                };
                self.set(Tok::CloseBrack);
            }
            return;
        }
        if c == Chr::from(b'\\') {
            self.note(REG_UBBS);
            if self.cflags & REG_ADVF == 0 {
                self.setv(Tok::Plain, i64::from(c));
                return;
            }
            self.note(REG_UNONPOSIX);
            if self.ateos() {
                self.fail(Err::Eescape);
                return;
            }
            self.lexescape();
            match self.nexttype {
                Tok::Plain => {}
                Tok::Shorthand => match u8::try_from(self.nextvalue).ok() {
                    // Only d/s/w (positive classes) are valid inside [].
                    Some(b'd' | b's' | b'w') => {}
                    _ => self.fail(Err::Eescape),
                },
                _ => self.fail(Err::Eescape),
            }
            return;
        }
        if c == Chr::from(b'-') {
            if self.lasttype == Tok::OpenBrack || self.next1(b']') {
                self.setv(Tok::Plain, i64::from(c));
            } else {
                self.setv(Tok::Range, i64::from(c));
            }
            return;
        }
        if c == Chr::from(b'[') {
            if self.ateos() {
                self.fail(Err::Ebrack);
                return;
            }
            let nc = self.input[self.now];
            self.now += 1;
            match u8::try_from(nc).ok() {
                Some(b'.') => {
                    self.lexcon = Con::Cel;
                    self.set(Tok::Collel);
                }
                Some(b'=') => {
                    self.lexcon = Con::Ecl;
                    self.note(REG_ULOCALE);
                    self.set(Tok::Eclass);
                }
                Some(b':') => {
                    self.lexcon = Con::Ccl;
                    self.note(REG_ULOCALE);
                    self.set(Tok::Cclass);
                }
                _ => {
                    self.now -= 1;
                    self.setv(Tok::Plain, i64::from(c));
                }
            }
            return;
        }
        self.setv(Tok::Plain, i64::from(c));
    }

    fn erenext(&mut self, mut c: Chr) {
        match u8::try_from(c).ok() {
            Some(b'|') => {
                self.set(Tok::Bar);
                return;
            }
            Some(b'*') => {
                let g = self.adv_quant_q();
                self.setv(Tok::Star, g);
                return;
            }
            Some(b'+') => {
                let g = self.adv_quant_q();
                self.setv(Tok::Plus, g);
                return;
            }
            Some(b'?') => {
                let g = self.adv_quant_q();
                self.setv(Tok::Quest, g);
                return;
            }
            Some(b'{') => {
                if self.cflags & REG_EXPANDED != 0 {
                    self.skip();
                }
                if self.ateos() || !self.peek(0).is_some_and(is_digit) {
                    self.note(REG_UBRACES);
                    self.note(REG_UUNSPEC);
                    self.setv(Tok::Plain, i64::from(c));
                } else {
                    self.note(REG_UBOUNDS);
                    self.lexcon = Con::Ebnd;
                    self.set(Tok::OpenBrace);
                }
                return;
            }
            Some(b'(') => {
                if self.cflags & REG_ADVF != 0 && self.next1(b'?') {
                    self.note(REG_UNONPOSIX);
                    self.now += 1;
                    let k = self.input.get(self.now).copied();
                    self.now += 1;
                    match k.and_then(|x| u8::try_from(x).ok()) {
                        Some(b':') => {
                            self.setv(Tok::OpenParen, 0);
                            return;
                        }
                        Some(b'#') => {
                            while !self.ateos() && !self.next1(b')') {
                                self.now += 1;
                            }
                            if !self.ateos() {
                                self.now += 1;
                            }
                            self.next();
                            return;
                        }
                        Some(b'=') => {
                            self.note(REG_ULOOKAHEAD);
                            self.setv(Tok::Lacon, 1);
                            return;
                        }
                        Some(b'!') => {
                            self.note(REG_ULOOKAHEAD);
                            self.setv(Tok::Lacon, 0);
                            return;
                        }
                        _ => {
                            self.fail(Err::Badrpt);
                            return;
                        }
                    }
                }
                if self.cflags & REG_NOSUB != 0 {
                    self.setv(Tok::OpenParen, 0);
                } else {
                    self.setv(Tok::OpenParen, 1);
                }
                return;
            }
            Some(b')') => {
                if self.lasttype == Tok::OpenParen {
                    self.note(REG_UUNSPEC);
                }
                self.setv(Tok::CloseParen, i64::from(c));
                return;
            }
            Some(b'[') => {
                if self.have(6)
                    && self.peek(0) == Some(Chr::from(b'['))
                    && self.peek(1) == Some(Chr::from(b':'))
                    && (self.peek(2) == Some(Chr::from(b'<'))
                        || self.peek(2) == Some(Chr::from(b'>')))
                    && self.peek(3) == Some(Chr::from(b':'))
                    && self.peek(4) == Some(Chr::from(b']'))
                    && self.peek(5) == Some(Chr::from(b']'))
                {
                    let which = self.peek(2).unwrap();
                    self.now += 6;
                    self.note(REG_UNONPOSIX);
                    if which == Chr::from(b'<') {
                        self.set(Tok::Lt);
                    } else {
                        self.set(Tok::Gt);
                    }
                    return;
                }
                self.lexcon = Con::Brack;
                if self.next1(b'^') {
                    self.now += 1;
                    self.setv(Tok::OpenBrack, 0);
                } else {
                    self.setv(Tok::OpenBrack, 1);
                }
                return;
            }
            Some(b'.') => {
                self.set(Tok::Dot);
                return;
            }
            Some(b'^') => {
                self.set(Tok::Caret);
                return;
            }
            Some(b'$') => {
                self.set(Tok::Dollar);
                return;
            }
            Some(b'\\') => {
                if self.ateos() {
                    self.fail(Err::Eescape);
                    return;
                }
                // fall through to escape handling below
            }
            _ => {
                self.setv(Tok::Plain, i64::from(c));
                return;
            }
        }
        // ERE/ARE backslash handling; backslash already consumed.
        debug_assert!(!self.ateos());
        if self.cflags & REG_ADVF == 0 {
            c = self.input[self.now];
            self.now += 1;
            if is_alnum(c) {
                self.note(REG_UBSALNUM);
                self.note(REG_UUNSPEC);
            }
            self.setv(Tok::Plain, i64::from(c));
            return;
        }
        self.lexescape();
    }

    /// The `?` greedy/non-greedy suffix on `* + ?` in ARE mode. Returns 1 for
    /// greedy, 0 for non-greedy (and notes `UNONPOSIX`).
    fn adv_quant_q(&mut self) -> i64 {
        if self.cflags & REG_ADVF != 0 && self.next1(b'?') {
            self.now += 1;
            self.note(REG_UNONPOSIX);
            0
        } else {
            1
        }
    }

    /// Parse an ARE backslash escape (backslash already consumed).
    fn lexescape(&mut self) {
        debug_assert!(self.cflags & REG_ADVF != 0);
        if self.ateos() {
            self.fail(Err::Eescape);
            return;
        }
        let c = self.input[self.now];
        self.now += 1;
        if !is_alnum(c) {
            self.setv(Tok::Plain, i64::from(c));
            return;
        }
        self.note(REG_UNONPOSIX);
        match u8::try_from(c).ok() {
            Some(b'a') => {
                // `\a`/`\e` resolve via the locale-aware element lookup in C.
                self.note(REG_ULOCALE);
                self.setv(Tok::Plain, 0x07);
            }
            Some(b'A') => self.setv(Tok::Sbegin, 0),
            Some(b'b') => self.setv(Tok::Plain, 0x08),
            Some(b'B') => self.setv(Tok::Plain, i64::from(b'\\')),
            Some(b'c') => {
                self.note(REG_UUNPORT);
                if self.ateos() {
                    self.fail(Err::Eescape);
                    return;
                }
                let n = self.input[self.now];
                self.now += 1;
                self.setv(Tok::Plain, i64::from(n & 0o37));
            }
            Some(b'd') => {
                self.note(REG_ULOCALE);
                self.setv(Tok::Shorthand, i64::from(b'd'));
            }
            Some(b'D') => {
                self.note(REG_ULOCALE);
                self.setv(Tok::Shorthand, i64::from(b'D'));
            }
            Some(b'e') => {
                self.note(REG_UUNPORT);
                self.note(REG_ULOCALE);
                self.setv(Tok::Plain, 0x1B);
            }
            Some(b'f') => self.setv(Tok::Plain, 0x0C),
            Some(b'm') => self.set(Tok::Lt),
            Some(b'M') => self.set(Tok::Gt),
            Some(b'n') => self.setv(Tok::Plain, i64::from(b'\n')),
            Some(b'r') => self.setv(Tok::Plain, i64::from(b'\r')),
            Some(b's') => {
                self.note(REG_ULOCALE);
                self.setv(Tok::Shorthand, i64::from(b's'));
            }
            Some(b'S') => {
                self.note(REG_ULOCALE);
                self.setv(Tok::Shorthand, i64::from(b'S'));
            }
            Some(b't') => self.setv(Tok::Plain, i64::from(b'\t')),
            Some(b'u') => {
                let v = self.lexdigits(16, 1, 4);
                if self.iserr() {
                    self.fail(Err::Eescape);
                    return;
                }
                self.setv(Tok::Plain, v);
            }
            Some(b'U') => {
                let v = self.lexdigits(16, 1, 8);
                if self.iserr() {
                    self.fail(Err::Eescape);
                    return;
                }
                self.setv(Tok::Plain, v);
            }
            Some(b'v') => self.setv(Tok::Plain, 0x0B),
            Some(b'w') => {
                self.note(REG_ULOCALE);
                self.setv(Tok::Shorthand, i64::from(b'w'));
            }
            Some(b'W') => {
                self.note(REG_ULOCALE);
                self.setv(Tok::Shorthand, i64::from(b'W'));
            }
            Some(b'x') => {
                self.note(REG_UUNPORT);
                let v = self.lexdigits(16, 1, 2);
                if self.iserr() {
                    self.fail(Err::Eescape);
                    return;
                }
                self.setv(Tok::Plain, v);
            }
            Some(b'y') => {
                self.note(REG_ULOCALE);
                self.setv(Tok::Wbdry, 0);
            }
            Some(b'Y') => {
                self.note(REG_ULOCALE);
                self.setv(Tok::NWbdry, 0);
            }
            Some(b'Z') => self.setv(Tok::Send, 0),
            Some(d @ b'1'..=b'9') => {
                let save = self.now;
                self.now -= 1;
                let v = self.lexdigits(10, 1, 255);
                if self.iserr() {
                    self.fail(Err::Eescape);
                    return;
                }
                let ndig = self.now - (save - 1);
                if ndig == 1 || (v > 0 && (v as usize) <= self.nsubexp) {
                    self.note(REG_UBACKREF);
                    self.setv(Tok::Backref, v);
                    return;
                }
                let _ = d;
                // Not a backref after all: reparse as octal.
                self.now = save - 1;
                self.octal_escape();
            }
            Some(b'0') => {
                self.now -= 1;
                self.octal_escape();
            }
            _ => self.fail(Err::Eescape),
        }
    }

    fn octal_escape(&mut self) {
        self.note(REG_UUNPORT);
        let mut v = self.lexdigits(8, 1, 3);
        if self.iserr() {
            self.fail(Err::Eescape);
            return;
        }
        if v > 0xFF {
            self.now -= 1;
            v >>= 3;
        }
        self.setv(Tok::Plain, v);
    }

    fn lexdigits(&mut self, base: i64, minlen: usize, maxlen: usize) -> i64 {
        let mut n: i64 = 0;
        let mut len = 0;
        while len < maxlen && !self.ateos() {
            if n > 0x10FFF {
                break;
            }
            let c = self.input[self.now];
            self.now += 1;
            let Some(d) = char::from_u32(c).and_then(|c| c.to_digit(16)) else {
                self.now -= 1;
                break;
            };
            let d = i64::from(d);
            if d >= base {
                self.now -= 1;
                break;
            }
            n = n * base + d;
            len += 1;
        }
        if len < minlen {
            self.fail(Err::Eescape);
        }
        n
    }

    fn brenext(&mut self, mut c: Chr) {
        match u8::try_from(c).ok() {
            Some(b'*') => {
                if matches!(self.lasttype, Tok::Empty | Tok::OpenParen | Tok::Caret) {
                    self.setv(Tok::Plain, i64::from(c));
                } else {
                    self.setv(Tok::Star, 1);
                }
                return;
            }
            Some(b'[') => {
                if self.have(6)
                    && self.peek(0) == Some(Chr::from(b'['))
                    && self.peek(1) == Some(Chr::from(b':'))
                    && (self.peek(2) == Some(Chr::from(b'<'))
                        || self.peek(2) == Some(Chr::from(b'>')))
                    && self.peek(3) == Some(Chr::from(b':'))
                    && self.peek(4) == Some(Chr::from(b']'))
                    && self.peek(5) == Some(Chr::from(b']'))
                {
                    let which = self.peek(2).unwrap();
                    self.now += 6;
                    self.note(REG_UNONPOSIX);
                    if which == Chr::from(b'<') {
                        self.set(Tok::Lt);
                    } else {
                        self.set(Tok::Gt);
                    }
                    return;
                }
                self.lexcon = Con::Brack;
                if self.next1(b'^') {
                    self.now += 1;
                    self.setv(Tok::OpenBrack, 0);
                } else {
                    self.setv(Tok::OpenBrack, 1);
                }
                return;
            }
            Some(b'.') => {
                self.set(Tok::Dot);
                return;
            }
            Some(b'^') => {
                if self.lasttype == Tok::Empty {
                    self.set(Tok::Caret);
                    return;
                }
                if self.lasttype == Tok::OpenParen {
                    self.note(REG_UUNSPEC);
                    self.set(Tok::Caret);
                    return;
                }
                self.setv(Tok::Plain, i64::from(c));
                return;
            }
            Some(b'$') => {
                if self.cflags & REG_EXPANDED != 0 {
                    self.skip();
                }
                if self.ateos() {
                    self.set(Tok::Dollar);
                    return;
                }
                if self.next2(b'\\', b')') {
                    self.note(REG_UUNSPEC);
                    self.set(Tok::Dollar);
                    return;
                }
                self.setv(Tok::Plain, i64::from(c));
                return;
            }
            Some(b'\\') => {}
            _ => {
                self.setv(Tok::Plain, i64::from(c));
                return;
            }
        }
        // backslash
        if self.ateos() {
            self.fail(Err::Eescape);
            return;
        }
        c = self.input[self.now];
        self.now += 1;
        match u8::try_from(c).ok() {
            Some(b'{') => {
                self.lexcon = Con::Bbnd;
                self.note(REG_UBOUNDS);
                self.set(Tok::OpenBrace);
            }
            Some(b'(') => self.setv(Tok::OpenParen, 1),
            Some(b')') => self.setv(Tok::CloseParen, i64::from(c)),
            Some(b'<') => {
                self.note(REG_UNONPOSIX);
                self.set(Tok::Lt);
            }
            Some(b'>') => {
                self.note(REG_UNONPOSIX);
                self.set(Tok::Gt);
            }
            Some(d @ b'1'..=b'9') => {
                self.note(REG_UBACKREF);
                self.setv(Tok::Backref, i64::from(d - b'0'));
            }
            _ => {
                if is_alnum(c) {
                    self.note(REG_UBSALNUM);
                    self.note(REG_UUNSPEC);
                }
                self.setv(Tok::Plain, i64::from(c));
            }
        }
    }

    // parser

    fn see(&self, t: Tok) -> bool {
        self.nexttype == t
    }
    fn eat(&mut self, t: Tok) -> bool {
        if self.see(t) {
            self.next();
            true
        } else {
            false
        }
    }

    /// Parse an alternation up to a stopper (`)` if `!top`, else EOS).
    fn parse(&mut self, top: bool, in_lacon: bool) -> Node {
        // Guard the recursion: each nested group re-enters `parse` one frame
        // deeper, so a deeply nested pattern would overflow the stack. Bail with
        // the engine's resource-exhaustion error before that happens.
        if self.depth >= MAX_PARSE_DEPTH {
            self.fail(Err::Etoobig);
            return Node::Empty;
        }
        self.depth += 1;
        let node = self.parse_inner(top, in_lacon);
        self.depth -= 1;
        node
    }

    /// The body of [`parse`]; split out so the depth counter is restored on
    /// every return path without repeating the decrement at each `return`.
    fn parse_inner(&mut self, top: bool, in_lacon: bool) -> Node {
        let mut branches: Vec<Node> = Vec::new();
        loop {
            let b = self.parsebranch(false, in_lacon, top);
            branches.push(b);
            if self.iserr() {
                return Node::Empty;
            }
            if !self.eat(Tok::Bar) {
                break;
            }
        }
        let stopper_ok = if top {
            self.see(Tok::Eos)
        } else {
            self.see(Tok::CloseParen)
        };
        if !stopper_ok && !self.iserr() {
            // Reached EOS while expecting ')', or vice-versa.
            self.fail(Err::Eparen);
            return Node::Empty;
        }
        if branches.len() == 1 {
            branches.pop().unwrap()
        } else {
            Node::Alt(branches)
        }
    }

    /// Parse one concatenation branch. `top` controls whether `)` is the
    /// stopper (only inside parens) — at top level a stray `)` is an atom (the
    /// ERE botch) rather than an end-of-expression marker.
    fn parsebranch(&mut self, partial: bool, in_lacon: bool, top: bool) -> Node {
        let mut items: Vec<Node> = Vec::new();
        let mut seencontent = false;
        while !self.see(Tok::Bar) && (top || !self.see(Tok::CloseParen)) && !self.see(Tok::Eos) {
            seencontent = true;
            let n = self.parseqatom(in_lacon);
            if self.iserr() {
                return Node::Empty;
            }
            match n {
                Node::Empty => {}
                other => items.push(other),
            }
        }
        if !seencontent {
            if !partial {
                self.note(REG_UUNSPEC);
            }
            return Node::Empty;
        }
        if items.is_empty() {
            Node::Empty
        } else if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Node::Concat(items)
        }
    }

    /// Parse one quantified atom or constraint.
    fn parseqatom(&mut self, in_lacon: bool) -> Node {
        let atomtype = self.nexttype;
        // Constraints end by returning immediately.
        match atomtype {
            Tok::Caret => {
                self.next();
                return Node::Anchor(Anchor::Bol);
            }
            Tok::Dollar => {
                self.next();
                return Node::Anchor(Anchor::Eol);
            }
            Tok::Sbegin => {
                self.next();
                return Node::Anchor(Anchor::Bos);
            }
            Tok::Send => {
                self.next();
                return Node::Anchor(Anchor::Eos);
            }
            // Word-boundary constraints reference the (locale-dependent) word
            // class, so the C engine notes REG_ULOCALE via `wordchrs`.
            Tok::Lt => {
                self.note(REG_ULOCALE);
                self.next();
                return Node::Anchor(Anchor::WordBegin);
            }
            Tok::Gt => {
                self.note(REG_ULOCALE);
                self.next();
                return Node::Anchor(Anchor::WordEnd);
            }
            Tok::Wbdry => {
                self.note(REG_ULOCALE);
                self.next();
                return Node::Anchor(Anchor::WordBoundary);
            }
            Tok::NWbdry => {
                self.note(REG_ULOCALE);
                self.next();
                return Node::Anchor(Anchor::NotWordBoundary);
            }
            Tok::Lacon => {
                let positive = self.nextvalue != 0;
                self.next();
                let sub = self.parse(false, true);
                if self.iserr() {
                    return Node::Empty;
                }
                self.next(); // consume ')'
                return Node::Look {
                    positive,
                    sub: Box::new(sub),
                };
            }
            Tok::Star | Tok::Plus | Tok::Quest | Tok::OpenBrace => {
                self.fail(Err::Badrpt);
                return Node::Empty;
            }
            _ => {}
        }

        // The atom itself.
        let atom: Node;
        let mut cap_subno: Option<usize> = None;
        let mut is_backref = false;
        let mut backref_no = 0usize;
        match atomtype {
            Tok::CloseParen => {
                // Unbalanced ) — legal in EREs (spec botch), else EPAREN.
                if self.cflags & REG_ADVANCED != REG_EXTENDED {
                    self.fail(Err::Eparen);
                    return Node::Empty;
                }
                self.note(REG_UPBOTCH);
                atom = Node::Set(CharSet::one(
                    self.nextvalue as Chr,
                    self.cflags & REG_ICASE != 0,
                ));
                self.next();
            }
            Tok::Plain => {
                atom = Node::Set(CharSet::one(
                    self.nextvalue as Chr,
                    self.cflags & REG_ICASE != 0,
                ));
                self.next();
            }
            Tok::Shorthand => {
                atom = self.shorthand_set(self.nextvalue);
                self.next();
            }
            Tok::OpenBrack => {
                let negated = self.nextvalue == 0;
                atom = self.bracket(negated);
                if self.iserr() {
                    return Node::Empty;
                }
                self.next(); // consume ]
            }
            Tok::Dot => {
                atom = Node::Set(CharSet::any(self.cflags & REG_NLSTOP != 0));
                self.next();
            }
            Tok::OpenParen => {
                let cap = !in_lacon && self.nextvalue != 0;
                let subno = if cap {
                    self.nsubexp += 1;
                    let n = self.nsubexp;
                    if n >= self.subs_defined.len() {
                        self.subs_defined.resize(n + 1, false);
                    }
                    Some(n)
                } else {
                    None
                };
                self.next();
                let inner = self.parse(false, in_lacon);
                if self.iserr() {
                    return Node::Empty;
                }
                self.next(); // consume ')'
                if let Some(n) = subno {
                    self.subs_defined[n] = true;
                    cap_subno = Some(n);
                    atom = Node::Capture {
                        subno: n,
                        sub: Box::new(inner),
                    };
                } else {
                    atom = inner;
                }
            }
            Tok::Backref => {
                let n = self.nextvalue as usize;
                if in_lacon || n >= self.subs_defined.len() || !self.subs_defined[n] {
                    self.fail(Err::Esubreg);
                    return Node::Empty;
                }
                is_backref = true;
                backref_no = n;
                atom = Node::Empty; // placeholder; replaced below
                self.next();
            }
            _ => {
                self.fail(Err::Assert);
                return Node::Empty;
            }
        }

        // Optional quantifier.
        let (m, n, qpref) = match self.nexttype {
            Tok::Star => {
                let p = Self::pref_from(self.nextvalue);
                self.next();
                (0, DUPINF, Some(p))
            }
            Tok::Plus => {
                let p = Self::pref_from(self.nextvalue);
                self.next();
                (1, DUPINF, Some(p))
            }
            Tok::Quest => {
                let p = Self::pref_from(self.nextvalue);
                self.next();
                (0, 1, Some(p))
            }
            Tok::OpenBrace => {
                self.next();
                let m = self.scannum();
                if self.iserr() {
                    return Node::Empty;
                }
                let (n, pref) = if self.eat(Tok::Comma) {
                    let n = if self.see(Tok::Digit) {
                        self.scannum()
                    } else {
                        DUPINF
                    };
                    if self.iserr() {
                        return Node::Empty;
                    }
                    if m > n {
                        self.fail(Err::Badbr);
                        return Node::Empty;
                    }
                    (n, Some(Self::pref_from(self.nextvalue)))
                } else {
                    (m, None)
                };
                if !self.see(Tok::CloseBrace) {
                    self.fail(Err::Badbr);
                    return Node::Empty;
                }
                self.next();
                (m, n, pref)
            }
            _ => (1, 1, None),
        };

        // {0} / {0,0} cancels everything.
        if m == 0 && n == 0 {
            if let Some(sub) = cap_subno {
                // The group existed (nsubexp counted) but its backrefs are now
                // invalid; mirror C's `v->subs[subno] = NULL`.
                self.subs_defined[sub] = false;
            }
            return Node::Empty;
        }

        let pref = qpref.unwrap_or(Pref::Longer);

        if is_backref {
            return Node::Backref {
                subno: backref_no,
                min: m,
                max: n,
                pref,
            };
        }

        if m == 1 && n == 1 && qpref.is_none() {
            return atom;
        }

        Node::Repeat {
            sub: Box::new(atom),
            min: m,
            max: n,
            pref,
        }
    }

    fn pref_from(greedy: i64) -> Pref {
        if greedy != 0 {
            Pref::Longer
        } else {
            Pref::Shorter
        }
    }

    fn scannum(&mut self) -> i32 {
        let mut n: i32 = 0;
        while self.see(Tok::Digit) && n < DUPMAX {
            n = n * 10 + self.nextvalue as i32;
            self.next();
        }
        if self.see(Tok::Digit) || n > DUPMAX {
            self.fail(Err::Badbr);
            return 0;
        }
        n
    }

    /// `\d \D \s \S \w \W` at the atom level → a `Set` node.
    fn shorthand_set(&self, letter: i64) -> Node {
        let nocase = self.cflags & REG_ICASE != 0;
        let mut set = CharSet {
            nocase,
            ..CharSet::default()
        };
        match u8::try_from(letter).ok() {
            Some(b'd') => set.classes.push(ClassKind::Digit),
            Some(b'D') => {
                set.classes.push(ClassKind::Digit);
                set.negated = true;
            }
            Some(b's') => set.classes.push(ClassKind::Space),
            Some(b'S') => {
                set.classes.push(ClassKind::Space);
                set.negated = true;
            }
            Some(b'w') => {
                set.classes.push(ClassKind::Alnum);
                set.ranges.push((u32::from(b'_'), u32::from(b'_')));
            }
            Some(b'W') => {
                set.classes.push(ClassKind::Alnum);
                set.ranges.push((u32::from(b'_'), u32::from(b'_')));
                set.negated = true;
            }
            _ => {}
        }
        Node::Set(set)
    }

    /// Parse a bracket expression `[...]` (the `[` token is current). `negated`
    /// is the `[^...]` form. On return the current token is `]`.
    fn bracket(&mut self, negated: bool) -> Node {
        let nocase = self.cflags & REG_ICASE != 0;
        let mut set = CharSet {
            negated,
            nocase,
            exclude_newline: negated && (self.cflags & REG_NLSTOP != 0),
            ..CharSet::default()
        };
        self.next(); // consume '['
        while !self.see(Tok::CloseBrack) && !self.see(Tok::Eos) {
            self.brackpart(&mut set);
            if self.iserr() {
                return Node::Empty;
            }
        }
        Node::Set(set)
    }

    fn brackpart(&mut self, set: &mut CharSet) {
        match self.nexttype {
            Tok::Range => {
                self.fail(Err::Erange);
            }
            Tok::Plain => {
                let startc = self.nextvalue as Chr;
                self.next();
                if !self.see(Tok::Range) {
                    set.ranges.push((startc, startc));
                    return;
                }
                self.next(); // consume '-'
                self.bracket_range_end(set, startc);
            }
            Tok::Shorthand => {
                let letter = self.nextvalue;
                self.next();
                Self::add_shorthand_to_set(set, letter);
            }
            Tok::Cclass => {
                let name = self.scan_class_name();
                if self.iserr() {
                    return;
                }
                match ClassKind::from_name(&name) {
                    Some(mut k) => {
                        // Under -nocase, C Tcl's `cclass()` remaps CC_UPPER /
                        // CC_LOWER to CC_ALPHA — *letters*, not alnum. Folding to
                        // `Alnum` wrongly matched digits, so
                        // `regexp -nocase {[[:upper:]]} 5` matched here but not
                        // in tclsh.
                        if set.nocase && matches!(k, ClassKind::Lower | ClassKind::Upper) {
                            k = ClassKind::Alpha;
                        }
                        set.classes.push(k);
                    }
                    None => self.fail(Err::Ectype),
                }
            }
            Tok::Collel => {
                let chars = self.scan_collating();
                if self.iserr() {
                    return;
                }
                if chars.len() > 1 {
                    // A named collating element is locale-dependent.
                    self.note(REG_ULOCALE);
                }
                let Some(startc) = collating_element(&chars) else {
                    self.fail(Err::Ecollate);
                    return;
                };
                if !self.see(Tok::Range) {
                    set.ranges.push((startc, startc));
                    return;
                }
                self.next();
                self.bracket_range_end(set, startc);
            }
            Tok::Eclass => {
                let chars = self.scan_collating();
                if self.iserr() {
                    return;
                }
                let Some(c) = collating_element(&chars) else {
                    self.fail(Err::Ecollate);
                    return;
                };
                // Equivalence class — without locale data this is just {c}.
                set.ranges.push((c, c));
            }
            _ => self.fail(Err::Assert),
        }
    }

    fn bracket_range_end(&mut self, set: &mut CharSet, startc: Chr) {
        let endc = match self.nexttype {
            Tok::Plain | Tok::Range => {
                let c = self.nextvalue as Chr;
                self.next();
                c
            }
            Tok::Collel => {
                let chars = self.scan_collating();
                if self.iserr() {
                    return;
                }
                if let Some(c) = collating_element(&chars) {
                    c
                } else {
                    self.fail(Err::Ecollate);
                    return;
                }
            }
            _ => {
                self.fail(Err::Erange);
                return;
            }
        };
        if startc > endc {
            self.fail(Err::Erange);
            return;
        }
        if startc != endc {
            self.note(REG_UUNPORT);
        }
        set.ranges.push((startc, endc));
    }

    fn add_shorthand_to_set(set: &mut CharSet, letter: i64) {
        // Inside [], only d/s/w are reachable (lexer rejects D/S/W).
        match u8::try_from(letter).ok() {
            Some(b'd') => set.classes.push(ClassKind::Digit),
            Some(b's') => set.classes.push(ClassKind::Space),
            Some(b'w') => {
                set.classes.push(ClassKind::Alnum);
                set.ranges.push((u32::from(b'_'), u32::from(b'_')));
            }
            _ => {}
        }
    }

    /// Read the name inside `[:`…`:]` (current token is `Cclass`). Leaves the
    /// token after the closing `:]`.
    fn scan_class_name(&mut self) -> String {
        self.next(); // consume the Cclass marker
        let mut s = String::new();
        while self.see(Tok::Plain) {
            if let Some(c) = char::from_u32(self.nextvalue as u32) {
                s.push(c);
            }
            self.next();
        }
        // Should be at End.
        self.next(); // consume END
        s
    }

    /// Read the codepoints inside `[.`…`.]` / `[=`…`=]`.
    fn scan_collating(&mut self) -> Vec<Chr> {
        self.next(); // consume the marker
        let mut v = Vec::new();
        while self.see(Tok::Plain) {
            v.push(self.nextvalue as Chr);
            self.next();
        }
        self.next(); // consume END
        v
    }
}

fn collating_element(chars: &[Chr]) -> Option<Chr> {
    if chars.len() == 1 {
        return Some(chars[0]);
    }
    // Named collating elements — a faithful port of C Tcl's `cnames[]` table
    // (`generic/regc_locale.c`): the POSIX portable-character-set names plus the
    // ASCII control-code mnemonics. Verified entry-by-entry against tclsh 9.0.3
    // (see `tests/collating_names_oracle.rs`). Names are case-sensitive, exactly
    // as C Tcl compares them.
    let name: String = chars.iter().filter_map(|&c| char::from_u32(c)).collect();
    let cp: u8 = match name.as_str() {
        "NUL" => 0,
        "SOH" => 1,
        "STX" => 2,
        "ETX" => 3,
        "EOT" => 4,
        "ENQ" => 5,
        "ACK" => 6,
        "alert" | "BEL" => 7,
        "BS" => 8,
        "tab" | "HT" => 9,
        "LF" | "newline" => 10,
        "VT" => 11,
        "FF" => 12,
        "CR" => 13,
        "SO" => 14,
        "SI" => 15,
        "DLE" => 16,
        "DC1" => 17,
        "DC2" => 18,
        "DC3" => 19,
        "DC4" => 20,
        "NAK" => 21,
        "SYN" => 22,
        "ETB" => 23,
        "CAN" => 24,
        "EM" => 25,
        "SUB" => 26,
        "ESC" => 27,
        "IS4" | "FS" => 28,
        "IS3" | "GS" => 29,
        "IS2" | "RS" => 30,
        "IS1" | "US" => 31,
        "space" => 32,
        "exclamation-mark" => 33,
        "quotation-mark" => 34,
        "number-sign" => 35,
        "dollar-sign" => 36,
        "percent-sign" => 37,
        "ampersand" => 38,
        "apostrophe" => 39,
        "left-parenthesis" => 40,
        "right-parenthesis" => 41,
        "asterisk" => 42,
        "plus-sign" => 43,
        "comma" => 44,
        "hyphen" | "hyphen-minus" => 45,
        "period" | "full-stop" => 46,
        "slash" | "solidus" => 47,
        "zero" => 48,
        "one" => 49,
        "two" => 50,
        "three" => 51,
        "four" => 52,
        "five" => 53,
        "six" => 54,
        "seven" => 55,
        "eight" => 56,
        "nine" => 57,
        "colon" => 58,
        "semicolon" => 59,
        "less-than-sign" => 60,
        "equals-sign" => 61,
        "greater-than-sign" => 62,
        "question-mark" => 63,
        "commercial-at" => 64,
        "left-square-bracket" => 91,
        "backslash" | "reverse-solidus" => 92,
        "right-square-bracket" => 93,
        "circumflex" | "circumflex-accent" => 94,
        "underscore" | "low-line" => 95,
        "grave-accent" => 96,
        "left-brace" | "left-curly-bracket" => 123,
        "vertical-line" => 124,
        "right-brace" | "right-curly-bracket" => 125,
        "tilde" => 126,
        "DEL" => 127,
        _ => return None,
    };
    Some(u32::from(cp))
}

fn is_digit(c: Chr) -> bool {
    (0x30..=0x39).contains(&c)
}
fn digit_val(c: Chr) -> Option<u32> {
    if is_digit(c) { Some(c - 0x30) } else { None }
}
fn is_alpha(c: Chr) -> bool {
    matches!(c, 0x41..=0x5A | 0x61..=0x7A)
}
fn is_alnum(c: Chr) -> bool {
    is_alpha(c) || is_digit(c)
}

fn is_ascii_ws(c: Chr) -> bool {
    matches!(c, 0x09 | 0x0A | 0x0B | 0x0C | 0x0D | 0x20)
}
