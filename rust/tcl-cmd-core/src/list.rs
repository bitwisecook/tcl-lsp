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

//! Portable `list`-family command logic, generic over [`ValueOps`].
//!
//! The pure list operations — those that read/build list values without touching
//! interpreter variables — and the new list a list cell update (`lset`, `ledit`,
//! `lpop`) writes back, which each runtime reads from and stores into its own
//! variable. The other variable-mutating members (`lappend`, `lassign`) stay
//! per-runtime over the Family-B var store. Indices go through the shared
//! [`crate::index`] parser, so a malformed index spec raises `bad index` rather
//! than quietly yielding the empty string.
//!
//! [`ValueOps`]: tcl_syntax::value::ValueOps

use tcl_dialect::TclVersion;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;
use crate::index;

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `list ?arg ...?` — a list of the arguments verbatim.
pub fn list<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> O::Value {
    ops.new_list(args.to_vec())
}

/// `llength list`.
pub fn llength<O: ValueOps>(ops: &mut O, value: &O::Value) -> Result<O::Value, CmdError> {
    let n = ops.list_len(value)?;
    Ok(ops.new_int(ilen(n)))
}

/// `lindex list ?index ...?` — descend through each index; an out-of-range index
/// yields the empty string, a malformed index spec errors.
///
/// A lone index argument is itself split into an index *path*
/// (`lindex {{a b} c} {0 1}` → `b`); multiple arguments are each a single index.
/// Mirrors `Tcl_LindexObjCmd` (`TclLindexList`/`TclLindexFlat`).
pub fn lindex<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    idxs: &[O::Value],
) -> Result<O::Value, CmdError> {
    let [only] = idxs else {
        return lindex_flat(ops, value, idxs);
    };
    let s = ops.as_str(only);
    // A single index argument is an index *list* (`lindex $l {0 1}`). When it
    // is not a well-formed list (`lindex $l \{`), C falls back to treating it
    // as one index spec — which then fails as `bad index "{"`, not as a list
    // parse error (lindex-10.4).
    let specs: Vec<String> = tcl_syntax::list::split_list(&s).map_or_else(
        |_| vec![s.to_string()],
        |parts| parts.iter().map(|p| p.as_ref().to_string()).collect(),
    );
    lindex_specs(ops, value, &specs)
}

/// `TclLindexFlat` — `lindex` with every index already flattened, so each
/// element of `idxs` is exactly *one* index spec and is never re-split as an
/// index list. This is the core `INST_LIST_INDEX_MULTI` uses
/// (`tclExecute.c:4833-4858`) and the `lindex list i1 i2 …` argument form; the
/// `objc == 3` form goes through [`lindex`] (C's `TclLindexList`) instead.
///
/// # Errors
/// `bad index …` for a malformed spec, or the list-parse error of a level that
/// is not a well-formed list.
pub fn lindex_flat<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    idxs: &[O::Value],
) -> Result<O::Value, CmdError> {
    let specs: Vec<String> = idxs.iter().map(|i| ops.as_str(i).to_string()).collect();
    lindex_specs(ops, value, &specs)
}

/// Walk `specs` one level per index — the shared body of [`lindex`] and
/// [`lindex_flat`] (C's `TclLindexFlat` loop). An empty `specs` returns `value`
/// unchanged, which is how `lindex $l {}` yields the whole list.
fn lindex_specs<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    specs: &[String],
) -> Result<O::Value, CmdError> {
    let mut cur = value.clone();
    for (k, spec) in specs.iter().enumerate() {
        let elems = ops.list_elements(&cur)?;
        let i = index::resolve(spec, elems.len())?;
        if let Some(i) = usize::try_from(i).ok().filter(|&i| i < elems.len()) {
            cur = elems[i].clone();
        } else {
            // Out of range yields the empty result, but a *malformed* later
            // index is still an error — C parses every index before navigating,
            // so `lindex {} end foo` reports `bad index "foo"` (lindex-17.0).
            // The format check is length-independent.
            for rest in &specs[k + 1..] {
                index::resolve(rest, 0)?;
            }
            return Ok(ops.empty());
        }
    }
    Ok(cur)
}

