// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected native prefix operations over original values and string storage.

use std::rc::Rc;

use tcl_core_types::c_string_extent;
use tcl_dialect::{StringCharacterModel, TclVersion};
use tcl_syntax::{
    naming::{NamePolicyProtocol, NativeNameProtocol},
    native_tcl_utf::NativeTclUtf,
    raw_string::RawString,
    value::{ValueError, ValueOps},
};

use super::{Resolution, bad_key_message, scan};
use crate::CmdError;

/// Authenticated policy for the native `tcl::prefix` implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePrefixProtocol(NamePolicyProtocol);

/// Lookup result retaining the strings materialised at the native frontier.
pub struct NativePrefixTableMatch {
    /// Issuer and selected native recipe.
    pub protocol: NativePrefixProtocol,
    /// Actual native table outcome.
    pub resolution: Resolution,
    /// Table entries materialised before the selected operation stopped.
    pub entries: Vec<Rc<[u8]>>,
    /// Original key string; absent on C's member-identity shortcut.
    pub key: Option<Rc<[u8]>>,
}

impl NativePrefixProtocol {
    /// C8.4/8.5 do not implement this command; an absent issuer abstains.
    #[must_use]
    pub fn from_policy(policy: NamePolicyProtocol) -> Option<Self> {
        match policy.recipe() {
            NativeNameProtocol::C(version) if version < TclVersion::V8_6 => None,
            _ => Some(Self(policy)),
        }
    }

    /// Keep the issuer separate from the selected operation recipe.
    #[must_use]
    pub const fn policy(self) -> NamePolicyProtocol {
        self.0
    }

    /// Resolve a native C-string table. Jim terminates on the second prefix
    /// before seeing a later exact entry, and permits an empty abbreviation.
    #[must_use]
    pub fn resolve(self, table: &[Rc<[u8]>], key: &[u8], exact: bool) -> Resolution {
        let original = key;
        let key = c_string_extent(key);
        let entries: Vec<&[u8]> = table.iter().map(|entry| c_string_extent(entry)).collect();
        if !self.0.recipe().is_jim084() {
            return scan(&entries, key, exact);
        }
        let mut abbreviated = None;
        for (index, entry) in entries.iter().enumerate() {
            if *entry == key {
                return Resolution::Exact(index);
            }
            if !exact && entry.starts_with(original) {
                if original == b"-" {
                    break;
                }
                if abbreviated.is_some() {
                    return Resolution::Ambiguous;
                }
                abbreviated = Some(index);
            }
        }
        abbreviated.map_or(Resolution::NoMatch, Resolution::UniquePrefix)
    }

    /// Error ownership for a native table miss. Jim keeps its interpreter code.
    #[must_use]
    pub fn miss_error(
        self,
        table: &[Rc<[u8]>],
        noun: &[u8],
        key: &[u8],
        ambiguous: bool,
    ) -> CmdError {
        let message = self.miss_message(table, noun, key, ambiguous);
        if self.0.recipe().is_jim084() {
            CmdError::from_byte_details(crate::CmdErrorDetails {
                string_result: None,
                message,
                error_code: crate::CmdErrorCodeUpdate::Unchanged,
                error_info: None,
                error_line: None,
                primitive_getter: None,
            })
        } else {
            CmdError::lookup_index_bytes(message, c_string_extent(noun), c_string_extent(key))
        }
    }

    /// A missing option operand has its own native message and tuple.
    #[must_use]
    pub fn missing_value(self, error_options: bool) -> CmdError {
        let message: &[u8] = match (self.0.recipe().is_jim084(), error_options) {
            (true, true) => b"missing error options",
            (true, false) => b"missing message",
            (false, true) => b"missing value for -error",
            (false, false) => b"missing value for -message",
        };
        CmdError::from_byte_details(crate::CmdErrorDetails {
            string_result: None,
            message: message.to_vec(),
            error_code: if self.0.recipe().is_jim084() {
                crate::CmdErrorCodeUpdate::Unchanged
            } else {
                crate::CmdErrorCodeUpdate::Set(b"TCL OPERATION NOARG".to_vec())
            },
            error_info: None,
            error_line: None,
            primitive_getter: None,
        })
    }

