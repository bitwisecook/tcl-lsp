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

//! Literal command evaluation under an explicitly supplied semantic context.
//! The registry supplies policy and evidence; this module owns only value
//! algorithms and conservative agreement across unselected release policies.

pub use crate::list::clamp_range;

/// The selected operand grammar and native result producer for literal values.
#[derive(Debug, Clone, Copy)]
pub struct LiteralContext {
    /// Numeral grammar of an index or repetition count.
    pub numbers: tcl_dialect::NumberSyntax,
    /// List operand grammar.
    pub lists: tcl_dialect::ListParse,
    /// Backslash grammar while decoding list values.
    pub escapes: tcl_dialect::EscapeSyntax,
    /// Native string/result semantics, independently of parsing.
    pub protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
}

impl LiteralContext {
    fn read_list(
        self,
        text: &str,
    ) -> Result<Vec<std::borrow::Cow<'_, str>>, tcl_syntax::list::ListError> {
        tcl_syntax::list::split_list_with_syntax(text, self.lists, self.escapes)
    }

    const fn native_string_protocol(
        self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        self.protocol
    }

    fn list_result_serialization(
        self,
    ) -> Option<tcl_syntax::list_result::NativeListResultSerialization> {
        self.protocol
            .map(tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol)
    }
}

/// Which half of each dictionary pair is selected by a projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionaryMember {
    /// Dictionary keys.
    Keys,
    /// Dictionary values.
    Values,
}

/// Common list elements for a literal with no target. Source escapes need the
/// selected receiver. Strict Tcl errors and Jim's more permissive parse must
/// agree before returning a value.
#[must_use]
pub fn split_list(s: &str) -> Option<Vec<String>> {
    if s.contains('\\') {
        return None;
    }
    let parsed = tcl_syntax::list::split_list(s).ok()?;
    if parsed != tcl_syntax::list::split_list_jim(s) {
        return None;
    }
    Some(
        parsed
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .collect(),
    )
}

// One element is quoted by the shared `Tcl_ScanElement`+`Tcl_ConvertElement`
// owner, `tcl_syntax::list::list_element`, called directly where a lone element
// is rendered. It quotes a leading `#` because it renders *list position 0*;
// a whole list goes through [`list_join`] instead, which applies the rule
// position by position.

/// Join already-split elements into a Tcl list string — the shared `Tcl_Merge`
/// owner [`tcl_syntax::list::join_list`], not a per-element loop over
/// [`tcl_syntax::list::list_element`].
///
/// The difference is the comment-safety rule: `Tcl_ConvertElement` brace-quotes
/// a leading `#` only in **list position 0**, because that is the only place a
/// `#` could start a comment when the list is evaluated as a script.
/// The single-element quoter always quotes it (it renders a *single* element,
/// so it has to assume position 0), and mapping it over every element made every fold
/// that re-renders a list disagree with C Tcl on any non-first `#`:
/// `[dict keys {a 1 # 2}]` folded to `a {#}` where both tclsh oracles print
/// `a #`, and likewise for `list`, `lrange`, `lreverse`, `lrepeat`, `split`
/// and `dict create`/`values`/`merge`. Same elements, different *string* — and
/// a fold bakes the string into the program — caught by
/// the `dict_canonicalisation_parity` gate.
#[must_use]
pub fn list_join<S: AsRef<str>>(elems: &[S]) -> String {
    tcl_syntax::list::join_list(elems)
}

