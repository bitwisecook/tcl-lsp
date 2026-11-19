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

//! Parsers for one word or value: flags, dialect sets, lifecycles and stamp gates, the closed vocabularies and the descriptor values a row carries.

use super::statement_reader::Word;
use super::{Log, available, leak_str};
use crate::catalogue;
use std::fmt;
use tcl_dialect::TclVersion;
use tcl_dialect::model::SpecSurface;
use tcl_registry::traits::Traits;

mod descriptor_values;
mod lifecycles;
mod vocabularies;

pub(super) use descriptor_values::{
    deprecation_fix_from, object_class_row, parse_appended_arity, parse_arity,
    parse_byte_array_effect, parse_byte_array_payload, parse_defines_symbol, parse_deprecation_fix,
    parse_handle_binding, parse_integer_domain, parse_representation_effect, parse_return_elements,
    parse_semantic_operation, parse_setter_constraint, parse_var_elements_effect,
    parse_var_write_typing,
};
pub use descriptor_values::{semantic_operation_spelling, semantic_operations};
pub(super) use lifecycles::{
    check_contained, checked_arity_windows, checked_lifecycle, checked_option_effect_families,
    checked_stamp_windows, form_stamp_is_plain, lifecycle_flag, place_stamp, stamp_gate,
};
pub(super) use vocabularies::{
    ANALYSER_HOOKS, BODY_KINDS, CODEGEN_HOOKS, CONNECTION_SIDES, DEFAULT_FORM_FIRST_WORDS,
    FORM_KINDS, FORMAT_TYPES, FRAME_LAYOUTS, FRAME_LEVEL_WORDS, INLINE_CODEGEN_HOOKS,
    LOWERING_HOOKS, PRESENTATIONS, RETURN_TYPE_HOOKS, SCRIPT_TIMINGS, SIDE_EFFECT_TARGETS,
    STORAGE_TYPES, TCL_TYPES, VARIABLE_SCOPES, completion_row,
};

// Small word / value parsers

/// Advance past a flag and return its value word's text.
pub(super) fn next_text(words: &[Word], i: &mut usize) -> String {
    *i += 1;
    words.get(*i).map_or_default(|w| w.text.clone())
}

/// A descriptor whose invalid list must withdraw its metadata uses the
/// fixed pack input grammar. Lenient presentation readers have a separate API.
pub(super) fn strict_pack_list(
    text: &str,
) -> Result<Vec<std::borrow::Cow<'_, str>>, tcl_syntax::list::ListError> {
    let config =
        super::pack_source_lexer_config(tcl_lexer::SourceSegmentationPurpose::NestedScript);
    tcl_syntax::list::split_list_with_syntax(text, config.list_parse, config.escapes)
}

/// The Tcl list elements of a word, braces stripped, tolerating a malformed
/// list rather than refusing the whole declaration.
pub(crate) fn list_words(text: &str) -> Vec<String> {
    tcl_syntax::list::split_list_lenient(text)
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .collect()
}

/// A bare property word means `yes`; an explicit `yes` / `no` overrides it.
pub(super) fn parse_flag(tail: &[Word]) -> bool {
    !matches!(
        tail.first().map(|w| w.text.as_str()),
        Some("no" | "false" | "0")
    )
}

/// The tri-state form, where the argument is required.
pub(super) fn parse_tristate(text: &str) -> Option<bool> {
    match text {
        "yes" | "true" | "1" => Some(true),
        "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

pub(super) fn parse_version(name: &str) -> Option<TclVersion> {
    match name {
        "tcl8.4" | "8.4" => Some(TclVersion::V8_4),
        "tcl8.5" | "8.5" => Some(TclVersion::V8_5),
        "tcl8.6" | "8.6" => Some(TclVersion::V8_6),
        "tcl9.0" | "9.0" => Some(TclVersion::V9_0),
        "tcl9.1" | "9.1" => Some(TclVersion::V9_1),
        _ => None,
    }
}

/// The `tclX.Y+` "and later" closure — one core row open-ended at `base`.
fn version_and_later(base: TclVersion) -> &'static [SpecSurface] {
    match base {
        TclVersion::V8_4 => SpecSurface::ALL_TCL,
        TclVersion::V8_5 => SpecSurface::TCL85_PLUS,
        TclVersion::V8_6 => SpecSurface::TCL86_PLUS,
        TclVersion::V9_0 => SpecSurface::TCL90_PLUS,
        TclVersion::V9_1 => SpecSurface::TCL91,
    }
}

/// The surface of a `jimX.Y+` word ([`catalogue::jim_and_later_release`]).
fn jim_and_later(word: &str) -> Option<&'static [SpecSurface]> {
    let release = catalogue::jim_and_later_release(word)?;
    let window: &'static [tcl_dialect::model::SpecWindow] =
        Box::leak(Box::new([(leak_str(release), None)]));
    Some(Box::leak(Box::new([SpecSurface::core_in(
        tcl_dialect::model::Family::Jim,
        window,
    )])))
}

