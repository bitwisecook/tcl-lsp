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

// Altered Rust implementation of Jim's bundled jimregexp engine.
// Original engine: Copyright (c) 1986 by University of Toronto.
// Written by Henry Spencer. Not derived from licensed software.
// Jim's Tcl-compatible engine includes contributions by Steve Bennett.
//
// Permission is granted to anyone to use this software for any purpose on any
// computer system, and to redistribute it freely, subject to these restrictions:
// 1. The author is not responsible for the consequences of use of this software,
//    no matter how awful, even if they arise from defects in it.
// 2. The origin of this software must not be misrepresented, either by explicit
//    claim or by omission.
// 3. Altered versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.

//! Jim084's bundled integer-program regular-expression compiler and matcher.
//! Inputs retain original `CString` extents and matches retain byte offsets.
//! Numeric decoding and case mapping use the shared pinned Jim owners.

use tcl_syntax::raw_string::{jim084_decode, jim084_upper};

const END: i32 = 0;
const BOL: i32 = 1;
const EOL: i32 = 2;
const ANY: i32 = 3;
const ANYOF: i32 = 4;
const ANYBUT: i32 = 5;
const BRANCH: i32 = 6;
const BACK: i32 = 7;
const EXACTLY: i32 = 8;
const NOTHING: i32 = 9;
const REP: i32 = 10;
const REPMIN: i32 = 11;
const REPX: i32 = 12;
const REPXMIN: i32 = 13;
const BOLX: i32 = 14;
const EOLX: i32 = 15;
const WORDA: i32 = 16;
const WORDZ: i32 = 17;
const OPENNC: i32 = 1000;
const OPEN: i32 = 1001;
const CLOSENC: i32 = 2000;
const CLOSE: i32 = 2001;
const MAGIC: i32 = -86_061_043; // 0xFADED00D as signed native int
const WIDTH: u8 = 1;
const SIMPLE: u8 = 2;
const SPSTART: u8 = 4;
const MAX_COUNT: i32 = 1_000_000;
const MAX_DEPTH: usize = 1000;

/// Physical bundled Jim compile flags, independent of the C ARE ABI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    key: u32,
}
impl Flags {
    #[must_use]
    pub const fn new() -> Self {
        Self { key: 0 }
    }

    const fn set(self, bit: u32, enabled: bool) -> Self {
        Self {
            key: if enabled {
                self.key | bit
            } else {
                self.key & !bit
            },
        }
    }

    #[must_use]
    pub const fn with_nocase(self, enabled: bool) -> Self {
        self.set(2, enabled)
    }

    #[must_use]
    pub const fn with_lineanchor(self, enabled: bool) -> Self {
        self.set(4, enabled)
    }

    #[must_use]
    pub const fn with_linestop(self, enabled: bool) -> Self {
        self.set(8, enabled)
    }

    #[must_use]
    pub const fn with_expanded(self, enabled: bool) -> Self {
        self.set(32, enabled)
    }

    #[must_use]
    pub const fn key(self) -> u32 {
        self.key
    }

    const fn nocase(self) -> bool {
        self.key & 2 != 0
    }

    const fn lineanchor(self) -> bool {
        self.key & 4 != 0
    }

    const fn linestop(self) -> bool {
        self.key & 8 != 0
    }