/// Parse a Tcl index expression against a string/list of `length`, returning the
/// resolved index (which may be negative or `>= length` — the caller clamps).
///
/// Delegates to the shared [`crate::index`] grammar — the *same* parser
/// the runtime's `lindex` / `lrange` / `string index` use — so the optimiser
/// folds the forms Tcl resolves at run time: `end`, `end±N`, the arithmetic
/// operands (`1+1`, `0-1`, `end--1`), and every integer radix (`0x2`, `0o7`,
/// `0b101`).
///
/// An index word is read by `Tcl_GetIntForIndex`, so it inherits every version
/// difference in the numeral grammar and the integer range — `lindex $l 010` is
/// index 8 up to 8.6 and 10 from 9.0, `lindex $l 4294967295` the last element
/// up to 8.6 and past it from 9.0. These folds are registered as plain
/// `ConstFoldFn`s, which carry no release, so this
/// resolves under **every** C release grammar and the Jim grammar, and folds
/// only when they agree.
/// Portable readings and literal/value container outcomes must also agree;
/// a host-width-dependent reading remains unavailable.
///
/// Declining is free: an unfolded `lindex` is evaluated at run time by an
/// interpreter that does know its release. Folding under one release's grammar
/// would instead bake a wrong constant into a program built for another — the
/// one outcome a const-folder must never produce. (When these folds migrate to
/// `VersionedConstFoldFn`, the release can
/// be named and `index::resolve_opt_in` used directly.)
#[must_use]
pub fn parse_index(s: &str, length: usize) -> Option<i64> {
    parse_index_consensus(s, length, |index| index)
}

/// A proved container selection, distinct from unproved native parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeIndexSelection<T> {
    /// Every selected native policy chooses this element or clamped range.
    Selected(T),
    /// Every selected native policy produces an empty container selection.
    Empty,
}

impl<T> NativeIndexSelection<T> {
    fn from_selection(selection: Option<T>) -> Self {
        selection.map_or(Self::Empty, Self::Selected)
    }
}

/// Selected element under every portable native index policy. Absence of the
/// proof means parsing or native outcome agreement remains unproved.
#[must_use]
pub fn parse_element_index(s: &str, length: usize) -> Option<NativeIndexSelection<usize>> {
    parse_index_consensus(s, length, |index| {
        NativeIndexSelection::from_selection(
            usize::try_from(index).ok().filter(|&index| index < length),
        )
    })
}

/// Clamped range selected unanimously by the native index policies.
/// Compare the container outcome rather than the engines' raw index encodings.
#[must_use]
pub fn parse_range(
    first: &str,
    last: &str,
    length: usize,
) -> Option<NativeIndexSelection<(usize, usize)>> {
    index_result_consensus(|syntax, release| {
        let first_value = portable_index(first, length, syntax, release)?;
        let last_value = portable_index(last, length, syntax, release)?;
        let selected =
            NativeIndexSelection::from_selection(clamp_range(first_value, last_value, length));
        let first_compiled =
            alternate_compiled_index(first, length, release).unwrap_or(first_value);
        let last_compiled = alternate_compiled_index(last, length, release).unwrap_or(last_value);
        for (first, last) in [
            (first_compiled, last_value),
            (first_value, last_compiled),
            (first_compiled, last_compiled),
        ] {
            if NativeIndexSelection::from_selection(clamp_range(first, last, length)) != selected {
                return None;
            }
        }
        Some(selected)
    })
}

/// Compare the result needed by a container operation. Native encodings may
/// differ while every engine selects the same element or out-of-range result.
fn parse_index_consensus<T: PartialEq>(
    s: &str,
    length: usize,
    project: impl Fn(i64) -> T,
) -> Option<T> {
    index_result_consensus(|syntax, release| {
        let value = portable_index(s, length, syntax, release)?;
        let selected = project(value);
        if alternate_compiled_index(s, length, release)
            .is_some_and(|value| project(value) != selected)
        {
            return None;
        }
        Some(selected)
    })
}

fn index_result_consensus<T: PartialEq>(
    evaluate: impl Fn(tcl_dialect::IndexSyntax, Option<tcl_dialect::TclVersion>) -> Option<T>,
) -> Option<T> {
    let mut answers = tcl_dialect::TclVersion::ALL.iter().map(|&version| {
        evaluate(
            tcl_dialect::IndexSyntax::for_version(version),
            Some(version),
        )
    });
    let first = answers.next()?;
    if !answers.all(|answer| answer == first) {
        return None;
    }
    let jim = tcl_dialect::IndexSyntax {
        numbers: tcl_dialect::NumberSyntax::Jim080,
        grammar: tcl_dialect::IndexGrammar::Jim,
        width: tcl_dialect::IndexIntegerWidth::Jim32,
        end_abbreviations: false,
    };
    (evaluate(jim, None) == first).then_some(first).flatten()
}