/// `lrange list first last` — the inclusive sublist, clamped.
pub fn lrange<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    first: &O::Value,
    last: &O::Value,
) -> Result<O::Value, CmdError> {
    let elems = ops.list_elements(value)?;
    let len = elems.len();
    let lo = index::resolve(&ops.as_str(first), len)?.max(0);
    let hi = index::resolve(&ops.as_str(last), len)?;
    let Ok(lo) = usize::try_from(lo) else {
        return Ok(ops.new_list(Vec::new()));
    };
    if hi < 0 || lo >= len {
        return Ok(ops.new_list(Vec::new()));
    }
    let hi = usize::try_from(hi).unwrap_or(usize::MAX).min(len - 1);
    if lo > hi {
        return Ok(ops.new_list(Vec::new()));
    }
    Ok(ops.new_list(elems[lo..=hi].to_vec()))
}

/// `lreverse list`.
pub fn lreverse<O: ValueOps>(ops: &mut O, value: &O::Value) -> Result<O::Value, CmdError> {
    let mut elems = ops.list_elements(value)?;
    elems.reverse();
    Ok(ops.new_list(elems))
}

/// `lrepeat count ?value ...?` — `count` copies of the value sequence.
pub fn lrepeat<O: ValueOps>(
    ops: &mut O,
    count: &O::Value,
    values: &[O::Value],
) -> Result<O::Value, CmdError> {
    let n = ops.as_int(count)?;
    if n < 0 {
        return Err(CmdError::new(format!(
            "bad count \"{n}\": must be integer >= 0"
        )));
    }
    let n = usize::try_from(n).unwrap_or(0);
    let mut out = Vec::with_capacity(n.saturating_mul(values.len()));
    for _ in 0..n {
        out.extend(values.iter().cloned());
    }
    Ok(ops.new_list(out))
}

/// `linsert list index ?element ...?` — insert `elements` before `index`.
///
/// `index` is an **insertion index**: unlike `lindex`/`lrange`, `end` names the
/// position *after* the last element (`len`, not `len-1`) — so `linsert {a b}
/// end c` appends. Resolving against `len + 1` gives `end`/`end±N` that offset.
/// A bad index spec errors faithfully; an out-of-range result clamps to
/// `[0, len]`.
pub fn linsert<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    index: &O::Value,
    elements: &[O::Value],
) -> Result<O::Value, CmdError> {
    let mut elems = ops.list_elements(value)?;
    let len = elems.len();
    let at = index::resolve(&ops.as_str(index), len + 1)?;
    let at = usize::try_from(at.max(0)).unwrap_or(0).min(len);
    for (k, e) in elements.iter().enumerate() {
        elems.insert(at + k, e.clone());
    }
    Ok(ops.new_list(elems))
}

/// `lreplace list first last ?element ...?` — replace the inclusive range
/// `first..last` with `elements`.
///
/// `first`/`last` are element indices (`end` = `len-1`), clamped to the list;
/// `last < first` (or both past the end) makes this a pure insertion at `first`.
/// A bad index spec errors faithfully.
pub fn lreplace<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    first: &O::Value,
    last: &O::Value,
    elements: &[O::Value],
) -> Result<O::Value, CmdError> {
    let mut elems = ops.list_elements(value)?;
    let len = elems.len();
    let first = index::resolve(&ops.as_str(first), len)?;
    let last = index::resolve(&ops.as_str(last), len)?;
    let (lo, end) = replace_range(first, last, len);
    elems.splice(lo..end, elements.iter().cloned());
    Ok(ops.new_list(elems))
}

/// The `bad index` error `release` raises for `spec`: 8.4 words the grammar
/// it reads (`integer or end?-integer?`), 8.5 on the sums too, and from 8.6
/// the error carries `TCL VALUE INDEX` (measured, 8.4.20 to 9.1.0).
fn bad_index_under(spec: &str, release: TclVersion) -> CmdError {
    if release < TclVersion::V8_5 {
        return CmdError::new(format!(
            "bad index \"{spec}\": must be integer or end?-integer?"
        ));
    }
    let message = format!("bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?");
    if release < TclVersion::V8_6 {
        CmdError::new(message)
    } else {
        CmdError::with_error_code(message, "TCL VALUE INDEX")
    }
}

