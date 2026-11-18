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

//! Splitting a written name: whether it is qualified or absolute, its segments and their byte extents, the separator run and the command tail.

/// Does `name` contain a `::` namespace separator (i.e. is it qualified)?
#[must_use]
pub fn is_qualified(name: &[u8]) -> bool {
    tcl_core_types::naming::has_namespace_separator(name)
}

/// Is the **written** `name` absolute, that is rooted at the global namespace?
///
/// `TclGetNamespaceForQualName` (`tclNamesp.c`) starts its component walk at
/// the global namespace exactly when the name begins with two colons, and then
/// skips every further colon of that run. A leading run of two or more colons
/// is therefore the root marker (`::a`, `:::a`, and `::` alone, which names
/// the global namespace or its `{}` entity), while a single leading `:` is the
/// first character of a simple name, so `:a` is relative. A separator later in
/// the name qualifies it without rooting it: `a::b` is relative.
///
/// This reads a written name. A constructed key carries its own rooting
/// convention ([`root_unrooted_key`], [`unroot_rooted_key`]), and Jim's flat
/// names theirs ([`jim_global_variable_key_bytes`]).
///
/// ```
/// use tcl_syntax::naming::is_absolute;
/// assert!(is_absolute("::a"));
/// assert!(is_absolute(":::a"));
/// assert!(is_absolute("::"));
/// assert!(!is_absolute(":a"));
/// assert!(!is_absolute("a::b"));
/// assert!(!is_absolute(""));
/// ```
#[must_use]
pub fn is_absolute(name: &str) -> bool {
    is_absolute_bytes(name.as_bytes())
}

/// Counted-byte twin of [`is_absolute`], for names that need not be UTF-8.
#[must_use]
pub fn is_absolute_bytes(name: &[u8]) -> bool {
    name.starts_with(b"::")
}

/// Split a (possibly qualified) name on `::`, dropping empty segments — so
/// `::a::b::cmd` → `[a, b, cmd]`, `::cmd` → `[cmd]`, `cmd` → `[cmd]`, `::` → `[]`.
/// A run of **two or more** colons is one separator (all consecutive colons are
/// consumed), while a lone interior colon is an ordinary name character, so
/// `a:::b` → `[a, b]` and `a:b` → `[a:b]`. A trailing separator drops its empty
/// tail (`a::b::` → `[a, b]`); callers that care about the `{}`-named cmd/var a
/// trailing `::` denotes test for it themselves. Mirrors
/// `TclGetNamespaceForQualName`'s component walk (`tclNamesp.c`).
#[must_use]
pub fn qualifier_segments(name: &[u8]) -> Vec<&[u8]> {
    qualifier_component_extents(name)
        .map(|extent| &name[extent])
        .collect()
}

/// Exact original byte extents of the nonempty components of a written name.
/// The same owner supplies [`qualifier_segments`]: a complete run of at least
/// two colons is one separator; a lone colon remains a component character.
/// Leading and trailing separators produce no empty component. Consumers that
/// distinguish a namespace operand from an empty terminal command or variable
/// must retain that independently selected purpose.
///
/// Extents retain original Unicode/NUL bytes and all separator widths. This
/// iterator does not select a native input prefix, invert a constructed key,
/// establish a namespace or grant an editable source container.
///
/// ```
/// use tcl_syntax::naming::qualifier_component_extents;
/// assert_eq!(qualifier_component_extents(b"::::a:::b::").collect::<Vec<_>>(),
///            vec![4..5, 8..9]);
/// assert_eq!(qualifier_component_extents(b"a:b").collect::<Vec<_>>(),
///            vec![0..3]);
/// ```
pub fn qualifier_component_extents(
    name: &[u8],
) -> impl Iterator<Item = std::ops::Range<usize>> + '_ {
    let mut next = 0;
    std::iter::from_fn(move || {
        while next < name.len() {
            let start = next;
            let mut at = start;
            while at < name.len() {
                if name[at] == b':' && name.get(at + 1) == Some(&b':') {
                    let end = at;
                    at += 2;
                    while name.get(at) == Some(&b':') {
                        at += 1;
                    }
                    next = at;
                    if start < end {
                        return Some(start..end);
                    }
                    break;
                }
                at += 1;
            }
            if at == name.len() && next == start {
                next = at;
                return Some(start..at);
            }
        }
        None
    })
}

/// What one literal fragment of a partly computed name fixes whatever its
/// computed neighbours hold. Only a component closed by a separator run inside
/// the fragment is fixed on its right, and only one opened by a separator run
/// inside it is fixed on its left: a neighbour can add colons to an edge run,
/// which leaves it a separator, but cannot join a component across one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenFragmentComponents<'a> {
    /// The fragment starts with a separator run.
    pub rooted: bool,
    /// Components each followed by a separator run inside the fragment, in
    /// order, up to the first component that is not.
    pub closed: Vec<&'a [u8]>,
    /// The text after the fragment's last separator run, when the fragment
    /// has one; empty when the fragment ends with that run.
    pub tail_after_separator: Option<&'a [u8]>,
}