// Pure original index reading is not object/cache or execution admission.
// A portable fold never selects the build host's C long width.
fn portable_index(
    spec: &str,
    length: usize,
    syntax: tcl_dialect::IndexSyntax,
    release: Option<tcl_dialect::TclVersion>,
) -> Option<i64> {
    let Some(release) = release else {
        return crate::index::resolve_opt_in(spec, length, syntax);
    };
    match crate::index::read_under(spec, length, release) {
        crate::index::IndexReading::At(value) => Some(value),
        _ => None,
    }
}

// C8.6's positive end-offset int overflow is clamped after the end by its
// literal encoder, but wraps when the original runtime word is parsed.
fn alternate_compiled_index(
    spec: &str,
    length: usize,
    release: Option<tcl_dialect::TclVersion>,
) -> Option<i64> {
    let release = release?;
    crate::index::compiles_apart(spec, length, release)
        .then(|| i64::try_from(length).ok())
        .flatten()
}

// list commands

/// `concat ?arg ...?` — materialise the shared native concatenation grammar.
#[must_use]
pub fn fold_concat(args: &[&str]) -> String {
    tcl_syntax::list::concat_values(args.iter().copied())
}

/// Portable `string reverse`: the supported character models agree on ASCII.
#[must_use]
pub fn fold_string_reverse(args: &[&str]) -> Option<String> {
    let [value] = args else {
        return None;
    };
    value.is_ascii().then_some(())?;
    crate::string::reverse(&mut LiteralOps, &(*value).to_owned()).ok()
}

/// Portable `string map` over ASCII text and a release-independent map list.
#[must_use]
pub fn fold_string_map(args: &[&str]) -> Option<String> {
    match args {
        [map, text] | ["-nocase", map, text] if map.is_ascii() && text.is_ascii() => {}
        _ => return None,
    }
    let values = args
        .iter()
        .map(|word| (*word).to_owned())
        .collect::<Vec<_>>();
    crate::string::map(&mut LiteralOps, &values).ok()
}

/// Build a canonical Tcl list through the same kernel as the runtimes.
#[must_use]
pub fn fold_list(args: &[&str]) -> String {
    crate::list::list(
        &mut LiteralOps,
        &args
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
    )
}

/// `llength list`.
#[must_use]
pub fn fold_llength(args: &[&str]) -> Option<String> {
    let [l] = args else {
        return None;
    };
    crate::list::llength(&mut LiteralOps, &(*l).to_owned()).ok()
}

fn read_literal_list(text: &str, dialect: Option<LiteralContext>) -> Option<Vec<String>> {
    match dialect {
        Some(dialect) => Some(
            dialect
                .read_list(text)
                .ok()?
                .into_iter()
                .map(std::borrow::Cow::into_owned)
                .collect(),
        ),
        None => split_list(text),
    }
}

fn render_literal_list(words: &[String], dialect: Option<LiteralContext>) -> Option<String> {
    let bytes = match dialect {
        Some(dialect) => dialect.list_result_serialization()?.render(words),
        None => tcl_syntax::list_result::portable_list_result(words)?,
    };
    String::from_utf8(bytes).ok()
}

/// `lreverse list`.
#[must_use]
pub fn fold_lreverse(args: &[&str]) -> Option<String> {
    fold_lreverse_in(args, None)
}

#[must_use]
pub fn fold_lreverse_in(args: &[&str], dialect: Option<LiteralContext>) -> Option<String> {
    let [list] = args else {
        return None;
    };
    let mut elems = read_literal_list(list, dialect)?;
    let preserve = match dialect.and_then(LiteralContext::native_string_protocol) {
        Some(tcl_syntax::native_string::NativeStringProtocol::C(version)) => {
            Some(elems.is_empty() || (elems.len() == 1 && version == tcl_dialect::TclVersion::V9_1))
        }
        Some(tcl_syntax::native_string::NativeStringProtocol::Jim084) => Some(false),
        None => None,
    };
    if preserve == Some(true) {
        return Some((*list).to_owned());
    }
    elems.reverse();
    let result = render_literal_list(&elems, dialect)?;
    (preserve.is_some() || elems.len() > 1 || result == *list).then_some(result)
}

