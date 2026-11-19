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

//! Compile-time constant-folding callbacks for Tcl list / dict commands.
//!
//! Each callback takes the resolved literal argument strings and returns
//! the result string, or `None` when the operation cannot be folded soundly.
//! Wired onto the `const_fold` field of the matching `CommandSpec` /
//! `SubCommand`; consumed by the optimiser's O129 path, which renders the
//! result as a single word.
//!
//! List-string codec: the element quoter is the canonical
//! [`tcl_syntax::list::list_element`] (`Tcl_ConvertElement`). [`split_list`]
//! below is **deliberately not** the canonical `Tcl_SplitList` — it is a
//! conservative *fold-safety* splitter that bails (`None`) on any backslash or
//! any bare `{`/`}`/`"`, so the optimiser only folds provably-simple lists. The
//! shared splitter ([`tcl_syntax::list::split_list`]) decodes backslashes and
//! accepts the full grammar; using it here would fold *more* (changing optimiser
//! output), so the policy stays local on purpose.
//!
//! `parse_index` delegates to the shared [`tcl_cmd_core::index`] grammar (the
//! same parser the runtime uses), so the optimiser folds the index forms Tcl
//! resolves at run time — but only where every release agrees on the answer,
//! since these callbacks carry no release and an index word inherits the
//! numeral grammar's version differences. `clamp_range` is the post-resolution
//! range clamp shared with the `string` subcommand folds.