    const fn expanded(self) -> bool {
        self.key & 32 != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Syntax(u8),
    Unavailable,
}
impl Error {
    #[must_use]
    pub fn message(self) -> &'static str {
        const ERRORS: [&str; 20] = [
            "success",
            "no match",
            "bad pattern",
            "null argument",
            "unknown error",
            "too big",
            "out of memory",
            "too many ()",
            "parentheses () not balanced",
            "braces {} not balanced",
            "invalid repetition count(s)",
            "extra characters",
            "*+ of empty atom",
            "nested count",
            "internal error",
            "count follows nothing",
            "invalid escape \\ sequence",
            "corrupted program",
            "contains null char",
            "brackets [] not balanced",
        ];
        match self {
            Self::Syntax(code) => ERRORS
                .get(usize::from(code))
                .copied()
                .unwrap_or("Bad error code"),
            Self::Unavailable => "Jim regexp execution capability unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    pub start: Option<usize>,
    pub end: Option<usize>,
}

/// Actual ordered integer instructions emitted by the bundled Jim compiler.
pub struct Regex {
    program: Vec<i32>,
    flags: Flags,
    nsub: usize,
    anchored: bool,
    first: Option<i32>,
    required: Option<Vec<i32>>,
}
impl Regex {
    pub fn compile(original: &[u8], flags: Flags) -> Result<Self, Error> {
        let source = tcl_core_types::c_string_extent(original);
        let source = if flags.expanded() {
            expanded(source)
        } else {
            source.to_vec()
        };
        let mut compiler = Compiler {
            source,
            pos: 0,
            program: vec![MAGIC],
            flags,
            nsub: 0,
            depth: 0,
        };
        let (_, shape) = compiler.group(false)?;
        if compiler.nsub >= 100 {
            return Err(Error::Syntax(5));
        }
        let next = compiler.next(1)?;
        let anchored = compiler.program[next] == END && compiler.program[3] == BOL;
        let first = (compiler.program[next] == END && compiler.program[3] == EXACTLY)
            .then(|| compiler.program[5]);
        let mut required = None;
        if compiler.program[next] == END && shape & SPSTART != 0 {
            let mut scan = 3;
            while scan != 0 {
                if compiler.program[scan] == EXACTLY {
                    let literal: Vec<_> = compiler.program[scan + 2..]
                        .iter()
                        .copied()
                        .take_while(|&unit| unit != 0)
                        .collect();
                    if required
                        .as_ref()
                        .is_none_or(|previous: &Vec<i32>| literal.len() >= previous.len())
                    {
                        required = Some(literal);
                    }
                }
                scan = compiler.next(scan)?;
            }
        }
        Ok(Self {
            program: compiler.program,
            flags,
            nsub: compiler.nsub,
            anchored,
            first,
            required,
        })
    }
    #[must_use]
    pub const fn nsub(&self) -> usize {
        self.nsub
    }
    #[must_use]
    pub fn program(&self) -> &[i32] {
        &self.program
    }
    pub fn execute(
        &mut self,
        original: &[u8],
        captures: usize,
        notbol: bool,
    ) -> Result<Option<Vec<Match>>, Error> {
        let source = tcl_core_types::c_string_extent(original);
        if let Some(required) = &self.required {
            let mut cursor = 0;
            let mut found = false;
            while cursor <= source.len() {
                if prefix(required, source, cursor, self.flags.nocase()) {
                    found = true;
                    break;
                }
                if cursor == source.len() {
                    break;
                }
                cursor += decode(source, cursor, self.flags.nocase()).1;
            }
            if !found {
                return Ok(None);
            }
        }
        let mut scan = 3;
        while scan < self.program.len() && self.program[scan] != END {
            if matches!(self.program[scan], REPX | REPXMIN) {
                self.program[scan + 4] = 0;
            }
            scan += instruction_size(&self.program, scan)?;
        }
        let mut state = Execution {
            regex: self,
            source,
            input: 0,
            bol: 0,
            matches: vec![
                Match {
                    start: None,
                    end: None
                };
                captures.max(1)
            ],
            notbol,
            depth: 0,
        };
        let mut start = 0;
        loop {
            if let Some(first) = state.regex.first.filter(|&unit| unit != 0) {
                while start < source.len()
                    && decode(source, start, state.regex.flags.nocase()).0 != first
                {
                    start += decode(source, start, state.regex.flags.nocase()).1;
                }
                if start >= source.len() {
                    break;
                }
            }
            if !state.regex.anchored || !notbol || start > 0 {
                state.input = start;
                state.matches.fill(Match {
                    start: None,
                    end: None,
                });
                if state.run(1)? {
                    state.matches[0] = Match {
                        start: Some(start),
                        end: Some(state.input),
                    };
                    return Ok(Some(state.matches));
                }
            }
            if start >= source.len() {
                break;
            }
            if state.regex.anchored {
                if !state.regex.flags.lineanchor() {
                    break;
                }
                let Some(next) = source[start..].iter().position(|&byte| byte == b'\n') else {
                    break;
                };
                start += next + 1;
                state.bol = start;
            } else if state.regex.first.is_some_and(|unit| unit != 0) {
                start += 1;
            } else {
                start += decode(source, start, false).1;
            }
        }
        Ok(None)
    }
}

fn prefix(literal: &[i32], source: &[u8], mut cursor: usize, nocase: bool) -> bool {
    for &unit in literal {
        if cursor >= source.len() {
            return false;
        }
        let (actual, width) = decode(source, cursor, nocase);
        if unit != actual {
            return false;
        }
        cursor += width;
    }
    true
}

fn expanded(source: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut escaped = false;
    let mut pos = 0;
    while let Some(&byte) = source.get(pos) {
        if escaped {
            escaped = false;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
        } else if b" \t\r\n\x0c\x0b".contains(&byte) {
            pos += 1;
            continue;
        } else if byte == b'#' {
            while pos < source.len() && source[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }
        result.push(byte);
        pos += 1;
    }
    result
}
fn decode(source: &[u8], pos: usize, nocase: bool) -> (i32, usize) {
    let (unit, width) = jim084_decode(source.get(pos..).unwrap_or_default()).unwrap_or((0, 1));
    (
        i32::try_from(if nocase { jim084_upper(unit) } else { unit }).unwrap_or(i32::MAX),
        width,
    )
}
fn mult(source: &[u8], pos: usize) -> bool {
    matches!(source.get(pos), Some(b'*' | b'+' | b'?'))
        || (source.get(pos) == Some(&b'{') && source.get(pos + 1).is_some_and(u8::is_ascii_digit))
}
struct Compiler {
    source: Vec<u8>,
    pos: usize,
    program: Vec<i32>,
    flags: Flags,
    nsub: usize,
    depth: usize,
}
impl Compiler {
    fn byte(&self) -> u8 {
        self.source.get(self.pos).copied().unwrap_or(0)
    }
    fn node(&mut self, opcode: i32) -> usize {
        let id = self.program.len();
        self.program.extend([opcode, 0]);
        id
    }
    fn next(&self, id: usize) -> Result<usize, Error> {
        let offset = *self.program.get(id + 1).ok_or(Error::Unavailable)?;
        if offset == 0 {
            return Ok(0);
        }
        let offset = usize::try_from(offset).map_err(|_| Error::Unavailable)?;
        if self.program[id] == BACK {
            id.checked_sub(offset)
        } else {
            id.checked_add(offset)
        }
        .filter(|&id| id < self.program.len())
        .ok_or(Error::Unavailable)
    }
    fn tail(&mut self, mut id: usize, value: usize) -> Result<(), Error> {
        loop {
            let next = self.next(id)?;
            if next == 0 {
                break;
            }
            id = next;
        }
        let offset = if self.program[id] == BACK {
            id.checked_sub(value)
        } else {
            value.checked_sub(id)
        }
        .ok_or(Error::Unavailable)?;
        self.program[id + 1] = i32::try_from(offset).map_err(|_| Error::Unavailable)?;
        Ok(())
    }
    fn group(&mut self, parenthesized: bool) -> Result<(usize, u8), Error> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::Unavailable);
        }
        self.depth += 1;
        let result = self.group_inner(parenthesized);
        self.depth -= 1;
        result
    }
    fn group_inner(&mut self, parenthesized: bool) -> Result<(usize, u8), Error> {
        let mut flags = WIDTH;
        let number = if parenthesized && self.source.get(self.pos..self.pos + 2) == Some(b"?:") {
            self.pos += 2;
            -1
        } else {
            self.nsub += usize::from(parenthesized);
            i32::try_from(self.nsub).map_err(|_| Error::Unavailable)?
        };
        let mut result = if parenthesized {
            self.node(OPEN + number)
        } else {
            0
        };
        let (branch, f) = self.branch()?;
        if result == 0 {
            result = branch;
        } else {
            self.tail(result, branch)?;
        }
        if f & WIDTH == 0 {
            flags &= !WIDTH;
        }
        flags |= f & SPSTART;
        while self.byte() == b'|' {
            self.pos += 1;
            let (branch, f) = self.branch()?;
            self.tail(result, branch)?;
            if f & WIDTH == 0 {
                flags &= !WIDTH;
            }
            flags |= f & SPSTART;
        }
        let end = self.node(if parenthesized { CLOSE + number } else { END });
        self.tail(result, end)?;
        let mut branch = result;
        while branch != 0 {
            if self.program[branch] == BRANCH {
                self.tail(branch + 2, end)?;
            }
            branch = self.next(branch)?;
        }
        if parenthesized {
            if self.byte() != b')' {
                return Err(Error::Syntax(8));
            }
            self.pos += 1;
        } else if self.byte() != 0 {
            return Err(Error::Syntax(if self.byte() == b')' { 8 } else { 11 }));
        }
        Ok((result, flags))
    }
    fn branch(&mut self) -> Result<(usize, u8), Error> {
        let result = self.node(BRANCH);
        let mut chain = 0;
        let mut flags = 0;
        while !matches!(self.byte(), 0 | b')' | b'|') {
            let (part, f) = self.piece()?;
            flags |= f & WIDTH;
            if chain == 0 {
                flags |= f & SPSTART;
            } else {
                self.tail(chain, part)?;
            }
            chain = part;
        }
        if chain == 0 {
            self.node(NOTHING);
        }
        Ok((result, flags))
    }
    fn count(&mut self) -> Result<Option<i32>, Error> {
        let begin = self.pos;
        let mut value = 0i32;
        while self.byte().is_ascii_digit() {
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(i32::from(self.byte() - b'0')))
                .ok_or(Error::Syntax(10))?;
            self.pos += 1;
        }
        Ok((begin != self.pos).then_some(value))
    }
    fn piece(&mut self) -> Result<(usize, u8), Error> {
        let (result, flags) = self.atom()?;
        if !mult(&self.source, self.pos) {
            return Ok((result, flags));
        }
        let op = self.byte();
        if flags & WIDTH == 0 && op != b'?' {
            return Err(Error::Syntax(12));
        }
        let (min, max) = if op == b'{' {
            self.pos += 1;
            let min = self.count()?.ok_or(Error::Syntax(10))?;
            let max = if self.byte() == b'}' {
                min
            } else {
                if self.byte() == 0 {
                    return Err(Error::Syntax(9));
                }
                self.pos += 1;
                self.count()?.unwrap_or(MAX_COUNT)
            };
            if self.byte() != b'}' {
                return Err(Error::Syntax(9));
            }
            if min >= 100 || max < min || (max >= 100 && max != MAX_COUNT) {
                return Err(Error::Syntax(10));
            }
            (min, max)
        } else {
            (
                i32::from(op == b'+'),
                if op == b'?' { 1 } else { MAX_COUNT },
            )
        };
        let minimal = self.source.get(self.pos + 1) == Some(&b'?');
        if minimal {
            self.pos += 1;
        }
        let opcode = match (flags & SIMPLE != 0, minimal) {
            (true, false) => REP,
            (true, true) => REPMIN,
            (false, false) => REPX,
            (false, true) => REPXMIN,
        };
        self.program
            .splice(result..result, [opcode, 0, max, min, 0]);
        if flags & SIMPLE == 0 {
            let back = self.node(BACK);
            self.tail(back, result)?;
            self.tail(result + 5, back)?;
        }
        self.pos += 1;
        if mult(&self.source, self.pos) {
            return Err(Error::Syntax(13));
        }
        Ok((result, if min != 0 { WIDTH } else { SPSTART }))
    }
    fn range(&mut self, lower: i32, upper: i32) {
        if lower > upper {
            self.range(upper, lower);
        }
        self.program.extend([upper - lower + 1, lower]);
    }
    fn ascii_set(&mut self, source: &[u8]) {
        for &byte in source {
            self.range(i32::from(byte), i32::from(byte));
        }
    }
    fn class(&mut self, name: &[u8]) -> bool {
        match name {
            b"alnum" => {
                self.range(48, 57);
                self.class(b"alpha");
            }
            b"alpha" => {
                if !self.flags.nocase() {
                    self.range(97, 122);
                }
                self.range(65, 90);
            }
            b"space" => self.ascii_set(b" \t\r\n\x0c\x0b"),
            b"blank" => self.ascii_set(b" \t"),
            b"upper" => self.range(65, 90),
            b"lower" => self.range(97, 122),
            b"xdigit" => {
                self.range(97, 102);
                self.range(65, 70);
                self.range(48, 57);
            }
            b"digit" => self.range(48, 57),
            b"cntrl" => {
                self.range(0, 31);
                self.range(127, 127);
            }
            b"print" => self.range(32, 126),
            b"graph" => self.range(33, 126),
            b"punct" => {
                self.range(33, 47);
                self.range(58, 64);
                self.range(91, 96);
                self.range(123, 126);
            }
            _ => return false,
        }
        true
    }
    fn escape(&mut self) -> i32 {
        let byte = self.byte();
        self.pos += usize::from(byte != 0);
        let mut value = i32::from(byte);
        match byte {
            b'b' => value = 8,
            b'e' => value = 27,
            b'f' => value = 12,
            b'n' => value = 10,
            b'r' => value = 13,
            b't' => value = 9,
            b'v' => value = 11,
            0 => value = 92,
            b'u' | b'U' | b'x' => {
                let start = self.pos;
                let braces = byte == b'u' && self.byte() == b'{';
                if braces {
                    self.pos += 1;
                }
                let begin = self.pos;
                let digits = if braces {
                    6
                } else if byte == b'u' {
                    4
                } else if byte == b'U' {
                    8
                } else {
                    2
                };
                let mut decoded = 0u32;
                for _ in 0..digits {
                    let Some(d) = char::from(self.byte()).to_digit(16) else {
                        break;
                    };
                    decoded = (decoded << 4) | d;
                    self.pos += 1;
                }
                if self.pos != begin && (!braces || (self.byte() == b'}' && decoded <= 0x1fffff)) {
                    value = decoded as i32;
                    if braces {
                        self.pos += 1;
                    }
                } else {
                    self.pos = start;
                }
            }
            _ => {}
        }
        value
    }
    fn charset(&mut self) -> Result<(usize, u8), Error> {
        let complement = self.byte() == b'^';
        self.pos += usize::from(complement);
        let result = self.node(if complement { ANYBUT } else { ANYOF });
        if matches!(self.byte(), b']' | b'-') {
            let c = i32::from(self.byte());
            self.range(c, c);
            self.pos += 1;
        }
        while self.byte() != b']' {
            if self.byte() == 0 {
                return Err(Error::Syntax(19));
            }
            let (mut start, width) = decode(&self.source, self.pos, self.flags.nocase());
            self.pos += width;
            if start == 92 {
                let shorthand = self.byte();
                if matches!(shorthand, b's' | b'd' | b'w') {
                    self.pos += 1;
                    if shorthand == b'w' {
                        self.range(95, 95);
                    }
                    self.class(match shorthand {
                        b's' => b"space",
                        b'd' => b"digit",
                        _ => b"alnum",
                    });
                    continue;
                }
                start = self.escape();
                if start == 0 {
                    return Err(Error::Syntax(18));
                }
                if start == 92 && self.byte() == 0 {
                    return Err(Error::Syntax(16));
                }
            }
            if self.byte() == b'-'
                && self
                    .source
                    .get(self.pos + 1)
                    .is_some_and(|&b| b != 0 && b != b']')
            {
                self.pos += 1;
                let (mut end, width) = decode(&self.source, self.pos, self.flags.nocase());
                self.pos += width;
                if end == 92 {
                    end = self.escape();
                    if end == 0 {
                        return Err(Error::Syntax(18));
                    }
                    if end == 92 && self.byte() == 0 {
                        return Err(Error::Syntax(16));
                    }
                }
                self.range(start, end);
                continue;
            }
            if start == 91
                && self.byte() == b':'
                && let Some(end) = self.source[self.pos + 1..]
                    .windows(2)
                    .position(|p| p == b":]")
            {
                let name = self.source[self.pos + 1..self.pos + 1 + end].to_vec();
                if self.class(&name) {
                    self.pos += end + 3;
                    continue;
                }
            }
            self.range(start, start);
        }
        self.program.push(0);
        self.pos += 1;
        Ok((result, WIDTH | SIMPLE))
    }
    fn atom(&mut self) -> Result<(usize, u8), Error> {
        let saved = self.pos;
        let (unit, width) = decode(&self.source, self.pos, self.flags.nocase());
        self.pos += width;
        let opcode = match unit {
            94 => Some(BOL),
            36 => Some(EOL),
            46 => Some(ANY),
            _ => None,
        };
        if let Some(opcode) = opcode {
            return Ok((
                self.node(opcode),
                if opcode == ANY { WIDTH | SIMPLE } else { 0 },
            ));
        }
        if unit == 91 {
            return self.charset();
        }
        if unit == 40 {
            return self.group(true).map(|(id, f)| (id, f & (WIDTH | SPSTART)));
        }
        if matches!(unit, 0 | 124 | 41) {
            return Err(Error::Syntax(14));
        }
        if unit == 92 {
            let escaped = self.byte();
            self.pos += usize::from(escaped != 0);
            let opcode = match escaped {
                0 => return Err(Error::Syntax(16)),
                b'A' => Some(BOLX),
                b'Z' => Some(EOLX),
                b'<' | b'm' => Some(WORDA),
                b'>' | b'M' => Some(WORDZ),
                _ => None,
            };
            if let Some(opcode) = opcode {
                return Ok((self.node(opcode), 0));
            }
            if matches!(escaped, b'd' | b'D' | b'w' | b'W' | b's' | b'S') {
                let result = self.node(if escaped.is_ascii_lowercase() {
                    ANYOF
                } else {
                    ANYBUT
                });
                let name = match escaped.to_ascii_lowercase() {
                    b'd' => b"digit".as_slice(),
                    b'w' => b"alnum",
                    _ => b"space",
                };
                // Standalone \w has a different emitted range order from
                // bracket CC_ALNUM in Jim's compiler.
                if escaped.eq_ignore_ascii_case(&b'w') {
                    self.class(b"alpha");
                    self.class(b"digit");
                    self.range(95, 95);
                } else {
                    self.class(name);
                }
                self.program.push(0);
                return Ok((result, WIDTH | SIMPLE));
            }
        }
        self.pos = saved;
        if mult(&self.source, self.pos) {
            return Err(Error::Syntax(15));
        }
        let result = self.node(EXACTLY);
        let mut added = 0;
        while self.byte() != 0
            && !b"^$.[()|".contains(&self.byte())
            && !mult(&self.source, self.pos)
        {
            let before = self.pos;
            let (mut c, width) = decode(&self.source, self.pos, self.flags.nocase());
            self.pos += width;
            if c == 92 && self.byte() != 0 {
                if b"<>mMwWdDsSAZ".contains(&self.byte()) {
                    self.pos = before;
                    break;
                }
                c = self.escape();
                if c == 0 {
                    return Err(Error::Syntax(18));
                }
            }
            if mult(&self.source, self.pos) && added != 0 {
                self.pos = before;
                break;
            }
            self.program.push(c);
            added += 1;
            if mult(&self.source, self.pos) {
                break;
            }
        }
        if added == 0 {
            return Err(Error::Unavailable);
        }
        self.program.push(0);
        Ok((result, WIDTH | if added == 1 { SIMPLE } else { 0 }))
    }
}