/// ASCII byte order agrees across the supported engines. Non-ASCII ordering
/// depends on the native encoding (notably C Tcl 8.6's supplementary units).
#[must_use]
pub fn fold_lsort(args: &[&str]) -> Option<String> {
    fold_lsort_in(args, None)
}

#[must_use]
pub fn fold_lsort_in(args: &[&str], dialect: Option<LiteralContext>) -> Option<String> {
    let (list, options) = args.split_last()?;
    let mut decreasing = false;
    let mut unique = false;
    for option in options {
        match *option {
            "-ascii" => {}
            "-increasing" => decreasing = false,
            "-decreasing" => decreasing = true,
            "-unique" => unique = true,
            _ => return None,
        }
    }
    let mut elems = read_literal_list(list, dialect)?;
    if elems.len() <= 1 {
        match dialect.and_then(LiteralContext::native_string_protocol) {
            Some(tcl_syntax::native_string::NativeStringProtocol::Jim084) => {
                return Some((*list).to_owned());
            }
            None => {
                let common = render_literal_list(&elems, dialect)?;
                return (common == *list).then_some(common);
            }
            Some(tcl_syntax::native_string::NativeStringProtocol::C(_)) => {}
        }
    }
    if elems
        .iter()
        .any(|elem| !elem.is_ascii() || elem.contains('\0'))
    {
        return None;
    }
    elems.sort_unstable();
    if unique {
        elems.dedup();
    }
    if decreasing {
        elems.reverse();
    }
    render_literal_list(&elems, dialect)
}

/// `join list ?joinString?` — flatten elements with the separator.
#[must_use]
pub fn fold_join(args: &[&str]) -> Option<String> {
    let (l, sep) = match args {
        [l] => (*l, " "),
        [l, s] => (*l, *s),
        _ => return None,
    };
    crate::list::join(&mut LiteralOps, &l.to_owned(), Some(&sep.to_owned())).ok()
}

/// Cap on a single constant-fold's materialised output (1 MiB) — bound the
/// product (count × element bytes), not just the count, so large elements
/// can't blow up the fold.
const MAX_FOLD_OUTPUT_BYTES: usize = 1 << 20;

/// Exact bounded count read under the receiver's numeral grammar.
#[must_use]
pub fn repetition_count(word: &str, numbers: tcl_syntax::number::Numbers) -> Option<usize> {
    let tcl_syntax::number::Number::Int(value) = numbers.parse_whole(word)? else {
        return None;
    };
    usize::try_from(value).ok()
}

/// `lrepeat count ?element ...?` on every release providing the command.
#[must_use]
pub fn fold_lrepeat(args: &[&str]) -> Option<String> {
    let repeated = repeated_elements(args, tcl_syntax::number::Numbers::Unknown, false)?;
    let result = list_join(&repeated);
    // C Tcl 8.5 and later quote a leading hash. Jim additionally makes its
    // own brace/escape choice, so an unselected fold needs both spellings.
    (tcl_syntax::list_result::NativeListResultSerialization::Jim084.render(&repeated)
        == result.as_bytes())
    .then_some(result)
}

fn repeated_elements<'a>(
    args: &[&'a str],
    numbers: tcl_syntax::number::Numbers,
    accepts_empty: bool,
) -> Option<Vec<&'a str>> {
    let (word, elems) = args.split_first()?;
    let count = repetition_count(word, numbers)?;
    if count > 1000 || (!accepts_empty && (count == 0 || elems.is_empty())) {
        return None;
    }
    let elem_bytes = elems.iter().try_fold(0usize, |sum, elem| {
        sum.checked_add(elem.len().checked_add(1)?)
    })?;
    if elem_bytes.checked_mul(count)? > MAX_FOLD_OUTPUT_BYTES {
        return None;
    }
    Some(elems.repeat(count))
}