/// Split a literal fragment of a partly computed name by the same separator
/// rule as [`qualifier_component_extents`].
///
/// ```
/// use tcl_syntax::naming::written_fragment_components;
/// let prefix = written_fragment_components(b"::other::");
/// assert!(prefix.rooted);
/// assert_eq!(prefix.closed, [b"other".as_slice()]);
/// let open = written_fragment_components(b"::other:");
/// assert!(open.closed.is_empty());
/// assert_eq!(written_fragment_components(b"::counter").tail_after_separator, Some(b"counter".as_slice()));
/// assert_eq!(written_fragment_components(b"ver").tail_after_separator, None);
/// ```
#[must_use]
pub fn written_fragment_components(fragment: &[u8]) -> WrittenFragmentComponents<'_> {
    let separator = |range: std::ops::Range<usize>| range.len() >= 2;
    let extents: Vec<_> = qualifier_component_extents(fragment).collect();
    let rooted = match extents.first() {
        Some(first) => separator(0..first.start),
        None => fragment.len() >= 2,
    };
    let mut closed = Vec::new();
    for (index, extent) in extents.iter().enumerate() {
        let next = extents
            .get(index + 1)
            .map_or(fragment.len(), |next| next.start);
        if !separator(extent.end..next) {
            break;
        }
        closed.push(&fragment[extent.clone()]);
    }
    let tail_after_separator = match extents.last() {
        Some(last) if last.end < fragment.len() => {
            separator(last.end..fragment.len()).then_some(&fragment[fragment.len()..])
        }
        Some(last) => {
            let previous = extents
                .len()
                .checked_sub(2)
                .map_or(0, |index| extents[index].end);
            separator(previous..last.start).then_some(&fragment[last.clone()])
        }
        None => rooted.then_some(&fragment[fragment.len()..]),
    };
    WrittenFragmentComponents {
        rooted,
        closed,
        tail_after_separator,
    }
}

/// The exact byte extent of the final written namespace separator run.
/// Each complete run of two or more colons separates components; a lone colon
/// remains component data. The counted input is not truncated or normalised.
/// This pure span query supplies no dialect, native input extent, namespace,
/// source edit, command lookup or publication authority.
#[must_use]
pub fn written_separator_extent(name: &[u8]) -> Option<std::ops::Range<usize>> {
    let mut previous_end = 0;
    let mut separator = None;
    for component in qualifier_component_extents(name) {
        if component.start > previous_end {
            separator = Some(previous_end..component.start);
        }
        previous_end = component.end;
    }
    if previous_end < name.len() {
        separator = Some(previous_end..name.len());
    }
    separator
}

/// Does `name` end with a namespace separator (a run of ≥2 colons)? In a
/// command or variable name such a trailing `::` names the `{}` (empty) entity
/// in the qualified namespace (`TclGetNamespaceForQualName`), so the simple name
/// is `""` rather than the last [`qualifier_segments`] element.
#[must_use]
pub fn ends_with_separator(name: &[u8]) -> bool {
    name.len() >= 2 && name[name.len() - 1] == b':' && name[name.len() - 2] == b':'
}

/// The simple (tail) part of a **written** command name, for the
/// *definition* direction: empty when the name is empty or ends in a
/// separator run — such a spelling addresses the empty-string `{}` command
/// inside its full qualifier chain (`proc x:: {} {}` defines `::x::`;
/// `rename foo x::` binds `::x::`; `rename bar ::` the global `{}` command
/// — all tclsh 8.6/9.0-pinned) — otherwise the last colon-run
/// segment, or the whole name when unqualified.  The resolution-direction
/// counterparts apply the same rule (`Namespaces::home_of` in the runtime,
/// native command-name protocols), so definition and dispatch can never
/// disagree about which command a trailing-separator spelling names.
#[must_use]
pub fn written_command_tail(name: &[u8]) -> &[u8] {
    written_separator_extent(name).map_or(name, |separator| &name[separator.end..])
}

/// Borrowed components of a written command name, including an empty tail.
/// These bytes select no native purpose, namespace token or command identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenCommandParts<'a> {
    /// Whether the written name starts with a root marker.
    pub absolute: bool,
    /// Nonempty namespace components before the simple name.
    pub qualifiers: Vec<&'a [u8]>,
    /// Exact simple name, empty after a trailing separator run.
    pub simple: &'a [u8],
}

/// Decompose counted written bytes without selecting a native input extent.
#[must_use]
pub fn written_command_parts(name: &[u8]) -> WrittenCommandParts<'_> {
    let mut qualifiers = qualifier_segments(name);
    if !ends_with_separator(name) {
        qualifiers.pop();
    }
    WrittenCommandParts {
        absolute: is_absolute_bytes(name),
        qualifiers,
        simple: written_command_tail(name),
    }
}

/// [`qualifier_segments`] over a `&str`, as owned `String`s — the one shared
/// segment split for consumers that hold qualified names as strings (the
/// compiler's interprocedural/optimiser namespace walks). Same colon-run rule:
/// `::a:::b::` → `["a", "b"]`.
///
/// ```
/// use tcl_syntax::naming::qualifier_segments_owned;
/// assert_eq!(qualifier_segments_owned("::a::b::cmd"), vec!["a", "b", "cmd"]);
/// assert_eq!(qualifier_segments_owned("a:::b"), vec!["a", "b"]);
/// assert!(qualifier_segments_owned("::").is_empty());
/// ```
#[must_use]
pub fn qualifier_segments_owned(name: &str) -> Vec<String> {
    qualifier_segments(name.as_bytes())
        .into_iter()
        .map(|s| {
            core::str::from_utf8(s)
                .expect("subslice of valid UTF-8")
                .to_owned()
        })
        .collect()
}
