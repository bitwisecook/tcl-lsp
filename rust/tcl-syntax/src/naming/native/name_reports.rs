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

//! Reported names: callback and trace names, checked Unicode views, full command and namespace names and the source spellings that round-trip.

use super::super::qualifier_segments;
use super::{NameProjectionUnavailable, NativeNameContext, NativeNameProtocol};
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, c_string_extent};
use tcl_dialect::TclVersion;

/// Audited native reporting operation, distinct from lookup or receiver identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNameReportPurpose {
    TraceCallbackName1,
    TraceCallbackName2,
    /// Missing command formatter, independently of command-table extent.
    CommandLookupError,
    /// Rename missing-source and occupied-destination formatters, separate from lookup keys.
    RenameCommandError,
    /// C interpreter-handle lookup diagnostic uses its `CString` operand.
    InterpreterLookupError,
    /// `TclOO` method enumeration formats the counted table key as `CString`.
    OoMethodEnumeration,
    /// `TclOO` unknown-method diagnostics format the counted operand as `CString`.
    OoMethodLookupError,
    /// Ensemble default-target miss exposed under the caller's subcommand.
    EnsembleMissingSubcommand,
}

/// Project a supplied native callback-name part. The caller supplies the actual
/// operation's part boundaries; combined array reporting is not reconstructed
/// from a selected root/element or an alias target.
///
/// # Errors
/// Returns `PurposeNotModelled` for an unaudited engine operation.
pub fn report_native_name_bytes(
    protocol: NativeNameProtocol,
    purpose: NativeNameReportPurpose,
    original_operation_part: &[u8],
) -> Result<&[u8], NameProjectionUnavailable> {
    if purpose == NativeNameReportPurpose::RenameCommandError && !protocol.is_jim084() {
        return Ok(original_operation_part);
    }
    if matches!(
        purpose,
        NativeNameReportPurpose::OoMethodEnumeration | NativeNameReportPurpose::OoMethodLookupError
    ) && !matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V8_6)
    {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    if protocol.is_jim084() && purpose == NativeNameReportPurpose::InterpreterLookupError {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    if protocol.is_jim084()
        && matches!(
            purpose,
            NativeNameReportPurpose::TraceCallbackName1
                | NativeNameReportPurpose::TraceCallbackName2
        )
    {
        Err(NameProjectionUnavailable::PurposeNotModelled)
    } else {
        Ok(c_string_extent(original_operation_part))
    }
}

/// Retained input parts plus the followed receiver's actual element key.
/// The key is reporting metadata from an already resolved cell, not a name
/// from which this pure function may infer that cell or its lifetime.
#[derive(Debug, Clone, Copy)]
pub struct NativeVariableTraceReportingInput<'a> {
    pub original_part1: &'a [u8],
    pub original_part2: Option<&'a [u8]>,
    pub actual_element_key: Option<&'a [u8]>,
}

/// Exact reported name parts for a reached variable access trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeVariableTraceNames<'a> {
    pub name1: &'a [u8],
    pub name2: &'a [u8],
}

/// Native access-trace name reporting. This is separate from trace registration,
/// cell resolution and unset/retirement reporting. C9 fills a missing second
/// name from the already followed array-element cell; older C does not.
///
/// # Errors
/// Returns `PurposeNotModelled` for an unaudited engine operation.
pub fn report_native_variable_access_trace_names(
    protocol: NativeNameProtocol,
    input: NativeVariableTraceReportingInput<'_>,
) -> Result<NativeVariableTraceNames<'_>, NameProjectionUnavailable> {
    let NativeNameProtocol::C(version) = protocol else {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    };
    let mut name1 = c_string_extent(input.original_part1);
    let mut name2 = input.original_part2.map(c_string_extent);
    if name2.is_none()
        && name1.last() == Some(&b')')
        && let Some(opening) = name1.iter().position(|byte| *byte == b'(')
    {
        name2 = Some(&name1[opening + 1..name1.len() - 1]);
        name1 = &name1[..opening];
    }
    if name2.is_none() && version >= TclVersion::V9_0 {
        name2 = input.actual_element_key.map(c_string_extent);
    }
    Ok(NativeVariableTraceNames {
        name1,
        name2: name2.unwrap_or(b""),
    })
}