/// `spec` read as one index of a level of `len` elements under `release`.
fn index_under(spec: &str, len: usize, release: TclVersion) -> Result<i64, CmdError> {
    index::resolve_opt_with(spec, len, release.number_syntax())
        .ok_or_else(|| bad_index_under(spec, release))
}

/// The element `spec` names at a level of `len` elements for `lset`, which
/// from 8.6 may also name `len` itself, the element it appends; anything else
/// is out of range, worded as `release` words it.
fn lset_level(spec: &str, len: usize, release: TclVersion) -> Result<usize, CmdError> {
    let index = index_under(spec, len, release)?;
    let limit = if release >= TclVersion::V8_6 {
        len + 1
    } else {
        len
    };
    match usize::try_from(index) {
        Ok(index) if index < limit => Ok(index),
        _ if release >= TclVersion::V9_0 => Err(CmdError::with_error_code(
            format!("index \"{spec}\" out of range"),
            "TCL VALUE INDEX OUTOFRANGE",
        )),
        _ if release >= TclVersion::V8_6 => Err(CmdError::with_error_code(
            "list index out of range",
            "TCL OPERATION LSET BADINDEX",
        )),
        _ => Err(CmdError::new("list index out of range")),
    }
}

/// `lset listVar ?index …? newValue`'s new list (`TclLsetList` and
/// `TclLsetFlat`) as `release` computes it: with no index, `value` itself;
/// with one index argument, that word as one index where it reads as one,
/// otherwise the indices it lists (none again being `value`), otherwise the
/// word as one index after all, which then fails; with several, each one
/// index. Each index descends a level, read against that level's length, a
/// level that is not a list being its parse error; from 8.6 an index equal to
/// a level's length appends there, a deeper index descending into a new empty
/// list (`lset x 3 0 v` on `a {b1 b2} c` raises before 8.6 and gives
/// `a {b1 b2} c v` after).
///
/// # Errors
///
/// A level that is not a list, a bad index, or an index out of range, as
/// `release` words each.
pub fn lset<O: ValueOps>(
    ops: &mut O,
    list: &O::Value,
    indices: &[O::Value],
    value: O::Value,
    release: TclVersion,
) -> Result<O::Value, CmdError> {
    let path: Vec<String> = match indices {
        [] => Vec::new(),
        [only] => {
            let text = ops.as_str(only).to_string();
            if index::resolve_opt_with(&text, 0, release.number_syntax()).is_some() {
                vec![text]
            } else {
                match tcl_syntax::list::split_list(&text) {
                    Ok(parts) => parts.iter().map(|part| part.as_ref().to_owned()).collect(),
                    Err(_) => vec![text],
                }
            }
        }
        many => many
            .iter()
            .map(|spec| ops.as_str(spec).to_string())
            .collect(),
    };
    // Walk down without recursing, so a long path costs no native stack; a
    // level past an appended element is empty, and builds no value of its own.
    let mut levels: Vec<(Vec<O::Value>, usize)> = Vec::with_capacity(path.len());
    let mut current = Some(list.clone());
    for spec in &path {
        let elements = match &current {
            Some(level) => ops.list_elements(level)?,
            None => Vec::new(),
        };
        let at = lset_level(spec, elements.len(), release)?;
        current = elements.get(at).cloned();
        levels.push((elements, at));
    }
    let mut rebuilt = value;
    for (mut elements, at) in levels.into_iter().rev() {
        if at == elements.len() {
            elements.push(rebuilt);
        } else {
            elements[at] = rebuilt;
        }
        rebuilt = ops.new_list(elements);
    }
    Ok(rebuilt)
}

