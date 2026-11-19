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

//! Map the selected native regular expression compiler's components onto UTF-8 byte offsets.

use tcl_cmd_core::regex::RegexFlags;
use tcl_dialect::{DialectProfile, TclVersion, model::Family};
use tcl_regex::PatternComponent;
use tcl_syntax::native_tcl_utf::{NativeCPlainChar, NativeTclUtf};

/// Options are taken only from descriptor-admitted occurrences, including
/// consumed values and the actual end of option parsing.
pub(crate) fn source_flags(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<RegexFlags> {
    use tcl_registry::AuthoredSourceOptionBoundary as Boundary;
    let scan = schema.authored_source_diagnostic_options()?;
    if !matches!(
        scan.boundary,
        Boundary::Positional(_) | Boundary::Terminator(_) | Boundary::End
    ) {
        return None;
    }
    let mut flags = RegexFlags::default();
    for occurrence in scan.options {
        if !occurrence.available || occurrence.values.is_none() {
            return None;
        }
        flags.apply_compile_option(occurrence.option.name);
    }
    Some(flags)
}

/// A detached source pattern with no option receipt may claim a component
/// only when both ordinary and expanded compilation produce it. The same
/// component agreement applies across an unversioned C Tcl profile.
pub(crate) fn components(
    text: &str,
    profile: &DialectProfile,
    flags: Option<RegexFlags>,
) -> Option<Vec<PatternComponent>> {
    let mut common = None;
    let unknown_options = flags.is_none();
    let mut options = flags.unwrap_or_default();
    for expanded in [false, true] {
        if unknown_options {
            options.expanded = expanded;
        } else if expanded {
            break;
        }
        match profile.runtime_core_family()? {
            Family::Tcl => {
                let single;
                let versions = if let Some(version) = profile.runtime_version() {
                    single = [version];
                    &single[..]
                } else {
                    &TclVersion::ALL[..]
                };
                for &version in versions {
                    options.z_anchor = version.regex_z_anchor();
                    agree(&mut common, c_components(text, version, options)?);
                }
            }
            Family::Jim => {
                if profile
                    .core_point
                    .is_some_and(|point| point.release() != tcl_dialect::model::Release::JIM_0_84)
                {
                    return None;
                }
                let observed = tcl_regex::jim::Regex::components(
                    text.as_bytes(),
                    tcl_regex::cmd_core::jim_flags(options),
                )
                .ok()?;
                agree(&mut common, observed);
            }
            _ => return None,
        }
    }
    common
}

fn agree(common: &mut Option<Vec<PatternComponent>>, current: Vec<PatternComponent>) {
    if let Some(retained) = common {
        // Both compiler traces are ordered by original input position.
        retained.retain(|candidate| {
            current
                .binary_search_by_key(&candidate.range.start, |item| item.range.start)
                .is_ok_and(|index| current[index] == *candidate)
        });
    } else {
        *common = Some(current);
    }
}

fn c_components(
    text: &str,
    version: TclVersion,
    flags: RegexFlags,
) -> Option<Vec<PatternComponent>> {
    let utf = NativeTclUtf::for_version(version);
    let bytes = text.as_bytes();
    let mut offsets = vec![0];
    let mut cursor = 0;
    let mut previous = None;
    while let Some(unit) = utf.decode_unit(&bytes[cursor..], previous) {
        cursor += unit.width;
        previous = Some(unit.value);
        offsets.push(cursor);
    }
    let recipe = tcl_syntax::native_regex::NativeRegexpRecipe::for_version(version);
    let units = recipe.pattern_units(bytes, NativeCPlainChar::Signed);
    let cflags = tcl_regex::cmd_core::are_flags(flags);
    let mut observed = tcl_regex::are_components(&units, cflags).ok()?;
    if version <= TclVersion::V8_5 && !text.is_ascii() {
        let unsigned = recipe.pattern_units(bytes, NativeCPlainChar::Unsigned);
        if unsigned != units && tcl_regex::are_components(&unsigned, cflags).ok()? != observed {
            return None;
        }
    }
    for entry in &mut observed {
        entry.range = *offsets.get(entry.range.start)?..*offsets.get(entry.range.end)?;
        text.get(entry.range.clone())?;
    }
    Some(observed)
}