/// Checked Unicode components of an already constructed path. This preserves
/// segment identity; it does not certify an equivalent written source spelling.
///
/// # Errors
/// Returns an error when a retained native component is not valid Rust UTF-8.
pub fn checked_namespace_path_utf8(
    path: &ByteNamespacePath,
) -> Result<Vec<String>, std::str::Utf8Error> {
    path.as_segments()
        .iter()
        .map(|part| part.try_utf8().map(str::to_owned))
        .collect()
}

/// Checked analytical slot without joining/reparsing its components. A caller
/// may length-frame these components for internal string maps. Opaque native
/// rows remain in their byte/token tables if this optional view is unavailable.
///
/// # Errors
/// Returns an error when a retained native component is not valid Rust UTF-8.
pub fn checked_command_slot_utf8(
    slot: &ByteCommandSlot,
) -> Result<tcl_core_types::CommandSlot<String, Vec<String>>, std::str::Utf8Error> {
    Ok(tcl_core_types::CommandSlot::new(
        checked_namespace_path_utf8(&slot.namespace)?,
        slot.simple.try_utf8()?.to_owned(),
    ))
}

/// A globally addressable written source spelling only when the selected
/// native recipe round-trips to this exact structured slot. Valid UTF-8 alone
/// does not certify native extent or injective colon boundaries. Failure is
/// absence of an analytical alias, never absence of the actual byte binding.
#[must_use]
pub fn native_command_source_spelling(
    protocol: NativeNameProtocol,
    slot: &ByteCommandSlot,
) -> Option<String> {
    let projected = checked_command_slot_utf8(slot).ok()?;
    let mut spelling = String::from("::");
    for part in &projected.namespace {
        spelling.push_str(part);
        spelling.push_str("::");
    }
    spelling.push_str(&projected.simple);
    let selected = protocol
        .command_publication_slot(NativeNameContext::root(), spelling.as_bytes())
        .ok()?;
    if selected != *slot {
        return None;
    }
    let lookup_agrees = if protocol.is_jim084() {
        slot.namespace.is_root()
            && protocol
                .jim_command_lookup_keys(NativeNameContext::root(), spelling.as_bytes())
                .ok()?
                .as_slice()
                == std::slice::from_ref(&slot.simple)
    } else {
        protocol
            .command_lookup_slot(NativeNameContext::root(), spelling.as_bytes())
            .ok()?
            == *slot
    };
    lookup_agrees.then_some(spelling)
}

/// One callable original source word for an exact structured slot. Reporting
/// bytes are only a candidate: publication and lookup must independently select
/// the same slot before the source-channel renderer can preserve their value.
/// This supplies no command existence, compiler preparation or edit geometry.
#[must_use]
pub fn native_command_source_word(
    protocol: NativeNameProtocol,
    slot: &ByteCommandSlot,
    channel: tcl_lexer::SourceChannel,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let value = native_command_full_name_bytes(slot);
    if protocol
        .command_publication_slot(NativeNameContext::root(), &value)
        .ok()?
        != *slot
    {
        return None;
    }
    if protocol.is_jim084() {
        if !slot.namespace.is_root()
            || protocol
                .jim_command_lookup_keys(NativeNameContext::root(), &value)
                .ok()?
                .as_slice()
                != std::slice::from_ref(&slot.simple)
        {
            return None;
        }
    } else if protocol
        .command_lookup_slot(NativeNameContext::root(), &value)
        .ok()?
        != *slot
    {
        return None;
    }
    crate::backslash::native_literal_source_word(
        &value,
        channel,
        config,
        protocol.string_protocol(),
    )
}

/// Selected immutable namespace data for reporting, independent of live identity.
/// Equal data can come from different namespace incarnations. These parts grant
/// no lookup, cell, callable, entry, or successful-completion authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeNamespaceReportParts {
    /// Original constructed C components, including literal separators in members.
    CConstructed(ByteNamespacePath),
    /// Counted bytes from the reached original Jim flat namespace holder.
    JimFlat(std::rc::Rc<[u8]>),
}