/// `lpop listVar ?index …?`'s removed element and the list without it (from
/// 9.0): with no index the last element; otherwise each word one index — a
/// word that lists several is a bad index, not a path — descending a level
/// each, the last naming the element removed, every one in range.
///
/// # Errors
///
/// A level that is not a list, a bad index, or `index "X" out of range`.
pub fn lpop<O: ValueOps>(
    ops: &mut O,
    list: &O::Value,
    indices: &[O::Value],
    release: TclVersion,
) -> Result<(O::Value, O::Value), CmdError> {
    let path: Vec<String> = if indices.is_empty() {
        vec!["end".to_owned()]
    } else {
        indices
            .iter()
            .map(|spec| ops.as_str(spec).to_string())
            .collect()
    };
    let mut levels: Vec<(Vec<O::Value>, usize)> = Vec::with_capacity(path.len());
    let mut current = list.clone();
    for spec in &path {
        let elements = ops.list_elements(&current)?;
        let index = index_under(spec, elements.len(), release)?;
        let at = usize::try_from(index)
            .ok()
            .filter(|&at| at < elements.len())
            .ok_or_else(|| {
                CmdError::with_error_code(
                    format!("index \"{spec}\" out of range"),
                    "TCL VALUE INDEX OUTOFRANGE",
                )
            })?;
        current = elements[at].clone();
        levels.push((elements, at));
    }
    let (mut innermost, at) = levels.pop().expect("the path has at least one index");
    let removed = innermost.remove(at);
    let mut rebuilt = ops.new_list(innermost);
    for (mut elements, at) in levels.into_iter().rev() {
        elements[at] = rebuilt;
        rebuilt = ops.new_list(elements);
    }
    Ok((removed, rebuilt))
}

/// `ledit listVar first last ?element …?`'s new list (from 9.0): the
/// [`lreplace`] of the list it holds, its indices read under `release`.
///
/// # Errors
///
/// The list's parse error, or a bad index.
pub fn ledit<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    first: &O::Value,
    last: &O::Value,
    elements: &[O::Value],
    release: TclVersion,
) -> Result<O::Value, CmdError> {
    let mut items = ops.list_elements(value)?;
    let len = items.len();
    let first = index_under(&ops.as_str(first), len, release)?;
    let last = index_under(&ops.as_str(last), len, release)?;
    let (lo, end) = replace_range(first, last, len);
    items.splice(lo..end, elements.iter().cloned());
    Ok(ops.new_list(items))
}

/// The `lo..end` element range `lreplace` and `ledit` replace for `first` and
/// `last` over `len` elements: clamped to the list, and empty — an insertion
/// at `first` — where `last` is before it.
fn replace_range(first: i64, last: i64, len: usize) -> (usize, usize) {
    let lo = usize::try_from(first.max(0)).unwrap_or(0).min(len);
    let end = if last < 0 {
        lo
    } else {
        usize::try_from(last)
            .unwrap_or(0)
            .saturating_add(1)
            .clamp(lo, len)
    };
    (lo, end)
}

/// The ASCII whitespace Tcl trims/splits on (space, tab, newline, CR, VT, FF).
const TCL_WS: &[char] = &[' ', '\t', '\n', '\r', '\u{0b}', '\u{0c}'];

/// `concat ?arg ...?` — trim each argument and join with single spaces, dropping
/// the args that are empty after trimming.
pub fn concat<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> O::Value {
    let mut parts: Vec<String> = Vec::new();
    for v in args {
        let s = ops.as_str(v);
        let t = trim_concat_element(&s);
        if !t.is_empty() {
            parts.push(t.to_string());
        }
    }
    ops.new_string(parts.join(" "))
}

/// Trim leading/trailing whitespace from one `concat` element, matching C's
/// `Tcl_ConcatObj` (`TclTrimLeft`/`TclTrimRight`). The right trim is
/// backslash-aware: a trailing whitespace byte escaped by an odd run of
/// backslashes (`\ `) is part of the element and kept, so `concat "a\ " b`
/// yields `a\  b` rather than `a b` (lreplace.test ledit-1.25). A leading
/// whitespace byte can never be escaped (the escaping `\` would precede it), so
/// the left trim is plain. Shared with the VM's inline `concatStk` opcode.
#[must_use]
pub fn trim_concat_element(s: &str) -> &str {
    let bytes = s.as_bytes();
    let is_ws = |b: u8| TCL_WS.contains(&(b as char));
    let mut start = 0;
    while start < bytes.len() && is_ws(bytes[start]) {
        start += 1;
    }
    let mut end = bytes.len();
    while end > start && is_ws(bytes[end - 1]) {
        let backslashes = bytes[start..end - 1]
            .iter()
            .rev()
            .take_while(|&&b| b == b'\\')
            .count();
        if backslashes % 2 == 1 {
            break; // escaped whitespace: part of the element.
        }
        end -= 1;
    }
    &s[start..end]
}