    /// Native miss text. C reports table order; Jim sorts its C-string table.
    #[must_use]
    pub fn miss_message(
        self,
        table: &[Rc<[u8]>],
        noun: &[u8],
        key: &[u8],
        ambiguous: bool,
    ) -> Vec<u8> {
        let noun = c_string_extent(noun);
        if self.0.recipe().is_jim084() && table.is_empty() {
            let mut message = b"bad ".to_vec();
            message.extend_from_slice(noun);
            message.extend_from_slice(b" \"");
            message.extend_from_slice(key);
            message.extend_from_slice(b"\": no valid options");
            return message;
        }
        let key = c_string_extent(key);
        let mut entries: Vec<&[u8]> = table.iter().map(|entry| c_string_extent(entry)).collect();
        if !self.0.recipe().is_jim084() {
            return bad_key_message(&entries, noun, key, ambiguous);
        }
        entries.sort_unstable();
        let mut message = if ambiguous {
            b"ambiguous ".to_vec()
        } else {
            b"bad ".to_vec()
        };
        message.extend_from_slice(noun);
        message.extend_from_slice(b" \"");
        message.extend_from_slice(key);
        message.extend_from_slice(b"\": must be ");
        for (index, entry) in entries.iter().enumerate() {
            if index != 0 {
                message.extend_from_slice(b", ");
                if index == entries.len() - 1 {
                    message.extend_from_slice(b"or ");
                }
            }
            message.extend_from_slice(entry);
        }
        message
    }
}

fn protocol<O: ValueOps>(ops: &O) -> Result<NativePrefixProtocol, CmdError> {
    ops.name_policy_protocol()
        .and_then(NativePrefixProtocol::from_policy)
        .ok_or_else(|| ValueError::CommandProtocolUnavailable("native prefix").into())
}

fn jim_count<O: ValueOps>(ops: &mut O, value: &O::Value) -> Result<usize, CmdError> {
    if ops.string_character_model() != Some(StringCharacterModel::Jim084Utf8) {
        return Err(ValueError::CommandProtocolUnavailable("native Jim prefix count").into());
    }
    ops.native_char_len(value).map_err(Into::into)
}

/// Match a table while retaining C's original member-identity shortcut and
/// native materialisation order. Returns the selected original member index.
pub fn native_table_match<O: ValueOps>(
    ops: &mut O,
    table: &[O::Value],
    key: &O::Value,
    exact: bool,
) -> Result<NativePrefixTableMatch, CmdError> {
    let selected = protocol(ops)?;
    let mut entries = Vec::with_capacity(table.len());
    for (index, entry) in table.iter().enumerate() {
        if !selected.policy().recipe().is_jim084() {
            match ops.same_object(entry, key) {
                Some(true) => {
                    return Ok(NativePrefixTableMatch {
                        protocol: selected,
                        resolution: Resolution::Exact(index),
                        entries,
                        key: None,
                    });
                }
                Some(false) => {}
                None => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native prefix object identity",
                    )
                    .into());
                }
            }
        }
        entries.push(ops.native_string_bytes(entry)?);
    }
    let bytes = ops.native_string_bytes(key)?;
    let resolution = selected.resolve(&entries, &bytes, exact);
    if selected.policy().recipe() == NativeNameProtocol::C(TclVersion::V8_6) {
        ops.discard_native_internal_representation(key)?;
    }
    Ok(NativePrefixTableMatch {
        protocol: selected,
        resolution,
        entries,
        key: Some(bytes),
    })
}