/// Common list elements for a literal with no target. Source escapes need the
/// selected receiver. Strict Tcl errors and Jim's more permissive parse must
/// agree before returning a value.
pub(crate) fn split_list(s: &str) -> Option<Vec<String>> {
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
pub(crate) fn list_join<S: AsRef<str>>(elems: &[S]) -> String {
    tcl_syntax::list::join_list(elems)
}

/// Parse a Tcl index expression against a string/list of `length`, returning the
/// resolved index (which may be negative or `>= length` — the caller clamps).
///
/// Delegates to the shared [`tcl_cmd_core::index`] grammar — the *same* parser
/// the runtime's `lindex` / `lrange` / `string index` use — so the optimiser
/// folds the forms Tcl resolves at run time: `end`, `end±N`, the arithmetic
/// operands (`1+1`, `0-1`, `end--1`), and every integer radix (`0x2`, `0o7`,
/// `0b101`).
///
/// An index word is read by `Tcl_GetIntForIndex`, so it inherits every version
/// difference in the numeral grammar and the integer range — `lindex $l 010` is
/// index 8 up to 8.6 and 10 from 9.0, `lindex $l 4294967295` the last element
/// up to 8.6 and past it from 9.0. These folds are registered as plain
/// [`ConstFoldFn`](crate::hooks::ConstFoldFn)s, which carry no release, so this
/// resolves under **every** C release grammar and the Jim grammar, and folds
/// only when they agree.
/// Portable readings and literal/value container outcomes must also agree;
/// a host-width-dependent reading remains unavailable.
///
/// Declining is free: an unfolded `lindex` is evaluated at run time by an
/// interpreter that does know its release. Folding under one release's grammar
/// would instead bake a wrong constant into a program built for another — the
/// one outcome a const-folder must never produce. (When these folds migrate to
/// [`VersionedConstFoldFn`](crate::hooks::VersionedConstFoldFn), the release can
/// be named and `index::resolve_opt_in` used directly.)
pub(crate) fn parse_index(s: &str, length: usize) -> Option<i64> {
    parse_index_consensus(s, length, |index| index)
}

/// A proved container selection, distinct from unproved native parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeIndexSelection<T> {
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
pub(crate) fn parse_element_index(s: &str, length: usize) -> Option<NativeIndexSelection<usize>> {
    parse_index_consensus(s, length, |index| {
        NativeIndexSelection::from_selection(
            usize::try_from(index).ok().filter(|&index| index < length),
        )
    })
}

/// Clamped range selected unanimously by the native index policies.
/// Compare the container outcome rather than the engines' raw index encodings.
pub(crate) fn parse_range(
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
        return tcl_cmd_core::index::resolve_opt_in(spec, length, syntax);
    };
    match tcl_cmd_core::index::read_under(spec, length, release) {
        tcl_cmd_core::index::IndexReading::At(value) => Some(value),
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
    tcl_cmd_core::index::compiles_apart(spec, length, release)
        .then(|| i64::try_from(length).ok())
        .flatten()
}

/// Resolve `(first, last)` parsed indices into a clamped `[lo, hi]`
/// inclusive range over a collection of `len` items, or `None` when the
/// range is empty (`first > last` after clamping `first` up to 0 and
/// `last` down to `len-1`).
pub(crate) fn clamp_range(first: i64, last: i64, len: usize) -> Option<(usize, usize)> {
    let last_max = i64::try_from(len).ok()? - 1;
    let first = first.max(0);
    let last = last.min(last_max);
    if first > last {
        return None;
    }
    Some((usize::try_from(first).ok()?, usize::try_from(last).ok()?))
}

// list commands

/// `concat ?arg ...?` — materialise the shared native concatenation grammar.
pub(crate) fn fold_concat(args: &[&str]) -> String {
    tcl_syntax::list::concat_values(args.iter().copied())
}

/// `list ?arg ...?` — build a proper Tcl list (each arg re-quoted).
///
/// Registered as a `ConstFoldFn` (`fn(&[&str]) -> Option<String>`); the
/// `Option` is the dispatch-table contract, not redundant wrapping — building
/// a list never fails. `unnecessary_wraps` is a false positive here.
///
/// Public because it is also the `SpecTcl` sandbox's `foldlist` builtin: a pack
/// fold body that returns a list must produce the same quoting the shipped
/// `list` fold produces, and sharing this function is what makes that true by
/// construction rather than by review.
#[allow(clippy::unnecessary_wraps)] // signature fixed by ConstFoldFn dispatch contract
#[must_use]
pub fn fold_list(args: &[&str]) -> Option<String> {
    Some(list_join(args))
}

/// `llength list`.
pub(crate) fn fold_llength(args: &[&str]) -> Option<String> {
    let [l] = args else {
        return None;
    };
    Some(split_list(l)?.len().to_string())
}

fn read_literal_list(text: &str, dialect: Option<crate::InvocationDialect>) -> Option<Vec<String>> {
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

fn render_literal_list(
    words: &[String],
    dialect: Option<crate::InvocationDialect>,
) -> Option<String> {
    let bytes = match dialect {
        Some(dialect) => dialect.list_result_serialization()?.render(words),
        None => tcl_syntax::list_result::portable_list_result(words)?,
    };
    String::from_utf8(bytes).ok()
}

/// `lreverse list`.
pub(crate) fn fold_lreverse(args: &[&str]) -> Option<String> {
    fold_lreverse_in(args, None)
}

pub(crate) fn fold_lreverse_in(
    args: &[&str],
    dialect: Option<crate::InvocationDialect>,
) -> Option<String> {
    let [list] = args else {
        return None;
    };
    let mut elems = read_literal_list(list, dialect)?;
    let preserve = match dialect.and_then(crate::InvocationDialect::native_string_protocol) {
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
pub(crate) fn fold_lsort(args: &[&str]) -> Option<String> {
    fold_lsort_in(args, None)
}

pub(crate) fn fold_lsort_in(
    args: &[&str],
    dialect: Option<crate::InvocationDialect>,
) -> Option<String> {
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
        match dialect.and_then(crate::InvocationDialect::native_string_protocol) {
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
pub(crate) fn fold_join(args: &[&str]) -> Option<String> {
    let (l, sep) = match args {
        [l] => (*l, " "),
        [l, s] => (*l, *s),
        _ => return None,
    };
    Some(split_list(l)?.join(sep))
}

/// A literal `split` whose release is unknown uses the shared list route's
/// release-independent answer.
pub(crate) fn fold_split(args: &[&str]) -> Option<String> {
    fold_split_versioned(args, None)
}

/// The selected release owns both source decoding and list rendering.
pub(crate) fn fold_split_versioned(
    args: &[&str],
    version: Option<tcl_dialect::TclVersion>,
) -> Option<String> {
    crate::value_transfer::evaluate_literal(
        &crate::value_transfer::builtins::SPLIT,
        "split",
        None,
        args,
        version,
    )
}

/// Cap on a single constant-fold's materialised output (1 MiB) — bound the
/// product (count × element bytes), not just the count, so large elements
/// can't blow up the fold.
const MAX_FOLD_OUTPUT_BYTES: usize = 1 << 20;

/// Exact bounded count read under the receiver's numeral grammar.
pub(crate) fn repetition_count(word: &str, numbers: tcl_syntax::number::Numbers) -> Option<usize> {
    let tcl_syntax::number::Number::Int(value) = numbers.parse_whole(word)? else {
        return None;
    };
    usize::try_from(value).ok()
}

/// `lrepeat count ?element ...?` on every release providing the command.
pub(crate) fn fold_lrepeat(args: &[&str]) -> Option<String> {
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
pub(crate) fn fold_lrepeat_in(args: &[&str], dialect: crate::InvocationDialect) -> Option<String> {
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
pub(crate) fn fold_lindex(args: &[&str]) -> Option<String> {
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

/// `lrange list first last` — returns the sublist (re-quoted).
pub(crate) fn fold_lrange(args: &[&str]) -> Option<String> {
    let [source, first, last] = args else {
        return None;
    };
    let length = split_list(source)?.len();
    parse_range(first, last, length)?;

    let contract = crate::native_result::NativeResultContract::ListRange {
        list_at: 0,
        first_at: 1,
        last_at: 2,
    };
    let dialects = tcl_dialect::TclVersion::ALL
        .into_iter()
        .map(crate::InvocationDialect::for_version)
        .chain([crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )]);
    let mut result = None;
    for dialect in dialects {
        let arguments = crate::InvocationArguments::literals(args).with_dialect(dialect);
        let value = contract
            .select(arguments, 0)
            .constant_range_literal_result(arguments)?;
        if result.as_ref().is_some_and(|previous| previous != &value) {
            return None;
        }
        result = Some(value);
    }
    result
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
fn parse_dict(s: &str) -> Option<Vec<(String, String)>> {
    parse_dict_in(s, None)
}

fn parse_dict_in(
    s: &str,
    dialect: Option<crate::InvocationDialect>,
) -> Option<Vec<(String, String)>> {
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
pub(crate) fn fold_dict_get(args: &[&str]) -> Option<String> {
    let (dict, keys) = args.split_first()?;
    if keys.is_empty() {
        return Some((*dict).to_owned());
    }
    let mut current = (*dict).to_owned();
    for key in keys {
        let pairs = parse_dict(&current)?;
        let val = pairs.iter().find(|(k, _)| k == key)?;
        current.clone_from(&val.1);
    }
    Some(current)
}

/// `dict exists dictionary key ?key ...?`.
pub(crate) fn fold_dict_exists(args: &[&str]) -> Option<String> {
    let (dict, keys) = args.split_first()?;
    if keys.is_empty() {
        return None;
    }
    let mut current = (*dict).to_owned();
    for (n, key) in keys.iter().enumerate() {
        let Some(pairs) = parse_dict(&current) else {
            return Some("0".to_owned());
        };
        let Some((_, v)) = pairs.iter().find(|(k, _)| k == key) else {
            return Some("0".to_owned());
        };
        if n + 1 < keys.len() {
            current.clone_from(v);
        }
    }
    Some("1".to_owned())
}

/// `dict size dictionary`.
pub(crate) fn fold_dict_size(args: &[&str]) -> Option<String> {
    let [d] = args else {
        return None;
    };
    Some(parse_dict(d)?.len().to_string())
}

/// `dict keys dictionary` (no glob pattern).
pub(crate) fn fold_dict_keys(args: &[&str]) -> Option<String> {
    fold_dict_members(
        args,
        None,
        crate::native_dictionary::NativeDictionaryCommand::Keys,
    )
}

/// `dict values dictionary` (no glob pattern).
pub(crate) fn fold_dict_values(args: &[&str]) -> Option<String> {
    fold_dict_members(
        args,
        None,
        crate::native_dictionary::NativeDictionaryCommand::Values,
    )
}

pub(crate) fn fold_dict_members(
    args: &[&str],
    dialect: Option<crate::InvocationDialect>,
    command: crate::native_dictionary::NativeDictionaryCommand,
) -> Option<String> {
    use crate::native_dictionary::NativeDictionaryCommand;
    use tcl_syntax::list_result::NativeListResultSerialization;

    let [dictionary] = args else {
        return None;
    };
    let members = parse_dict_in(dictionary, dialect)?;
    let selected: Vec<_> = match command {
        NativeDictionaryCommand::Keys => members.into_iter().map(|(key, _)| key).collect(),
        NativeDictionaryCommand::Values => members.into_iter().map(|(_, value)| value).collect(),
        _ => return None,
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
pub(crate) fn fold_dict_create(args: &[&str]) -> Option<String> {
    use tcl_syntax::value::NativeDictionaryLayout;
    let pairs = dictionary_argument_pairs(args)?;
    let bytes = NativeDictionaryLayout::Tcl.render(&pairs);
    if bytes != NativeDictionaryLayout::Jim.render(&pairs) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// The first position and last value of each key in validated dict-create argv.
pub(crate) fn dictionary_argument_pairs<'a>(args: &[&'a str]) -> Option<Vec<(&'a str, &'a str)>> {
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
pub(crate) fn fold_dict_merge(args: &[&str]) -> Option<String> {
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
pub(crate) fn fold_dict_merge_versioned(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal string-backed `ValueOps` so these folds can be cross-checked
    /// against the **real** canonicalisation owner rather than against a second
    /// copy of my own reasoning. Values are `Rc<str>`; lists are the elements
    /// this module's own `split_list` finds, so both sides see identical input.
    #[derive(Default)]
    struct Strs;

    impl tcl_cmd_core::index::NativeIndexOps for Strs {
        type IndexOperation = ();
        type IndexBinaryReceipt = ();
        type IndexKeyOwner = Self::Value;
        fn index_borrowed_key(
            &mut self,
            value: &Self::Value,
        ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
            Ok(value.clone())
        }
        fn index_created_key(
            &mut self,
            value: Self::Value,
        ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
            Ok(value)
        }
        fn index_key_value(owner: &Self::Value) -> &Self::Value {
            owner
        }
        fn index_hold_key_result(
            &mut self,
            owner: &Self::Value,
        ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
            Ok(owner.clone())
        }
        fn index_finish_held_key_result(
            &mut self,
            held: Self::Value,
        ) -> Result<tcl_cmd_core::index::IndexResult<Self::Value>, tcl_cmd_core::CmdError> {
            Ok(tcl_cmd_core::index::IndexResult::Detached(held))
        }
        fn index_key_argument(
            &mut self,
            owner: &Self::Value,
        ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
            Ok(owner.clone())
        }
        fn index_key_result(
            &mut self,
            owner: &Self::Value,
            created: bool,
            ancestor: bool,
        ) -> Result<tcl_cmd_core::index::IndexResult<Self::Value>, tcl_cmd_core::CmdError> {
            Ok(if created {
                tcl_cmd_core::index::IndexResult::Created(owner.clone())
            } else if ancestor {
                tcl_cmd_core::index::IndexResult::Detached(owner.clone())
            } else {
                tcl_cmd_core::index::IndexResult::Borrowed(owner.clone())
            })
        }
        fn enter_index_operation(&mut self) -> Result<(), tcl_cmd_core::CmdError> {
            Ok(())
        }
        fn check_index_operation(&mut self, (): &()) -> Result<(), tcl_cmd_core::CmdError> {
            Ok(())
        }
        fn retain_index_failure(&mut self, _error: &tcl_cmd_core::CmdError) {
            // Pure neutral adapter has no interpreter first-cause owner.
        }
        fn discard_owned_index_result(&mut self, created: Self::Value) {
            drop(created);
        }
    }

    impl tcl_syntax::value::ValueOps for Strs {
        type Value = std::rc::Rc<str>;

        fn new_str(&mut self, s: &str) -> Self::Value {
            std::rc::Rc::from(s)
        }
        fn new_int(&mut self, n: i64) -> Self::Value {
            std::rc::Rc::from(n.to_string().as_str())
        }
        fn new_double(&mut self, f: f64) -> Self::Value {
            std::rc::Rc::from(f.to_string().as_str())
        }
        fn new_bool(&mut self, b: bool) -> Self::Value {
            std::rc::Rc::from(if b { "1" } else { "0" })
        }
        fn new_list(&mut self, items: Vec<Self::Value>) -> Self::Value {
            std::rc::Rc::from(
                list_join(
                    &items
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>(),
                )
                .as_str(),
            )
        }
        fn as_bytes(&mut self, v: &Self::Value) -> std::rc::Rc<[u8]> {
            std::rc::Rc::from(v.as_bytes())
        }
        fn new_bytes(&mut self, bytes: &[u8]) -> Self::Value {
            self.new_str(std::str::from_utf8(bytes).expect("Unicode-only fixture input"))
        }

        fn as_int(&mut self, v: &Self::Value) -> Result<i64, tcl_syntax::value::ValueError> {
            v.parse::<i64>()
                .map_err(|_| tcl_syntax::value::ValueError::NotInteger(v.to_string()))
        }
        fn as_double(&mut self, v: &Self::Value) -> Result<f64, tcl_syntax::value::ValueError> {
            v.parse::<f64>()
                .map_err(|_| tcl_syntax::value::ValueError::NotDouble(v.to_string()))
        }
        fn as_bool(&mut self, v: &Self::Value) -> Result<bool, tcl_syntax::value::ValueError> {
            Ok(!matches!(v.as_ref(), "0" | "false" | "no" | "off" | ""))
        }
        fn list_elements(
            &mut self,
            v: &Self::Value,
        ) -> Result<Vec<Self::Value>, tcl_syntax::value::ValueError> {
            match split_list(v) {
                Some(elems) => Ok(elems
                    .into_iter()
                    .map(|e| std::rc::Rc::from(e.as_str()))
                    .collect()),
                None => Err(tcl_syntax::value::ValueError::BadList(
                    "unparsable list".to_string(),
                )),
            }
        }
    }

    /// These registry const-folds decode a dict
    /// **string** and must canonicalise duplicate keys exactly as the runtime
    /// does, or the O129 folds change program results.
    ///
    /// Each expectation is the byte-exact output of `tclsh9.0.4` (and
    /// `tclsh8.6.16`, which agrees on every row), and `parse_dict` is
    /// additionally cross-checked pair-for-pair against
    /// [`tcl_syntax::value::ValueOps::dict_pairs`] — the owner — on the same
    /// inputs, so the two cannot drift apart silently.
    #[test]
    fn dict_folds_canonicalise_duplicate_keys_like_the_owner() {
        use tcl_syntax::value::ValueOps as _;

        // Cross-check against the owner on the same inputs.
        for src in [
            "a 1 a 2",
            "x 1 x 2 y 3",
            "a 1 b 2 a 3",
            "a 1 b 2",
            "",
            "k {v 1 v 2}",
        ] {
            let mine = parse_dict(src).expect("parses");
            let mut ops = Strs;
            let v: std::rc::Rc<str> = std::rc::Rc::from(src);
            let theirs = ops.dict_pairs(&v).expect("owner parses");
            let theirs: Vec<(String, String)> = theirs
                .into_iter()
                .map(|(k, val)| (k.to_string(), val.to_string()))
                .collect();
            assert_eq!(
                mine, theirs,
                "parse_dict disagrees with the owner on {src:?}"
            );
        }

        // Oracle rows for the folds the duplicate bug actually reached.
        assert_eq!(fold_dict_get(&["a 1 a 2", "a"]).unwrap(), "2");
        assert_eq!(fold_dict_get(&["x 1 x 2 y 3", "x"]).unwrap(), "2");
        assert_eq!(fold_dict_size(&["a 1 a 2"]).unwrap(), "1");
        assert_eq!(fold_dict_size(&["x 1 x 2 y 3"]).unwrap(), "2");
        assert_eq!(fold_dict_keys(&["a 1 a 2"]).unwrap(), "a");
        assert_eq!(fold_dict_keys(&["x 1 x 2 y 3"]).unwrap(), "x y");
        assert_eq!(fold_dict_values(&["a 1 a 2"]).unwrap(), "2");
        assert_eq!(fold_dict_values(&["x 1 x 2 y 3"]).unwrap(), "2 3");
        assert_eq!(fold_dict_exists(&["a 1 a 2", "a"]).unwrap(), "1");
        // First-occurrence position survives a later duplicate.
        assert_eq!(fold_dict_keys(&["a 1 b 2 a 3"]).unwrap(), "a b");
        assert_eq!(fold_dict_values(&["a 1 b 2 a 3"]).unwrap(), "3 2");
        // `dict get` returns a nested value verbatim — it is not re-canonicalised.
        assert_eq!(fold_dict_get(&["o {a 1 a 2}", "o"]).unwrap(), "a 1 a 2");

        // `dict merge` only canonicalises when a pair is actually merged in.
        assert_eq!(fold_dict_merge(&[]).unwrap(), "");
        assert_eq!(fold_dict_merge(&["a 1 a 2"]).unwrap(), "a 1 a 2");
        assert_eq!(fold_dict_merge(&["x 1 x 2 y 3"]).unwrap(), "x 1 x 2 y 3");
        assert_eq!(fold_dict_merge(&["a 1 a 2", ""]).unwrap(), "a 1 a 2");
        assert_eq!(fold_dict_merge(&["a 1 a 2", "b 9"]).unwrap(), "a 2 b 9");
        assert_eq!(fold_dict_merge(&["a 1", "a 2 a 3"]).unwrap(), "a 3");
        assert_eq!(fold_dict_merge(&["", "a 1 a 2"]).unwrap(), "a 2");

        // `dict create` already had the rule; it must stay agreeing.
        assert_eq!(fold_dict_create(&["a", "1", "a", "2"]).unwrap(), "a 2");
    }

    #[test]
    fn split_list_handles_braces_quotes_bare() {
        assert_eq!(split_list("a b c").unwrap(), ["a", "b", "c"]);
        assert_eq!(split_list("{a b} c").unwrap(), ["a b", "c"]);
        assert_eq!(split_list("{a {b c}} d").unwrap(), ["a {b c}", "d"]);
        assert_eq!(split_list("\"a b\" c").unwrap(), ["a b", "c"]);
        assert_eq!(split_list("").unwrap(), Vec::<String>::new());
        // Malformed → None.
        assert_eq!(split_list("{a b"), None, "unbalanced brace");
        assert_eq!(split_list("\"a b"), None, "unterminated quote");
        assert_eq!(split_list("a\\ b"), None, "backslash bails");
        assert_eq!(split_list("{a}b"), None, "junk after brace");
    }

    #[test]
    fn list_element_quotes_like_tcl() {
        use tcl_syntax::list::list_element;
        assert_eq!(list_element("foo"), "foo");
        assert_eq!(list_element(""), "{}");
        assert_eq!(list_element("a b"), "{a b}");
        assert_eq!(list_element("a$b"), "{a$b}");
    }

    /// A leading `#` is comment-unsafe only in **list position 0**
    /// (`Tcl_Merge` passes `TCL_DONT_QUOTE_HASH` for every later element), so
    /// the folds that re-render a list must not quote it everywhere. Each row
    /// is the byte-exact output of `tclsh8.6.16` and `tclsh9.0.4`, which agree.
    #[test]
    fn lsort_folds_only_byte_ordered_ascii_sorts() {
        assert_eq!(fold_lsort(&["c b a"]).as_deref(), Some("a b c"));
        assert_eq!(
            fold_lsort(&["-ascii", "-increasing", "B a b"]).as_deref(),
            Some("B a b")
        );
        assert_eq!(
            fold_lsort(&["-decreasing", "-unique", "b a b"]).as_deref(),
            Some("b a")
        );
        assert_eq!(fold_lsort(&["{b c} a"]).as_deref(), Some("a {b c}"));
        for option in [
            "-integer",
            "-real",
            "-dictionary",
            "-nocase",
            "-command",
            "-index",
        ] {
            assert_eq!(fold_lsort(&[option, "b a"]), None, "{option}");
        }
        assert_eq!(fold_lsort(&[]), None);
    }

    #[test]
    fn short_list_results_keep_receiver_spelling_and_dictionary_members_use_its_writer() {
        use crate::native_dictionary::NativeDictionaryCommand::{Keys, Values};
        use tcl_dialect::model::{DialectPoint, Release};
        let jim = crate::InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        let c86 = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let c91 = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
        for (dialect, reversed, sorted, members) in [
            (jim, "abc", " {abc} ", r"a\]\ b"),
            (c86, "abc", "abc", "{a] b}"),
            (c91, " {abc} ", "abc", "{a] b}"),
        ] {
            assert_eq!(
                fold_lreverse_in(&[" {abc} "], Some(dialect)).as_deref(),
                Some(reversed)
            );
            assert_eq!(
                fold_lsort_in(&[" {abc} "], Some(dialect)).as_deref(),
                Some(sorted)
            );
            assert_eq!(
                fold_dict_members(&["{a] b} x"], Some(dialect), Keys).as_deref(),
                Some(members)
            );
            assert_eq!(
                fold_dict_members(&["x {a] b}"], Some(dialect), Values).as_deref(),
                Some(members)
            );
        }
        assert_eq!(fold_lreverse(&[" {abc} "]), None);
        assert_eq!(fold_lsort(&[" {abc} "]), None);
    }

    #[test]
    fn folded_lists_quote_a_leading_hash_only_in_position_zero() {
        assert_eq!(fold_list(&["a", "#"]).as_deref(), Some("a #"));
        assert_eq!(fold_list(&["#", "a"]).as_deref(), Some("{#} a"));
        assert_eq!(fold_lreverse(&["# a"]).as_deref(), Some("a #"));
        assert_eq!(fold_lsort(&["a #"]), None);
        assert_eq!(fold_lsort(&["-decreasing", "a #"]).as_deref(), Some("a #"));
        assert_eq!(fold_lrepeat(&["2", "#"]).as_deref(), Some("{#} #"));
        // A version-neutral fold cannot choose between native C84's bare
        // leading hash and the protected C85+/Jim list-object bytes.
        assert_eq!(fold_lrange(&["x # y", "1", "2"]), None);
        assert_eq!(fold_split(&["a-#", "-"]).as_deref(), Some("a #"));
        assert_eq!(fold_concat(&["a", "#"]), "a #");
        assert_eq!(fold_dict_keys(&["a 1 # 2"]).as_deref(), Some("a #"));
        assert_eq!(fold_dict_values(&["a # b 1"]).as_deref(), Some("{#} 1"));
        assert_eq!(
            fold_dict_create(&["a", "1", "#", "2"]).as_deref(),
            Some("a 1 # 2")
        );
    }

    #[test]
    fn list_folds_match_tcl() {
        assert_eq!(fold_concat(&["a", " b ", "c"]), "a b c");
        assert_eq!(fold_concat(&["{a b}", "c"]), "{a b} c");
        assert_eq!(fold_concat(&["a", "b\\ "]), "a b\\ ");
        assert_eq!(fold_list(&["a", "b c"]).as_deref(), Some("a {b c}"));
        assert_eq!(fold_llength(&["a b c"]).as_deref(), Some("3"));
        assert_eq!(fold_llength(&["{a b} c"]).as_deref(), Some("2"));
        assert_eq!(fold_lreverse(&["a b c"]).as_deref(), Some("c b a"));
        assert_eq!(fold_lreverse(&["{a b} c"]).as_deref(), Some("c {a b}"));
        assert_eq!(fold_join(&["a b c", "-"]).as_deref(), Some("a-b-c"));
        assert_eq!(fold_join(&["{a b} c"]).as_deref(), Some("a b c"));
        assert_eq!(fold_split(&["a,b,c", ","]).as_deref(), Some("a b c"));
        assert_eq!(fold_split(&["a b,c", ","]).as_deref(), Some("{a b} c"));
        // the default split set includes `\r`. `split "a\r\nb"`
        // → tclsh `a {} b` (empty element between \r and \n).
        assert_eq!(fold_split(&["a\r\nb"]).as_deref(), Some("a {} b"));
        assert_eq!(fold_split(&["a b\tc"]).as_deref(), Some("a b c"));
        assert_eq!(fold_lrepeat(&["3", "x"]).as_deref(), Some("x x x"));
        assert_eq!(fold_lrepeat(&["2", "a", "b"]).as_deref(), Some("a b a b"));
        assert_eq!(fold_lindex(&["a b c", "1"]).as_deref(), Some("b"));
        assert_eq!(fold_lindex(&["{a b} c", "0"]).as_deref(), Some("a b"));
        assert_eq!(fold_lindex(&["a b c", "9"]).as_deref(), Some(""));
        assert_eq!(fold_lrange(&["a b c d", "1", "2"]).as_deref(), Some("b c"));
        assert_eq!(
            fold_lrange(&["{a b} c d", "0", "1"]).as_deref(),
            Some("{a b} c")
        );
        // Malformed list arg → no fold.
        assert_eq!(fold_llength(&["{a b"]), None);
    }

    #[test]
    fn range_folding_uses_actual_native_serialization_and_portable_agreement() {
        let args = ["x #value end", "1", "1"];
        assert_eq!(fold_lrange(&args), None);
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let supplied = crate::model::ingress::static_context_for(profile);
            let registry = supplied.commands();
            let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
            let resolved = registry
                .resolve_call("lrange", &args, dialect.authoring_query())
                .unwrap();
            assert_eq!(
                resolved.spec.run_const_fold(&args, dialect.tcl_version),
                None
            );
            assert_eq!(
                resolved.spec.run_const_fold_in(&args, dialect).as_deref(),
                Some(if profile == "tcl8.4" {
                    "#value"
                } else {
                    "{#value}"
                }),
                "{profile}"
            );
        }
        assert_eq!(fold_lrange(&["x a b", "1", "2"]).as_deref(), Some("a b"));
        assert_eq!(fold_lrange(&["x a b", "9", "end"]).as_deref(), Some(""));
        assert_eq!(fold_lrange(&["x a b", "invalid", "end"]), None);
    }

    #[test]
    fn index_folds_match_tclsh_oracle() {
        // The unversioned callback preserves only the native grammar
        // intersection. C8.4 rejects these arithmetic forms, so it must decline
        // even though a selected modern grammar can resolve both to index 2.
        assert_eq!(fold_lindex(&["a b c d e", "1+1"]), None);
        assert_eq!(fold_lindex(&["a b c d e", "3-1"]), None);
        let selected = tcl_dialect::IndexSyntax::for_version(tcl_dialect::TclVersion::V8_6);
        for index in ["1+1", "3-1"] {
            assert_eq!(
                tcl_cmd_core::index::resolve_opt_in(index, 5, selected),
                Some(2)
            );
        }
        assert_eq!(fold_lindex(&["a b c d e", "0x2"]).as_deref(), Some("c"));
        assert_eq!(fold_lindex(&["a b c d e", "end-1"]).as_deref(), Some("d"));
        // C end+1 and Jim's encoded positive-end sentinel both return empty.
        // Raw index equality remains unknown; container result equality holds.
        assert_eq!(parse_index("end--1", 5), None);
        assert_eq!(fold_lindex(&["a b c d e", "end--1"]).as_deref(), Some(""));
        assert_eq!(
            fold_lrange(&["a b c d e", "0", "end--1"]).as_deref(),
            Some("a b c d e")
        );
        assert_eq!(
            fold_lrange(&["a b c d e", "end--1", "end"]).as_deref(),
            Some("")
        );
        assert_eq!(
            fold_lrange(&["a b c d e", "2", "end--1"]).as_deref(),
            Some("c d e")
        );
        assert_eq!(fold_lrange(&["a b c d e", "1+1", "end"]), None);
        assert_eq!(
            fold_lrange(&["a b c d e", "2", "end"]).as_deref(),
            Some("c d e")
        );
        // Still declines genuinely bad specs.
        assert_eq!(fold_lindex(&["a b c d e", "1.0"]), None);
        assert_eq!(fold_lindex(&["a b c d e", "foo"]), None);
    }

    // `fold_list` is registered through the `ConstFoldFn` callback contract
    // (`-> Option<String>`) but the computation is infallible. Positive: a
    // non-empty arg list folds. Edge: even the empty arg list folds (to the
    // empty list) — it never returns `None`, which is why the `Option` is a
    // dispatch artefact, not a real failure channel.
    #[test]
    fn fold_list_is_infallible() {
        assert_eq!(fold_list(&["a", "b c"]).as_deref(), Some("a {b c}"));
        assert_eq!(fold_list(&[]).as_deref(), Some(""));
        assert!(fold_list(&["x"]).is_some());
    }

    #[test]
    fn dict_folds_match_tcl() {
        assert_eq!(fold_dict_get(&["a 1 b 2", "b"]).as_deref(), Some("2"));
        assert_eq!(fold_dict_get(&["a 1 b 2", "z"]), None, "missing key");
        assert_eq!(fold_dict_exists(&["a 1 b 2", "b"]).as_deref(), Some("1"));
        assert_eq!(fold_dict_exists(&["a 1 b 2", "z"]).as_deref(), Some("0"));
        assert_eq!(fold_dict_size(&["a 1 b 2"]).as_deref(), Some("2"));
        assert_eq!(fold_dict_keys(&["a 1 b 2"]).as_deref(), Some("a b"));
        assert_eq!(fold_dict_values(&["a 1 b 2"]).as_deref(), Some("1 2"));
        // dict create de-dups, last value wins, position preserved.
        assert_eq!(
            fold_dict_create(&["a", "X", "b", "Y", "a", "Z"]).as_deref(),
            Some("a Z b Y")
        );
        assert_eq!(
            fold_dict_merge(&["a 1 b 2", "b 9 c 3"]).as_deref(),
            Some("a 1 b 9 c 3")
        );
        // Odd dict → no fold.
        assert_eq!(fold_dict_size(&["a 1 b"]), None);
    }

    /// A plain `ConstFoldFn` carries no release, so an index whose value depends
    /// on the release must not fold. Declining costs an optimisation; folding
    /// under the wrong grammar would bake a wrong constant into the program.
    #[test]
    fn index_folds_decline_when_the_releases_disagree() {
        // `010` is index 8 up to 8.6 and 10 from 9.0 — no single answer.
        assert_eq!(parse_index("010", 12), None);
        assert_eq!(parse_index("end-010", 12), None);
        // `1_0` and `0d1` are 9.0-only spellings: valid there, `bad index` before.
        assert_eq!(parse_index("1_0", 12), None);
        assert_eq!(parse_index("0d1", 12), None);
        // Unanimous spellings still fold.
        assert_eq!(parse_index("1", 12), Some(1));
        assert_eq!(parse_index("0x1", 12), Some(1));
        assert_eq!(parse_index("007", 12), Some(7));
        assert_eq!(parse_index("end", 12), Some(11));
        assert_eq!(parse_index("end-2", 12), Some(9));
        // Still nothing at all for a genuinely bad spec.
        assert_eq!(parse_index("nope", 12), None);
        // Past 32 bits 8.4 to 8.6 wrap or raise where 9.0 reads the wide, and
        // a 64-bit `long` decides the rest.
        for spec in [
            "2147483648",
            "4294967295",
            "4294967296",
            "-4294967295",
            "end-4294967295",
            "1+2147483647",
            "18446744073709551615",
            "9223372036854775808",
        ] {
            assert_eq!(parse_index(spec, 12), None, "{spec}");
        }
        assert_eq!(parse_index("2147483647", 12), Some(2_147_483_647));
        // Every grammar reads this one before the first element, but 8.6
        // encodes the literal after the end.
        assert_eq!(parse_index("end-2147483649", 12), None);
    }

    #[test]
    fn portable_folds_keep_host_width_and_compiled_origin_distinct() {
        use tcl_cmd_core::index::IndexReading;
        use tcl_dialect::{IndexSyntax, TclVersion};

        // Pure selected readings and container projections, not native object
        // types or compiler-instruction admission.
        let host_word = "18446744069414584321";
        assert_eq!(
            tcl_cmd_core::index::read_under(host_word, 12, TclVersion::V8_5),
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

    /// Fold only unanimous container outcomes, including clamped ranges whose
    /// raw native indices differ. A genuinely differing result still declines.
    #[test]
    fn lindex_folds_only_unanimous_indices() {
        assert_eq!(
            fold_lindex(&["a b c d e f g h i j k l", "1"]).as_deref(),
            Some("b")
        );
        assert_eq!(fold_lindex(&["a b c d e f g h i j k l", "010"]), None);
        assert_eq!(fold_lrange(&["a b c d", "1", "2"]).as_deref(), Some("b c"));
        assert_eq!(
            fold_lrange(&["a b c d", "1", "010"]).as_deref(),
            Some("b c d")
        );
        assert_eq!(fold_lrange(&["a b c d e f g h i j k l", "1", "010"]), None);
        assert_eq!(
            fold_lindex(&["a b c d e f g h i j k l", "4294967295"]),
            None
        );
        assert_eq!(fold_lrange(&["a b c d", "0", "4294967296"]), None);
        assert_eq!(fold_lrange(&["a b c d", "0", "end-2147483649"]), None);
    }
}
