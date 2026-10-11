// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native variable-name primaries retain original parts and concrete local owners.

use super::{IntRep, Value};
use std::rc::Rc;
use tcl_syntax::native_variable_name::{
    NativeLocalVariableName, NativeLocalVariableOwner, NativeParsedVariableElement,
    NativeParsedVariableName,
};
use tcl_syntax::value::ValueError;

pub(crate) type ParsedVariableName = NativeParsedVariableName<Value>;
pub(crate) type LocalVariableName =
    NativeLocalVariableName<crate::command::NativeProcedureReference, Value>;

impl Value {
    pub(crate) fn with_native_parsed_variable<R>(
        &self,
        read: impl FnOnce(&ParsedVariableName) -> R,
    ) -> Option<R> {
        match &*self.0.intrep.borrow() {
            IntRep::NativeParsedVariableName(cache) => Some(read(cache)),
            _ => None,
        }
    }
    pub(crate) fn with_native_local_variable<R>(
        &self,
        read: impl FnOnce(&LocalVariableName) -> R,
    ) -> Option<R> {
        match &*self.0.intrep.borrow() {
            IntRep::NativeLocalVariableName(cache) => Some(read(cache)),
            _ => None,
        }
    }
    pub(crate) fn install_native_parsed_variable_checked<E: From<ValueError>>(
        &self,
        dialect: tcl_registry::InvocationDialect,
        build: impl FnOnce() -> Result<ParsedVariableName, E>,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        let protocol = dialect.native_variable_name_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("native parsed variable issuer"),
        )?;
        if self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native parsed variable resident spelling",
            )
            .into());
        }
        self.replace_primary_using_with(
            || {
                let cache = build()?;
                let shape = cache.array.as_ref().is_none_or(|(_, element)| {
                    matches!(
                        (element, protocol.has_parsed_array_updater()),
                        (NativeParsedVariableElement::Bytes(_), true)
                            | (NativeParsedVariableElement::Object(_), false)
                    )
                });
                if protocol != cache.protocol || self.resident_string_bytes().is_none() || !shape {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native parsed variable origin, shape or storage",
                    )
                    .into());
                }
                Ok(IntRep::NativeParsedVariableName(cache))
            },
            &mut Some(check),
        )
    }

    pub(crate) fn install_native_local_variable_checked<E: From<ValueError>>(
        &self,
        dialect: tcl_registry::InvocationDialect,
        build: impl FnOnce() -> Result<LocalVariableName, E>,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        let protocol = dialect.native_variable_name_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("native local variable issuer"),
        )?;
        if self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native local variable resident spelling",
            )
            .into());
        }
        self.replace_primary_using_with(
            || {
                let cache = build()?;
                let shape = matches!(
                    (&cache.owner, protocol.local_cache_owns_procedure()),
                    (NativeLocalVariableOwner::Procedure(_), true)
                        | (NativeLocalVariableOwner::Name(_), false)
                );
                if protocol != cache.protocol || !shape || self.resident_string_bytes().is_none() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native local variable origin, shape or storage",
                    )
                    .into());
                }
                Ok(IntRep::NativeLocalVariableName(cache))
            },
            &mut Some(check),
        )
    }

    pub(crate) fn retire_native_variable_primary_checked<E: From<ValueError>>(
        &self,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.replace_primary_using_with(|| Ok(IntRep::Str), &mut Some(check))
    }

    pub(crate) fn native_primary_has_free_hook(&self) -> bool {
        !matches!(
            &*self.0.intrep.borrow(),
            IntRep::Str
                | IntRep::Int(_)
                | IntRep::Tcl84Long(_)
                | IntRep::Double(_)
                | IntRep::Bool(_)
                | IntRep::WordBoolean { .. }
                | IntRep::NativeArraySearch { .. }
                | IntRep::FrameLevel { .. }
                | IntRep::NativeInstructionName(_)
        )
    }
    pub(crate) fn copy_native_variable_primary_to(&self, duplicate: &Self) {
        let primary = match &*self.0.intrep.borrow() {
            IntRep::NativeParsedVariableName(cache) => Some(IntRep::NativeParsedVariableName(
                cache
                    .duplicate()
                    .expect("authenticated parsed variable shape"),
            )),
            IntRep::NativeLocalVariableName(cache) => {
                let mut duplicate = cache.clone();
                if matches!(duplicate.owner, NativeLocalVariableOwner::Name(None)) {
                    duplicate.owner = NativeLocalVariableOwner::Name(Some(self.clone()));
                }
                Some(IntRep::NativeLocalVariableName(duplicate))
            }
            _ => None,
        };
        if let Some(primary) = primary {
            *duplicate.0.intrep.borrow_mut() = primary;
        }
    }
    pub(crate) fn parsed_variable_string_bytes(
        &self,
        protocol: super::NativeStringProtocol,
    ) -> Option<Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable>> {
        if let Some(result) = self.with_native_local_variable(|cache| {
            let unavailable = tcl_syntax::native_string::NativeStringUnavailable::StringUpdater;
            if protocol != super::NativeStringProtocol::C(cache.protocol.version()) {
                return Err(unavailable);
            }
            let NativeLocalVariableOwner::Procedure(owner) = &cache.owner else {
                return Err(unavailable);
            };
            owner
                .declaration()
                .native_compiled_name(cache.index)
                .map(Rc::from)
        }) {
            return Some(result);
        }
        self.with_native_parsed_variable(|cache| {
            let unavailable = tcl_syntax::native_string::NativeStringUnavailable::StringUpdater;
            if protocol != super::NativeStringProtocol::C(cache.protocol.version()) {
                return Err(unavailable);
            }
            let Some((root, NativeParsedVariableElement::Bytes(element))) = &cache.array else {
                return Err(unavailable);
            };
            let root = root.native_string_bytes(protocol)?;
            cache
                .protocol
                .parsed_array_string(&root, element)
                .map(Rc::from)
                .ok_or(unavailable)
        })
    }
}