/// A selected native `lrepeat` uses its count grammar, arity and list writer.
#[must_use]
pub fn fold_lrepeat_in(args: &[&str], dialect: LiteralContext) -> Option<String> {
    use tcl_syntax::native_string::NativeStringProtocol;
    let protocol = dialect.native_string_protocol()?;
    let accepts_empty = match protocol {
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) => return None,
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5) => false,
        NativeStringProtocol::C(_) | NativeStringProtocol::Jim084 => true,
    };
    let repeated = repeated_elements(
        args,
        tcl_syntax::number::Numbers::Target(dialect.numbers),
        accepts_empty,
    )?;
    String::from_utf8(dialect.list_result_serialization()?.render(&repeated)).ok()
}

/// `lindex list ?index ...?` — returns the indexed element (raw; the
/// O129 path re-quotes it as a word).
#[must_use]
pub fn fold_lindex(args: &[&str]) -> Option<String> {
    let (list, indices) = args.split_first()?;
    if indices.is_empty() {
        return Some((*list).to_owned());
    }
    let mut current = (*list).to_owned();
    for idx_str in indices {
        let elems = split_list(&current)?;
        let selected = parse_element_index(idx_str, elems.len())?;
        match selected {
            NativeIndexSelection::Selected(i) => current.clone_from(&elems[i]),
            NativeIndexSelection::Empty => return Some(String::new()), // out of range → ""
        }
    }
    Some(current)
}

// dict commands

/// Parse a flat Tcl dict string into its **canonical** key/value pairs, or
/// `None` when malformed (odd element count or unsplittable).
///
/// Canonical means what `SetDictFromAny` (tclDictObj.c) produces: a repeated
/// key keeps its **first-occurrence position** and its **last value**, because
/// the `Tcl_DictObjPut` behind it overwrites on a hash collision.
///
/// The rule is not restated here — it is
/// [`tcl_syntax::value::canonical_dict_slots`], the same function the runtime
/// seam [`tcl_syntax::value::ValueOps::dict_pairs`] and the compiler's
/// `fold_dict_create_cmd` bind. That centralisation matters: hand-written
/// copies of this walk risk drifting from the six `dict` folders' shared
/// first-match semantics, so `[dict get {a 1 a 2} a]` could fold to `1`
/// where both tclsh oracles say `2` — an "optimisation" that changes
/// program results. This layer keeps only what is local to it: how a malformed
/// dict is reported (a declined fold, not an error).
#[must_use]
pub fn parse_dict(s: &str) -> Option<Vec<(String, String)>> {
    parse_dict_in(s, None)
}

fn parse_dict_in(s: &str, dialect: Option<LiteralContext>) -> Option<Vec<(String, String)>> {
    let elems = read_literal_list(s, dialect)?;
    if elems.len() % 2 != 0 {
        return None;
    }
    Some(
        tcl_syntax::value::canonical_dict_slots(elems.iter().step_by(2).map(String::as_str))
            .into_iter()
            .map(|(key_slot, value_slot)| {
                (
                    elems[key_slot * 2].clone(),
                    elems[value_slot * 2 + 1].clone(),
                )
            })
            .collect(),
    )
}

/// `dict get dictionary ?key ...?`.
#[must_use]
pub fn fold_dict_get(args: &[&str]) -> Option<String> {
    let (dictionary, keys) = args.split_first()?;
    crate::dict::get(
        &mut LiteralOps,
        &(*dictionary).to_owned(),
        &keys.iter().map(|key| (*key).to_owned()).collect::<Vec<_>>(),
    )
    .ok()
}

/// `dict exists dictionary key ?key ...?`.
#[must_use]
pub fn fold_dict_exists(args: &[&str]) -> Option<String> {
    let (dictionary, keys) = args.split_first()?;
    if keys.is_empty() {
        return None;
    }
    crate::dict::exists(
        &mut LiteralOps,
        &(*dictionary).to_owned(),
        &keys.iter().map(|key| (*key).to_owned()).collect::<Vec<_>>(),
    )
    .ok()
}

/// `dict size dictionary`.
#[must_use]
pub fn fold_dict_size(args: &[&str]) -> Option<String> {
    let [dictionary] = args else {
        return None;
    };
    crate::dict::size(&mut LiteralOps, &(*dictionary).to_owned()).ok()
}

/// `dict keys dictionary` (no glob pattern).
#[must_use]
pub fn fold_dict_keys(args: &[&str]) -> Option<String> {
    fold_dict_members(args, None, DictionaryMember::Keys)
}