/// Apply an [`available::Availability`] at a scope that has no
/// `required_package` field of its own.
///
/// Only `command` and `default` carry one, so a `{package NAME}` row
/// anywhere else has nowhere to land: it is reported rather than dropped in
/// silence, because a reader of the pack would otherwise believe the
/// requirement was in force.
pub(super) fn apply_availability(
    target: &mut Option<&'static [SpecSurface]>,
    availability: available::Availability,
    scope: &str,
    line: u32,
    log: &mut Log,
) {
    if let Some(package) = availability.required_package {
        log.say(
            line,
            format!(
                "`available {{package {package}}}` on a {scope} has no SpecTcl 1.x field \
                 (only `command` and `default` carry `required_package`); the requirement \
                 is dropped"
            ),
        );
    }
    if availability.surface().is_some() {
        *target = availability.surface();
    }
}

/// A legacy `dialects` word: members verbatim, plus `tclX.Y+`, `all-tcl`,
/// `tcl8.x`, translated to the surface rows they name.
pub(super) fn parse_dialects(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<&'static [SpecSurface]> {
    let mut rows: Vec<SpecSurface> = Vec::new();
    for member in list_words(text) {
        let named = match member.as_str() {
            "all-tcl" => Some(SpecSurface::ALL_TCL),
            "tcl8.x" => Some(SpecSurface::TCL8X),
            other => match other.strip_suffix('+') {
                Some(base) => parse_version(base)
                    .map(version_and_later)
                    .or_else(|| jim_and_later(other)),
                None => catalogue::dialect_surface(other),
            },
        };
        match named {
            Some(named) => rows.extend_from_slice(named),
            None => log.say(line, format!("unknown dialect `{member}` dropped")),
        }
    }
    (!rows.is_empty()).then(|| &*Box::leak(rows.into_boxed_slice()))
}

pub(super) fn parse_traits(text: &str, line: u32, log: &mut Log) -> Traits {
    let mut traits = Traits::empty();
    for name in list_words(text) {
        match catalogue::trait_bit(&name) {
            Some(bit) => traits |= bit,
            None => log.say(line, unaccepted_trait_notice(&name)),
        }
    }
    traits
}

/// The notice for a trait word the registry will not accept.
///
/// A retired spelling names what carries the fact now: a pack written against
/// an older registry is still out there saying `traits HAS_SWITCH_BODY`, and
/// its author needs the replacement, not the news that a word the registry
/// once documented is meaningless. The word is dropped either way — the
/// replacement is a descriptor the pack writes for itself.
pub(super) fn unaccepted_trait_notice(name: &str) -> String {
    match tcl_registry::traits::RETIRED_TRAITS
        .iter()
        .find(|(retired, _)| *retired == name)
    {
        Some((_, replacement)) => format!("retired trait `{name}` dropped: {replacement}"),
        None => format!("unknown trait `{name}` dropped"),
    }
}

pub(super) fn parse_taint(
    text: &str,
    line: u32,
    log: &mut Log,
) -> tcl_registry::taint::TaintColour {
    let mut colour = tcl_registry::taint::TaintColour::empty();
    for name in list_words(text) {
        match catalogue::taint_bit(&name) {
            Some(bit) => colour |= bit,
            None => log.say(line, format!("unknown taint colour `{name}` dropped")),
        }
    }
    colour
}

/// Resolve a fieldless enum value by its Rust variant spelling.
pub(super) fn by_name<T: Copy + fmt::Debug>(all: &[T], name: &str) -> Option<T> {
    tcl_registry::metadata_catalogue::metadata_variant_by_name(
        all,
        name,
        tcl_registry::metadata_catalogue::MetadataSpellingComparison::Exact,
    )
}

/// [`by_name`], reporting an unknown spelling rather than silently dropping.
pub(super) fn enum_by_name<T: Copy + fmt::Debug>(
    all: &[T],
    name: &str,
    what: &str,
    line: u32,
    log: &mut Log,
) -> Option<T> {
    let found = by_name(all, name);
    if found.is_none() {
        log.say(line, format!("unknown {what} `{name}` dropped"));
    }
    found
}