/// `join list ?joinString?` (default separator a single space).
pub fn join<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    sep: Option<&O::Value>,
) -> Result<O::Value, CmdError> {
    let sep = sep.map_or_else(|| " ".to_string(), |s| ops.as_str(s).to_string());
    let elems = ops.list_elements(value)?;
    let mut parts: Vec<String> = Vec::with_capacity(elems.len());
    for e in &elems {
        parts.push(ops.as_str(e).to_string());
    }
    Ok(ops.new_string(parts.join(&sep)))
}

/// The characters `split` splits on by default: space, newline, tab and
/// carriage return, `Tcl_SplitObjCmd`'s `" \n\t\r"` — not the vertical tab or
/// the form feed, which [`TCL_WS`] holds for `concat` (tclsh 8.4.20 to 9.1.0
/// split `"a\vb\fc d"` into two elements).
const SPLIT_WS: &[char] = &[' ', '\n', '\t', '\r'];

/// `split string ?splitChars?` — split into a list. The default split set is
/// [`SPLIT_WS`]; an empty split set splits into individual characters.
pub fn split<O: ValueOps>(ops: &mut O, value: &O::Value, chars: Option<&O::Value>) -> O::Value {
    let string = ops.as_str(value).to_string();
    let set = chars.map(|c| ops.as_str(c).to_string());
    let pieces: Vec<String> = if string.is_empty() {
        // `split ""` is the empty list (not a single empty element).
        Vec::new()
    } else if set.as_deref() == Some("") {
        // An empty split set makes each character its own element.
        string.chars().map(|c| c.to_string()).collect()
    } else {
        let set: Vec<char> = set.map_or_else(|| SPLIT_WS.to_vec(), |c| c.chars().collect());
        string
            .split(|c| set.contains(&c))
            .map(str::to_string)
            .collect()
    };
    let mut values = Vec::with_capacity(pieces.len());
    for p in pieces {
        values.push(ops.new_string(p));
    }
    ops.new_list(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text values whose lists render through the shared `Tcl_Merge` owner,
    /// as 8.5 to 9.1 render them.
    struct TextOps;

    impl ValueOps for TextOps {
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
            tcl_syntax::list::join_list(items.iter().map(String::as_str))
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
            tcl_syntax::list::split_list(v)
                .map(|elements| elements.iter().map(ToString::to_string).collect())
                .map_err(|error| tcl_syntax::value::ValueError::BadList(error.message().to_owned()))
        }
    }

    /// Each `split` as tclsh 8.5.19 to 9.1.0 give it (8.4.20 too, which
    /// renders a leading `#` unquoted).
    #[test]
    fn split_splits_as_tclsh_splits() {
        let cases: &[(&str, Option<&str>, &str)] = &[
            ("a b c", None, "a b c"),
            (" a  b ", None, "{} a {} b {}"),
            ("a\tb\nc\rd", None, "a b c d"),
            ("a\u{b}b\u{c}c d", None, "{a\u{b}b\u{c}c} d"),
            ("", None, ""),
            (" ", None, "{} {}"),
            ("a,b,,c", None, "a,b,,c"),
            ("{a} b", None, "{{a}} b"),
            ("a\\ b", None, "a\\\\ b"),
            ("\"x\" y", None, "{\"x\"} y"),
            ("#a b", None, "{#a} b"),
            ("a,b,,c", Some(","), "a b {} c"),
            ("abc", Some(""), "a b c"),
            ("a.b-c", Some(".-"), "a b c"),
            ("a  b", Some(" "), "a {} b"),
            ("", Some(","), ""),
            ("a,b", Some(""), "a , b"),
            (",a,", Some(","), "{} a {}"),
            ("abc", Some("abc"), "{} {} {} {}"),
            ("a{b}c", Some("{}"), "a b c"),
        ];
        for &(string, chars, expected) in cases {
            let chars = chars.map(str::to_owned);
            let got = split(&mut TextOps, &string.to_owned(), chars.as_ref());
            assert_eq!(got, expected, "split {string:?} {chars:?}");
        }
    }
}