/// `dict values dictionary` (no glob pattern).
#[must_use]
pub fn fold_dict_values(args: &[&str]) -> Option<String> {
    fold_dict_members(args, None, DictionaryMember::Values)
}

#[must_use]
pub fn fold_dict_members(
    args: &[&str],
    dialect: Option<LiteralContext>,
    command: DictionaryMember,
) -> Option<String> {
    use tcl_syntax::list_result::NativeListResultSerialization;

    let [dictionary] = args else {
        return None;
    };
    let members = parse_dict_in(dictionary, dialect)?;
    let selected: Vec<_> = match command {
        DictionaryMember::Keys => members.into_iter().map(|(key, _)| key).collect(),
        DictionaryMember::Values => members.into_iter().map(|(_, value)| value).collect(),
    };
    if dialect.is_some() {
        return render_literal_list(&selected, dialect);
    }
    // The dict command is available from C Tcl 8.5 and in Jim.
    let bytes = NativeListResultSerialization::Tcl85Plus.render(&selected);
    if bytes != NativeListResultSerialization::Jim084.render(&selected) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// `dict create ?key value ...?` — canonicalise duplicate keys (last
/// value wins, original insertion position preserved), matching Tcl 9's
/// `Tcl_DictObjPut` over `DictCreateCmd`'s argument walk.
///
/// The walk is [`tcl_syntax::value::canonical_dict_slots`]'s, not a second
/// copy of it: `DictCreateCmd` puts its arguments into a fresh
/// dict pairwise, which is the same rule `SetDictFromAny` applies to a list
/// rep, so the same function answers for both.
#[must_use]
pub fn fold_dict_create(args: &[&str]) -> Option<String> {
    use tcl_syntax::value::NativeDictionaryLayout;
    let pairs = dictionary_argument_pairs(args)?;
    let bytes = NativeDictionaryLayout::Tcl.render(&pairs);
    if bytes != NativeDictionaryLayout::Jim.render(&pairs) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// The first position and last value of each key in validated dict-create argv.
#[must_use]
pub fn dictionary_argument_pairs<'a>(args: &[&'a str]) -> Option<Vec<(&'a str, &'a str)>> {
    if !args.len().is_multiple_of(2) {
        return None;
    }
    Some(
        tcl_syntax::value::canonical_dict_slots(args.iter().step_by(2).copied())
            .into_iter()
            .map(|(key, value)| (args[key * 2], args[value * 2 + 1]))
            .collect(),
    )
}

/// `dict merge ?dictionary ...?` — later dicts override earlier keys
/// (last value wins, first-seen key position preserved).
///
/// C's `DictMergeCmd` does **not** canonicalise when it has nothing to merge:
/// with a single argument it validates and returns that object *verbatim*, and
/// with more it makes the first the base and `Tcl_DictObjPut`s each later
/// dict's pairs into it — so if no pair is ever put, the base keeps its
/// original string rep. Hence `dict merge {a 1 a 2}` and
/// `dict merge {a 1 a 2} {}` both stay `a 1 a 2`, while
/// `dict merge {} {a 1 a 2}` canonicalises to `a 2`. Verified on 8.6.16
/// and 9.0.4, which agree on every row.
#[must_use]
pub fn fold_dict_merge(args: &[&str]) -> Option<String> {
    let Some((first, rest)) = args.split_first() else {
        return Some(String::new());
    };
    // Every argument must parse, or this is not a fold we may perform.
    let base = parse_dict(first)?;
    let later: Vec<Vec<(String, String)>> =
        rest.iter().map(|a| parse_dict(a)).collect::<Option<_>>()?;
    if later.iter().all(Vec::is_empty) {
        // No `Tcl_DictObjPut` happens, so the base is returned untouched —
        // duplicates and all.
        return Some((*first).to_owned());
    }
    let pairs: Vec<_> = base
        .into_iter()
        .chain(later.into_iter().flatten())
        .collect();
    let order: Vec<_> =
        tcl_syntax::value::canonical_dict_slots(pairs.iter().map(|(key, _)| key.as_str()))
            .into_iter()
            .flat_map(|(key, value)| [pairs[key].0.as_str(), pairs[value].1.as_str()])
            .collect();
    Some(list_join(&order))
}