/// Selected original entries accepted by native `prefix all`.
pub fn native_all<O: ValueOps>(
    ops: &mut O,
    table: &O::Value,
    key: &O::Value,
) -> Result<O::Value, CmdError> {
    let selected = protocol(ops)?;
    let table = ops.list_elements(table)?;
    if selected.policy().recipe().is_jim084() && table.is_empty() {
        return Ok(ops.new_list(Vec::new()));
    }
    let key_bytes = ops.native_string_bytes(key)?;
    let jim = selected.policy().recipe().is_jim084();
    let count = if jim {
        Some(jim_count(ops, key)?)
    } else {
        None
    };
    let key_raw = RawString::from_bytes(key_bytes.clone());
    let mut selected_entries = Vec::new();
    for entry in table {
        let bytes = ops.native_string_bytes(&entry)?;
        let matched = if let Some(count) = count {
            let candidate_length = jim_count(ops, &entry)?;
            key_raw
                .jim084_prefix_compare(count, &RawString::from_bytes(bytes), candidate_length)
                .map_err(ValueError::from)?
                .is_eq()
        } else {
            NativeTclUtf::compare_counted_bytes(&bytes, &key_bytes, key_bytes.len())
                .is_some_and(std::cmp::Ordering::is_eq)
        };
        if matched {
            selected_entries.push(entry);
        }
    }
    Ok(ops.new_list(selected_entries))
}

/// Exact result bytes of native `prefix longest`, including Jim's byte-length
/// constructor fed by its native common character count.
pub fn native_longest<O: ValueOps>(
    ops: &mut O,
    table: &O::Value,
    key: &O::Value,
) -> Result<O::Value, CmdError> {
    let selected = protocol(ops)?;
    let table = ops.list_elements(table)?;
    if selected.policy().recipe().is_jim084() && table.is_empty() {
        return Err(CmdError::new_bytes(Vec::new()));
    }
    let key_bytes = ops.native_string_bytes(key)?;
    let jim = selected.policy().recipe().is_jim084();
    let key_count = if jim {
        Some(jim_count(ops, key)?)
    } else {
        None
    };
    let key_raw = RawString::from_bytes(key_bytes.clone());
    let mut first: Option<Rc<[u8]>> = None;
    let mut length = 0;
    for entry in table {
        let bytes = ops.native_string_bytes(&entry)?;
        let entry_count = if jim {
            Some(jim_count(ops, &entry)?)
        } else {
            None
        };
        let matched = if let Some(count) = key_count {
            key_raw
                .jim084_prefix_compare(
                    count,
                    &RawString::from_bytes(bytes.clone()),
                    entry_count.expect("Jim count"),
                )
                .map_err(ValueError::from)?
                .is_eq()
        } else {
            NativeTclUtf::compare_counted_bytes(&bytes, &key_bytes, key_bytes.len())
                .is_some_and(std::cmp::Ordering::is_eq)
        };
        if !matched {
            continue;
        }
        let Some(original) = first.as_ref() else {
            length = entry_count.unwrap_or(bytes.len());
            first = Some(bytes);
            continue;
        };
        if jim {
            length = RawString::from_bytes(original.clone())
                .jim084_common_prefix_count(
                    length,
                    &RawString::from_bytes(bytes),
                    entry_count.expect("Jim count"),
                )
                .map_err(ValueError::from)?;
        } else {
            length = length.min(bytes.len());
            if let Some(index) = original[..length]
                .iter()
                .zip(&bytes[..length])
                .position(|(a, b)| a != b)
            {
                let NativeNameProtocol::C(version) = selected.policy().recipe() else {
                    unreachable!("C prefix")
                };
                length = NativeTclUtf::for_version(version)
                    .previous_character_boundary(original, index + 1)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "native prefix UTF boundary",
                    ))?;
            }
        }
    }
    let Some(first) = first else {
        return Ok(ops.empty());
    };
    let bytes = first
        .get(..length)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native prefix result storage",
        ))?;
    Ok(ops.new_bytes(bytes))
}
