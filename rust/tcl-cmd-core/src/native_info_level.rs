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

//! Reached native caller-variable frame lookup over original object receivers.

use crate::CmdError;
use tcl_runtime_api::Introspect;
use tcl_syntax::scalar_getter::NativeScalarGetterKind;
use tcl_syntax::value::{ValueError, ValueOps};

/// Actual native integer getter and original frame-vector availability.
pub trait NativeInfoLevelObjects: ValueOps + Introspect<Value = <Self as ValueOps>::Value> {
    /// Reach Int/Wide on the same original header, including failed cache changes.
    fn native_level_integer(
        &mut self,
        original: &<Self as ValueOps>::Value,
        kind: NativeScalarGetterKind,
    ) -> Result<i64, ValueError>;
    /// Build a fresh native List from genuine retained frame argv children.
    /// Missing original frame data is a capability refusal, never rebuilt strings.
    fn native_level_arguments(
        &self,
        level: usize,
    ) -> Result<Option<<Self as ValueOps>::Value>, ValueError>;
}

/// Run an independently selected native opcode without dispatching a mutable handler.
///
/// # Errors
/// Preserves primitive getter/cache failures, genuine bad-level errors and missing
/// original frame-vector authority.
pub fn native_level<O: NativeInfoLevelObjects>(
    ops: &mut O,
    number: Option<&<O as ValueOps>::Value>,
    kind: NativeScalarGetterKind,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<<O as ValueOps>::Value, CmdError> {
    let current = i64::try_from(ops.level())
        .map_err(|_| ValueError::CommandProtocolUnavailable("native frame level width"))?;
    let Some(number) = number else {
        return Ok(ops.new_int(current));
    };
    let requested = ops
        .native_level_integer(number, kind)
        .map_err(|error| integer_getter_error(error, protocol))?;
    let target = if requested <= 0 {
        current.checked_add(requested)
    } else {
        Some(requested)
    };
    if let Some(target) = target.filter(|target| *target >= 1 && *target <= current)
        && let Some(argv) = ops
            .native_level_arguments(usize::try_from(target).map_err(|_| {
                ValueError::CommandProtocolUnavailable("native frame level width")
            })?)?
    {
        return Ok(argv);
    }
    let original = ops.native_string_bytes(number)?;
    let input = &original[..original
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(original.len())];
    let mut message = b"bad level \"".to_vec();
    message.extend_from_slice(input);
    message.push(b'"');
    let mut code = Vec::new();
    for element in [b"TCL".as_slice(), b"LOOKUP", b"STACK_LEVEL", input] {
        if !code.is_empty() {
            code.push(b' ');
        }
        tcl_syntax::list::append_list_element(&mut code, element, false);
    }
    Err(CmdError::with_error_code_bytes(message, code).with_native_string_result(protocol))
}

/// Primitive bytes, error code and result production come from the same
/// retained getter receipt. A contextual bad-level error has its own producer.
fn integer_getter_error(
    error: ValueError,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> CmdError {
    use tcl_syntax::scalar_getter::NativeScalarGetterResultProducer;
    let ValueError::NativeScalarGetter(record) = error else {
        return CmdError::from(error);
    };
    if record.protocol().tcl_version() != protocol.tcl_version() {
        return ValueError::CommandProtocolUnavailable("native integer error result protocol")
            .into();
    }
    let Some(producer) = record.integer_result_producer() else {
        return ValueError::CommandProtocolUnavailable("native integer error result producer")
            .into();
    };
    let error = CmdError::from(ValueError::NativeScalarGetter(record));
    match producer {
        NativeScalarGetterResultProducer::FreshString => error,
        NativeScalarGetterResultProducer::AppendString => error.with_native_string_result(protocol),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;
    use tcl_syntax::{
        native_string::NativeStringProtocol,
        scalar_getter::{NativeScalarGetterFailure, NativeScalarGetterProtocol},
    };

    #[test]
    fn native_level_integer_error_retains_primitive_fields_and_string_producer() {
        // naming.compiler.introspection-source-and-effect-frontiers
        // docs/design/analysis/name-resolution-proofs/compiler-introspection-source-and-effect-frontiers.md
        // naming.numeric.primitive-int-original-width-cache
        // docs/design/analysis/name-resolution-proofs/numeric-primitive-int-original-width-cache.md
        // Native introspection case13 observes the String result. Primitive
        // row10 separately supplies this exact original String bad's message
        // and error code; this transport check observes no header or frame.
        for (version, rows) in [
            (
                TclVersion::V8_6,
                include_str!("../../tcl-syntax/tests/data/native_scalar_getters/int/8.6.18.txt"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../../tcl-syntax/tests/data/native_scalar_getters/int/9.0.4.txt"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../../tcl-syntax/tests/data/native_scalar_getters/int/9.1.0.txt"),
            ),
        ] {
            let original: Vec<_> = rows.lines().nth(10).unwrap().split("\\t").collect();
            assert_eq!(&original[..4], &["10", "1", "777", "string"]);
            let getter = NativeScalarGetterProtocol::for_tcl_version(version);
            let record = getter
                .failure_presentation(
                    NativeScalarGetterKind::Int,
                    NativeScalarGetterFailure::Invalid,
                    b"bad",
                )
                .unwrap();
            let protocol = NativeStringProtocol::C(version);
            let error = integer_getter_error(
                ValueError::NativeScalarGetter(Box::new(record.clone())),
                protocol,
            );
            assert!(error.native_access_refusal().is_none());
            let fields = error.into_byte_details();
            assert_eq!(fields.string_result, Some(protocol));
            assert_eq!(fields.message, record.message_bytes());
            assert_eq!(
                fields.message,
                crate::binary::hex_decode(original[4].as_bytes()).unwrap(),
                "{version:?}: original primitive message"
            );
            assert_eq!(fields.primitive_getter.as_deref(), Some(&record));
            let crate::CmdErrorCodeUpdate::Set(code) = fields.error_code else {
                panic!("{version:?}: primitive must set the retained error code");
            };
            assert_eq!(
                code,
                crate::binary::hex_decode(original[5].as_bytes()).unwrap(),
                "{version:?}: original primitive error code"
            );
        }
    }

    #[test]
    fn native_level_width_failure_keeps_the_fresh_result_producer() {
        // naming.compiler.introspection-source-and-effect-frontiers
        // docs/design/analysis/name-resolution-proofs/compiler-introspection-source-and-effect-frontiers.md
        // Native C90 case12 reaches GetInt width overflow and observes no
        // result primary. C86/C91 select Wide for that source operand, so this
        // five-release pure recipe control does not claim the same opcode there.
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let getter = NativeScalarGetterProtocol::for_tcl_version(version);
            let record = getter
                .failure_presentation(
                    NativeScalarGetterKind::Int,
                    NativeScalarGetterFailure::IntWidthOverflow,
                    b"-2147483649",
                )
                .unwrap();
            let error = integer_getter_error(
                ValueError::NativeScalarGetter(Box::new(record.clone())),
                NativeStringProtocol::C(version),
            );
            assert!(error.native_access_refusal().is_none());
            let fields = error.into_byte_details();
            assert_eq!(fields.string_result, None);
            assert_eq!(fields.message, record.message_bytes());
            assert_eq!(fields.primitive_getter.as_deref(), Some(&record));
        }
    }

    #[test]
    fn native_level_primitive_result_cannot_borrow_another_protocol() {
        let record = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V9_0)
            .failure_presentation(
                NativeScalarGetterKind::Int,
                NativeScalarGetterFailure::Invalid,
                b"invalid",
            )
            .unwrap();
        let error = integer_getter_error(
            ValueError::NativeScalarGetter(Box::new(record)),
            NativeStringProtocol::C(TclVersion::V9_1),
        );
        assert!(error.native_access_refusal().is_some());
        assert_eq!(error.into_byte_details().string_result, None);
    }

    #[test]
    fn native_level_integer_refusal_does_not_acquire_a_guest_string_result() {
        for error in [
            ValueError::ScalarNumericInputUnavailable,
            ValueError::CommandProtocolUnavailable("native frame getter"),
        ] {
            let refusal = error.native_access_refusal().unwrap();
            let error = integer_getter_error(error, NativeStringProtocol::C(TclVersion::V9_1));
            assert_eq!(error.native_access_refusal(), Some(refusal));
            assert_eq!(error.into_byte_details().string_result, None);
        }
    }
}