/// A selected C Tcl may retain the original base spelling when it performs
/// no insertion. Without a C release, require that Jim's canonical
/// dictionary spelling agrees too.
#[must_use]
pub fn fold_dict_merge_versioned(
    args: &[&str],
    version: Option<tcl_dialect::TclVersion>,
) -> Option<String> {
    let result = fold_dict_merge(args)?;
    if version.is_some() {
        return Some(result);
    }
    let pairs = parse_dict(&result)?;
    let words: Vec<_> = pairs
        .iter()
        .flat_map(|(key, value)| [key.as_bytes(), value.as_bytes()])
        .collect();
    let jim = tcl_syntax::list_result::NativeListResultSerialization::Jim084.render(&words);
    (result.as_bytes() == jim).then_some(result)
}

struct LiteralOps;

impl tcl_syntax::value::ValueOps for LiteralOps {
    type Value = String;
    fn new_str(&mut self, value: &str) -> String {
        value.to_owned()
    }
    fn new_int(&mut self, value: i64) -> String {
        value.to_string()
    }
    fn new_double(&mut self, value: f64) -> String {
        value.to_string()
    }
    fn new_bool(&mut self, value: bool) -> String {
        if value { "1" } else { "0" }.to_owned()
    }
    fn new_list(&mut self, values: Vec<String>) -> String {
        tcl_syntax::list::join_list(&values)
    }
    fn as_bytes(&mut self, value: &String) -> std::rc::Rc<[u8]> {
        std::rc::Rc::from(value.as_bytes())
    }
    fn new_bytes(&mut self, bytes: &[u8]) -> String {
        std::str::from_utf8(bytes)
            .expect("Unicode literal command result")
            .to_owned()
    }
    fn as_int(&mut self, value: &String) -> Result<i64, tcl_syntax::value::ValueError> {
        value
            .parse()
            .map_err(|_| tcl_syntax::value::ValueError::NotInteger(value.clone()))
    }
    fn as_double(&mut self, value: &String) -> Result<f64, tcl_syntax::value::ValueError> {
        value
            .parse()
            .map_err(|_| tcl_syntax::value::ValueError::NotDouble(value.clone()))
    }
    fn as_bool(&mut self, value: &String) -> Result<bool, tcl_syntax::value::ValueError> {
        tcl_syntax::boolean::parse_boolean_word(value)
            .ok_or_else(|| tcl_syntax::value::ValueError::NotBoolean(value.clone()))
    }
    fn list_elements(
        &mut self,
        value: &String,
    ) -> Result<Vec<String>, tcl_syntax::value::ValueError> {
        split_list(value).ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "release-independent literal list",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_folds_keep_host_width_and_compiled_origin_distinct() {
        use crate::index::IndexReading;
        use tcl_dialect::{IndexSyntax, TclVersion};

        // Pure selected readings and container projections, not native object
        // types or compiler-instruction admission.
        let host_word = "18446744069414584321";
        assert_eq!(
            crate::index::read_under(host_word, 12, TclVersion::V8_5),
            IndexReading::HostLong(1)
        );
        assert_eq!(
            portable_index(
                host_word,
                12,
                IndexSyntax::for_version(TclVersion::V8_5),
                Some(TclVersion::V8_5)
            ),
            None
        );
        let compiled_word = "end+2147483647";
        assert_eq!(
            portable_index(
                compiled_word,
                12,
                IndexSyntax::for_version(TclVersion::V8_6),
                Some(TclVersion::V8_6)
            ),
            Some(-2_147_483_638)
        );
        assert_eq!(
            alternate_compiled_index(compiled_word, 12, Some(TclVersion::V8_6)),
            Some(12)
        );
        assert_eq!(
            alternate_compiled_index(compiled_word, 12, Some(TclVersion::V8_5)),
            None
        );
        assert_eq!(parse_range("0", compiled_word, 12), None);
        assert_eq!(
            parse_range("1", "010", 4),
            Some(NativeIndexSelection::Selected((1, 3)))
        );
        assert_eq!(parse_range("1", "010", 12), None);
    }
}