fn instruction_size(program: &[i32], scan: usize) -> Result<usize, Error> {
    let op = *program.get(scan).ok_or(Error::Unavailable)?;
    if matches!(op, REP | REPMIN | REPX | REPXMIN) {
        return Ok(5);
    }
    if matches!(op, ANYOF | ANYBUT | EXACTLY) {
        return program[scan + 2..]
            .iter()
            .position(|&v| v == 0)
            .map(|n| n + 3)
            .ok_or(Error::Unavailable);
    }
    Ok(2)
}
struct Execution<'a> {
    regex: &'a mut Regex,
    source: &'a [u8],
    input: usize,
    bol: usize,
    matches: Vec<Match>,
    notbol: bool,
    depth: usize,
}
impl Execution<'_> {
    fn next(&self, id: usize) -> Result<usize, Error> {
        let offset = usize::try_from(*self.regex.program.get(id + 1).ok_or(Error::Unavailable)?)
            .map_err(|_| Error::Unavailable)?;
        if offset == 0 {
            return Ok(0);
        }
        if self.regex.program[id] == BACK {
            id.checked_sub(offset)
        } else {
            id.checked_add(offset)
        }
        .filter(|&id| id < self.regex.program.len())
        .ok_or(Error::Unavailable)
    }
    fn eol(&self, c: i32) -> bool {
        c == 0 || (self.regex.flags.linestop() && c == 10)
    }
    fn range(&self, mut p: usize, c: i32) -> Result<bool, Error> {
        loop {
            let length = *self.regex.program.get(p).ok_or(Error::Unavailable)?;
            if length == 0 {
                return Ok(false);
            }
            let start = self.regex.program[p + 1];
            if c >= start && c < start + length {
                return Ok(true);
            }
            p += 2;
        }
    }
    fn run(&mut self, scan: usize) -> Result<bool, Error> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::Unavailable);
        }
        self.depth += 1;
        let result = self.run_inner(scan);
        self.depth -= 1;
        result
    }
    fn run_inner(&mut self, mut scan: usize) -> Result<bool, Error> {
        while scan != 0 {
            let mut next = self.next(scan)?;
            let op = self.regex.program[scan];
            let (c, width) = decode(self.source, self.input, self.regex.flags.nocase());
            match op {
                BOLX => {
                    if self.notbol || self.input != self.bol {
                        return Ok(false);
                    }
                }
                BOL => {
                    if self.input != self.bol {
                        return Ok(false);
                    }
                }
                EOLX => {
                    if c != 0 {
                        return Ok(false);
                    }
                }
                EOL => {
                    if !self.eol(c) {
                        return Ok(false);
                    }
                }
                WORDA => {
                    if !word(c as u8)
                        || (self.input > self.bol && word(self.source[self.input - 1]))
                    {
                        return Ok(false);
                    }
                }
                WORDZ => {
                    if self.input <= self.bol
                        || (!self.eol(c) && word(c as u8))
                        || !word(self.source[self.input - 1])
                    {
                        return Ok(false);
                    }
                }
                ANY => {
                    if self.eol(c) {
                        return Ok(false);
                    }
                    self.input += width;
                }
                ANYOF | ANYBUT => {
                    if self.eol(c) || self.range(scan + 2, c)? != (op == ANYOF) {
                        return Ok(false);
                    }
                    self.input += width;
                }
                EXACTLY => {
                    let mut p = scan + 2;
                    while self.regex.program[p] != 0 {
                        if self.source.get(self.input).is_none_or(|&b| b == 0) {
                            return Ok(false);
                        }
                        let (c, width) = decode(self.source, self.input, self.regex.flags.nocase());
                        if self.regex.program[p] != c {
                            return Ok(false);
                        }
                        self.input += width;
                        p += 1;
                    }
                }
                NOTHING | BACK => {}
                BRANCH => {
                    if self.regex.program.get(next) == Some(&BRANCH) {
                        loop {
                            let save = self.input;
                            if self.run(scan + 2)? {
                                return Ok(true);
                            }
                            self.input = save;
                            scan = self.next(scan)?;
                            if scan == 0 || self.regex.program[scan] != BRANCH {
                                return Ok(false);
                            }
                        }
                    }
                    next = scan + 2;
                }
                REP | REPMIN => return self.simple_repeat(scan, op == REPMIN),
                REPX | REPXMIN => return self.complex_repeat(scan, op == REPXMIN),
                END => return Ok(true),
                OPENNC | CLOSENC => return self.run(next),
                _ if op > OPEN && op < CLOSE + 100 => {
                    let save = self.input;
                    if self.run(next)? {
                        let (slot, start) = if op < CLOSE {
                            ((op - OPEN) as usize, true)
                        } else {
                            ((op - CLOSE) as usize, false)
                        };
                        if let Some(m) = self.matches.get_mut(slot) {
                            let edge = if start { &mut m.start } else { &mut m.end };
                            if edge.is_none() {
                                *edge = Some(save);
                            }
                        }
                        return Ok(true);
                    }
                    self.input = save;
                    return Ok(false);
                }
                _ => return Err(Error::Unavailable),
            }
            scan = next;
        }
        Err(Error::Unavailable)
    }
    fn simple_repeat(&mut self, scan: usize, minimal: bool) -> Result<bool, Error> {
        let max = usize::try_from(self.regex.program[scan + 2]).map_err(|_| Error::Unavailable)?;
        let min = usize::try_from(self.regex.program[scan + 3]).map_err(|_| Error::Unavailable)?;
        let next = self.next(scan)?;
        let atom = scan + 5;
        let op = self.regex.program[atom];
        let save = self.input;
        let mut positions = vec![save];
        for _ in 0..max {
            let (c, width) = decode(self.source, self.input, self.regex.flags.nocase());
            let matches = match op {
                ANY => !self.eol(c),
                EXACTLY => self.regex.program[atom + 2] == c,
                ANYOF => !self.eol(c) && self.range(atom + 2, c)?,
                ANYBUT => !self.eol(c) && !self.range(atom + 2, c)?,
                _ => return Err(Error::Unavailable),
            };
            if !matches {
                break;
            }
            let width = if op == ANY {
                tcl_syntax::raw_string::RawString::from_bytes(&self.source[self.input..])
                    .jim084_byte_offset(1)
                    .map_err(|_| Error::Unavailable)?
            } else {
                width
            };
            self.input = self
                .input
                .checked_add(width)
                .filter(|&v| v <= self.source.len())
                .ok_or(Error::Unavailable)?;
            positions.push(self.input);
        }
        if positions.len() <= min {
            return Ok(false);
        }
        let last = positions.len() - 1;
        let mut count = if minimal { min } else { last };
        loop {
            self.input = save
                + tcl_syntax::raw_string::RawString::from_bytes(&self.source[save..])
                    .jim084_byte_offset(count)
                    .map_err(|_| Error::Unavailable)?;
            if self.run(next)? {
                return Ok(true);
            }
            if minimal {
                if count == last {
                    break;
                }
                count += 1;
            } else {
                if count == min {
                    break;
                }
                count -= 1;
            }
        }
        Ok(false)
    }
    fn complex_repeat(&mut self, scan: usize, minimal: bool) -> Result<bool, Error> {
        let (max, min, count) = (
            self.regex.program[scan + 2],
            self.regex.program[scan + 3],
            self.regex.program[scan + 4],
        );
        if count < min {
            self.regex.program[scan + 4] += 1;
            if self.run(scan + 5)? {
                return Ok(true);
            }
            self.regex.program[scan + 4] -= 1;
            return Ok(false);
        }
        if count > max {
            return Ok(false);
        }
        let next = self.next(scan)?;
        if minimal && self.run(next)? {
            return Ok(true);
        }
        if minimal || count < max {
            self.regex.program[scan + 4] += 1;
            if self.run(scan + 5)? {
                return Ok(true);
            }
            self.regex.program[scan + 4] -= 1;
        }
        if minimal { Ok(false) } else { self.run(next) }
    }
}
fn word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hex(source: &str) -> Vec<u8> {
        source
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn original_jim_programs_and_ranges_match_all_150_native_controls() {
        let subjects: [&[u8]; 8] = [
            b"a",
            b"\xff",
            b"\xc0\x80",
            b"\xf0\x9f\x98\x80",
            b"aaab abc foo 12\n",
            b"aba",
            b"abbbc",
            b"xyz",
        ];
        let mut program: Option<Regex> = None;
        let mut compile_code = 0;
        let mut context = String::new();
        let mut cases = 0;
        let mut ranges = 0;
        for line in include_str!("../tests/data/native_jim_regexp/programs.txt").lines() {
            let fields: Vec<_> = line.split('\t').collect();
            match fields[0] {
                "case" => {
                    cases += 1;
                    context = format!("case {} flags {}", fields[1], fields[2]);
                    let key: u32 = fields[2].parse().unwrap();
                    let result = Regex::compile(
                        &hex(fields[3]),
                        Flags::new()
                            .with_nocase(key & 2 != 0)
                            .with_lineanchor(key & 4 != 0)
                            .with_linestop(key & 8 != 0)
                            .with_expanded(key & 32 != 0),
                    );
                    program = match result {
                        Ok(re) => {
                            compile_code = 0;
                            Some(re)
                        }
                        Err(Error::Syntax(code)) => {
                            compile_code = code;
                            None
                        }
                        Err(error) => panic!("{context}: {error:?}"),
                    };
                }
                "compile" => {
                    let code: u8 = fields[1].parse().unwrap();
                    assert_eq!(compile_code, code, "{context}: {line}");
                    if let Some(re) = &program {
                        assert_eq!(re.nsub(), fields[2].parse::<usize>().unwrap(), "{context}");
                    }
                }
                "program" => {
                    let expected: Vec<i32> = fields[1]
                        .split(',')
                        .map(|word| word.parse().unwrap())
                        .collect();
                    assert_eq!(program.as_ref().unwrap().program(), expected, "{context}");
                }
                "match" => {
                    ranges += 1;
                    let subject = subjects[fields[1].parse::<usize>().unwrap()];
                    let actual = program
                        .as_mut()
                        .unwrap()
                        .execute(subject, 4, false)
                        .unwrap_or_else(|error| panic!("{context}: {line}: {error:?}"));
                    assert_eq!(actual.is_some(), fields[2] == "0", "{context}: {line}");
                    if let Some(actual) = actual {
                        let expected: Vec<Match> = fields[3..]
                            .iter()
                            .map(|field| {
                                let (a, b) = field.split_once(',').unwrap();
                                Match {
                                    start: a.parse::<usize>().ok(),
                                    end: b.parse::<usize>().ok(),
                                }
                            })
                            .collect();
                        assert_eq!(actual, expected, "{context}: {line}");
                    }
                }
                _ => panic!("unexpected native record {line}"),
            }
        }
        assert_eq!(cases, 150);
        assert!(ranges > 800);
    }
}