/// Qualified inventory report from selected namespace data and a retained member key.
/// This pure projection never reparses a namespace report or issues a live slot,
/// lookup input, original object header, or successful-completion capability.
#[must_use]
pub fn native_qualified_inventory_report_bytes(
    namespace: &NativeNamespaceReportParts,
    member: &[u8],
) -> Vec<u8> {
    match namespace {
        NativeNamespaceReportParts::CConstructed(path) => {
            native_command_full_name_bytes(&ByteCommandSlot::new(path.clone(), member.into()))
        }
        NativeNamespaceReportParts::JimFlat(flat) => crate::naming::rooted_member_key_bytes(
            &crate::naming::root_unrooted_key_bytes(flat),
            member,
        ),
    }
}

/// `TclGetCommandFullName` reporting bytes from an already selected native slot.
/// Literal colons and non-text bytes remain data; this projection supplies no
/// source spelling, lookup key or command identity.
#[must_use]
pub fn native_command_full_name_bytes(slot: &ByteCommandSlot) -> Vec<u8> {
    constructed_command_address_bytes(slot)
}

/// Counted absolute command input from retained namespace components and a tail.
/// This constructs an address; lookup, table identity and original object headers
/// remain independent. Literal colons and non-text bytes are not reparsed.
#[must_use]
pub fn constructed_command_address_bytes(slot: &ByteCommandSlot) -> Vec<u8> {
    let mut report = native_namespace_full_name_bytes(&slot.namespace);
    if !slot.namespace.is_root() {
        report.extend_from_slice(b"::");
    }
    report.extend_from_slice(slot.simple.as_bytes());
    report
}

/// Counted full-name bytes of an already constructed C namespace path.
/// Component boundaries remain owned by the path; this reporting projection
/// supplies no written address, namespace existence or native token.
#[must_use]
pub fn native_namespace_full_name_bytes(path: &ByteNamespacePath) -> Vec<u8> {
    let mut report = b"::".to_vec();
    push_namespace_components(&mut report, path);
    report
}

/// Unrooted counterpart of [`native_namespace_full_name_bytes`]: the same
/// constructed components joined without the root marker (empty for root).
#[must_use]
pub fn native_namespace_key_bytes(path: &ByteNamespacePath) -> Vec<u8> {
    let mut key = Vec::new();
    push_namespace_components(&mut key, path);
    key
}

fn push_namespace_components(out: &mut Vec<u8>, path: &ByteNamespacePath) {
    for (index, component) in path.as_segments().iter().enumerate() {
        if index != 0 {
            out.extend_from_slice(b"::");
        }
        out.extend_from_slice(component.as_bytes());
    }
}

/// Written C namespace spelling only when its native address round-trips to
/// the exact constructed path. Jim's flat namespace object must instead be
/// supplied to `native_jim_namespace_source_spelling`; a path does not recover
/// that independently retained object. The root is shared by both engines.
#[must_use]
pub fn native_namespace_source_spelling(
    protocol: NativeNameProtocol,
    path: &ByteNamespacePath,
) -> Option<String> {
    if path.is_root() {
        return Some(String::from("::"));
    }
    if protocol.is_jim084() {
        return None;
    }
    let components = checked_namespace_path_utf8(path).ok()?;
    let spelling = format!("::{}", components.join("::"));
    let projected = protocol
        .namespace_address_input(NativeNameContext::root(), spelling.as_bytes())
        .ok()?;
    let selected = ByteNamespacePath::from_segments(qualifier_segments(projected.selected()));
    (selected == *path).then_some(spelling)
}

/// A globally addressable Jim namespace spelling from its actual flat object,
/// independently of C-style analytical paths and Jim's flat command keys.
#[must_use]
pub fn native_jim_namespace_source_spelling(actual_namespace_object: &[u8]) -> Option<String> {
    let namespace = std::str::from_utf8(actual_namespace_object).ok()?;
    let spelling = format!("::{namespace}");
    let projected = NativeNameProtocol::Jim084
        .namespace_address_input(NativeNameContext::root(), spelling.as_bytes())
        .ok()?;
    (projected.selected() == actual_namespace_object).then_some(spelling)
}
