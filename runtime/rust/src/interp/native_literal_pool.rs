// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual C executable object arrays and their native global registrations.

use super::Interp;
use crate::{dict, list, obj};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use tcl_core_types::NsId;
use tcl_runtime_api::native_literal::{
    source_literal_action, NativeLiteralKey, NativeLiteralWorld, NativeSourceLiteralAction,
};
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

/// Already-admitted local-array allocations, in their original slot order.
/// The executable compiler owns admission and deduplication. This carrier
/// grants no compiler selection, command binding, or local-variable authority.
pub(crate) enum NativeRuntimeLiteral {
    RegisteredBytes {
        bytes: Vec<u8>,
        namespace: Option<NsId>,
    },
    UnsharedBytes(Vec<u8>),
    /// Actual C9.1 constant-concat String original retained by TclAddLiteralObj.
    PrivateConcatString(Vec<u8>),
    /// C8.5 temporary logical compiler registration, released after capture.
    PrivateLogicalBoolean85(bool),
    /// A genuine TclAddLiteralObj producer already owns this original header.
    /// The input owner expires after array acquisition; no type is inferred.
    UnsharedOriginal(obj::Owned),
    PrivateConstantList {
        members: Vec<Vec<u8>>,
        protocol: NativeStringProtocol,
    },
}

/// Already-selected chronological compiler actions. Local deduplication is
/// independent of these operations: data-first slots can subsequently be primed.
pub(crate) enum NativeRuntimeLiteralAction {
    Register(usize),
    RetainSyntaxErrorInfo {
        options: usize,
        message: usize,
    },
    AdoptExpressionNumber {
        index: usize,
        version: tcl_dialect::TclVersion,
        value: tcl_bytecode::NativeExpressionNumberLiteral,
    },
    PrimeExpressionBoolean84(usize),
    Hide(usize),
    PrimeCommandName {
        index: usize,
        receipt: Box<tcl_runtime_api::native_command_name::NativeCommandNamePriming>,
    },
}

/// One native Bytecode object array. Sharing this allocation retains no
/// additional native member refs. The array never retains the interpreter.
pub(crate) struct NativeRuntimeLiteralArray {
    values: Vec<RefCell<Option<obj::Owned>>>,
    registrations: Vec<Option<usize>>,
    world: Weak<RefCell<NativeLiteralWorld<obj::Owned>>>,
    protocol: NativeStringProtocol,
}

impl NativeRuntimeLiteralArray {
    /// Borrow an original array member for a live executor. The caller must
    /// keep this array alive until the borrowed object has been consumed.
    pub(crate) fn original(&self, index: usize) -> Option<*mut obj::TclObj> {
        self.values
            .get(index)?
            .borrow()
            .as_ref()
            .map(obj::Owned::as_ptr)
    }

    fn retire_registered_members(&self) {
        // Tcl84's global-table destruction withdraws its registered object
        // arrays. This breaks a real source->Bytecode->source cycle without
        // replacing the retained original source's Bytecode primary.
        if self.protocol == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) {
            for (value, registration) in self.values.iter().zip(&self.registrations) {
                if registration.is_some() {
                    drop(value.borrow_mut().take());
                }
            }
        }
    }
}

impl Drop for NativeRuntimeLiteralArray {
    fn drop(&mut self) {
        if let Some(world) = self.world.upgrade() {
            for registration in self.registrations.iter().flatten() {
                world.borrow_mut().release(*registration);
            }
        }
        // Rust drops the actual local member owners after the registrations.
    }
}

impl Interp {
    /// Observe this actual C interpreter's original empty registration world.
    /// No object getter or additional original reference is taken. Each capture
    /// expires earlier observations, even without a compiler-epoch change.
    pub fn capture_native_empty_literal_world(
        &self,
    ) -> Option<tcl_runtime_api::native_literal::NativeEmptyLiteralWorld> {
        let epoch = self.native_compiler_cache_epochs(self.current_ns.get())?.0;
        self.native_literal_world
            .borrow()
            .capture_empty_world(self.native_command_interpreter, epoch)
    }

    /// Allocate a compiler-admitted C local object array and finalize only
    /// actual source-pointer matches before attaching Bytecode to the source.
    #[cfg(test)]
    pub(crate) fn create_native_literal_array(
        &mut self,
        source: *mut obj::TclObj,
        literals: &[NativeRuntimeLiteral],
    ) -> Result<Rc<NativeRuntimeLiteralArray>, ValueError> {
        let actions: Vec<_> = (0..literals.len())
            .map(NativeRuntimeLiteralAction::Register)
            .collect();
        self.create_native_literal_array_with_actions(source, literals, &actions)
    }

    pub(crate) fn create_native_literal_array_with_actions(
        &mut self,
        source: *mut obj::TclObj,
        literals: &[NativeRuntimeLiteral],
        actions: &[NativeRuntimeLiteralAction],
    ) -> Result<Rc<NativeRuntimeLiteralArray>, ValueError> {
        let dialect = self.native_invocation_dialect();
        let protocol = dialect
            .native_string_protocol()
            .filter(|p| p.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "actual C literal array",
            ))?;
        // Validate every private allocation before acquiring any global leases.
        for literal in literals {
            if let NativeRuntimeLiteral::PrivateConstantList {
                protocol: supplied, ..
            } = literal
            {
                if *supplied != protocol
                    || protocol
                        .tcl_version()
                        .is_none_or(|version| version < tcl_dialect::TclVersion::V8_6)
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native constant List allocation",
                    ));
                }
            }
        }
        let mut array = NativeRuntimeLiteralArray {
            values: Vec::with_capacity(literals.len()),
            registrations: Vec::with_capacity(literals.len()),
            world: Rc::downgrade(&self.native_literal_world),
            protocol,
        };
        for action in actions {
            let index = match action {
                NativeRuntimeLiteralAction::RetainSyntaxErrorInfo { options, message } => {
                    if protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1) {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "C91 Syntax original error-info issuer",
                        ));
                    }
                    let message =
                        array
                            .original(*message)
                            .ok_or(ValueError::CommandProtocolUnavailable(
                                "Syntax message before registration",
                            ))?;
                    let options_original =
                        array
                            .original(*options)
                            .ok_or(ValueError::CommandProtocolUnavailable(
                                "Syntax options before registration",
                            ))?;
                    let key = obj::Owned::fresh(obj::new_string_bytes(b"-errorinfo"));
                    let mut dictionary = crate::dict::PreparedNativeDictionary::prepare(
                        Some(options_original),
                        protocol,
                    )?;
                    dictionary.set_member(key.as_ptr(), message)?;
                    *array.values[*options].borrow_mut() = Some(dictionary.into_value());
                    self.publish_original_compiler_syntax(
                        message,
                        array.original(*options).expect("installed Syntax options"),
                    )?;
                    continue;
                }
                NativeRuntimeLiteralAction::PrimeExpressionBoolean84(index) => {
                    if protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4) {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "C84 Boolean literal issuer",
                        ));
                    }
                    let original =
                        array
                            .original(*index)
                            .ok_or(ValueError::CommandProtocolUnavailable(
                                "Boolean getter before literal registration",
                            ))?;
                    crate::typed_value::native_boolean(
                        original,
                        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
                    )?;
                    continue;
                }
                NativeRuntimeLiteralAction::AdoptExpressionNumber {
                    index,
                    version,
                    value,
                } => {
                    if protocol.tcl_version() != Some(*version)
                        || *version < tcl_dialect::TclVersion::V8_5
                    {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "native constant cache transfer issuer",
                        ));
                    }
                    let original =
                        array
                            .original(*index)
                            .ok_or(ValueError::CommandProtocolUnavailable(
                                "constant cache transfer before registration",
                            ))?;
                    if obj::obj_type_ptr(original).is_null() {
                        obj::adopt_native_scalar_cache(
                            original,
                            tcl_syntax::scalar_getter::NativeScalarCache::Number(value.number()),
                            tcl_registry::InvocationDialect::for_version(*version)
                                .native_scalar_getter_protocol()
                                .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                        )?;
                    }
                    continue;
                }
                NativeRuntimeLiteralAction::Register(index) => *index,
                NativeRuntimeLiteralAction::Hide(index) => {
                    if protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5) {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "native hidden literal action",
                        ));
                    }
                    let original =
                        array
                            .original(*index)
                            .ok_or(ValueError::CommandProtocolUnavailable(
                                "hidden literal before allocation",
                            ))?;
                    // TclHideLiteral makes a full ordinary header duplicate;
                    // source-cycle prevention is a later String-only operation.
                    let hidden = obj::Owned::fresh(obj::duplicate(original));
                    if let Some(registration) = array.registrations[*index].take() {
                        self.native_literal_world.borrow_mut().release(registration);
                    }
                    *array.values[*index].borrow_mut() = Some(hidden);
                    continue;
                }
                NativeRuntimeLiteralAction::PrimeCommandName { index, receipt } => {
                    let original =
                        array
                            .original(*index)
                            .ok_or(ValueError::CommandProtocolUnavailable(
                                "command priming before allocation",
                            ))?;
                    let origin = usize::try_from(receipt.context.namespace_token)
                        .ok()
                        .ok_or(ValueError::CommandProtocolUnavailable(
                            "native literal namespace token",
                        ))?;
                    if receipt.context.interpreter != self.native_command_interpreter
                        || protocol.tcl_version() != Some(receipt.version)
                        || self
                            .namespaces
                            .borrow()
                            .native_context_path(origin)
                            .as_ref()
                            != Some(&receipt.context.namespace_path)
                        || self
                            .native_compiler_cache_epochs(origin)
                            .map(|epochs| epochs.0)
                            != Some(receipt.context.entry_epoch)
                    {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "stale or foreign command literal priming context",
                        ));
                    }
                    let name_protocol = dialect.native_command_name_protocol().ok_or(
                        ValueError::CommandProtocolUnavailable("native command literal priming"),
                    )?;
                    let mut cache = match &receipt.authority {
                        tcl_runtime_api::native_command_name::NativeCommandNamePrimingAuthority::OriginalLookup => self
                            .native_command_name_from_binding(origin, receipt.original.as_bytes())?
                            .ok_or(ValueError::CommandProtocolUnavailable("retired compiled command literal binding"))?,
                        tcl_runtime_api::native_command_name::NativeCommandNamePrimingAuthority::CompilerSelected(required) => {
                            let registrations_match = required.matches_registration_with(|namespace, word| {
                                self.native_compilation_binding_at(usize::try_from(namespace).map_err(|_| ValueError::CommandProtocolUnavailable("native selected compiler lookup namespace"))?, word.as_bytes())
                            })?;
                            if required.interpreter != receipt.context.interpreter
                                || required.selected_worker.as_ref() != Some(&receipt.binding)
                                || !registrations_match
                            {
                                return Err(ValueError::CommandProtocolUnavailable("stale original selected-worker compiler priming"));
                            }
                            self.namespaces.borrow().native_command_name_cache_from_binding(
                                self.native_command_interpreter, &receipt.binding, name_protocol,
                            ).ok_or(ValueError::CommandProtocolUnavailable("retired original selected worker"))?
                        }
                    };
                    if cache.token != receipt.binding.token
                        || cache.implementation_generation
                            != receipt.binding.implementation_generation
                    {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "replaced compiled command literal binding",
                        ));
                    }
                    cache.reference = name_protocol.priming_reference(
                        receipt.fully_qualified,
                        self.namespaces.borrow().native_command_reference(origin),
                    );
                    obj::prime_native_command_name_cache(original, cache, dialect)?;
                    continue;
                }
            };
            if index != array.values.len() {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native literal allocation order",
                ));
            }
            let literal = literals
                .get(index)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native literal slot",
                ))?;
            let (registration, value) = match literal {
                NativeRuntimeLiteral::RegisteredBytes { bytes, namespace } => {
                    let (index, value) = self.native_literal_world.borrow_mut().register(
                        NativeLiteralKey {
                            protocol,
                            namespace: *namespace,
                            original: bytes.clone(),
                        },
                        |value| dict::native_object_bytes(value.as_ptr(), protocol),
                        || registered_string(bytes, dialect),
                    )?;
                    (Some(index), value)
                }
                NativeRuntimeLiteral::UnsharedBytes(bytes) => {
                    (None, obj::Owned::fresh(obj::new_string_bytes(bytes)))
                }
                NativeRuntimeLiteral::PrivateConcatString(bytes) => {
                    if protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1) {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "native constant concat String issuer",
                        ));
                    }
                    let original = obj::Owned::fresh(obj::new_string_bytes(bytes));
                    obj::set_native_append_string(
                        original.as_ptr(),
                        protocol,
                        Some((
                            std::rc::Rc::from(bytes.as_slice()),
                            tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                        )),
                        None,
                        None,
                    )?;
                    (None, original)
                }
                NativeRuntimeLiteral::PrivateLogicalBoolean85(value) => {
                    if protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5) {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "native C85 logical fold origin",
                        ));
                    }
                    let bytes = if *value { b"1" } else { b"0" };
                    let (registration, original) =
                        self.native_literal_world.borrow_mut().register(
                            NativeLiteralKey {
                                protocol,
                                namespace: None,
                                original: bytes.to_vec(),
                            },
                            |value| dict::native_object_bytes(value.as_ptr(), protocol),
                            || registered_string(bytes, dialect),
                        )?;
                    self.native_literal_world.borrow_mut().release(registration);
                    (None, original)
                }
                NativeRuntimeLiteral::UnsharedOriginal(original) => (None, original.clone()),
                NativeRuntimeLiteral::PrivateConstantList { members, .. } => {
                    let children: Vec<_> = members
                        .iter()
                        .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
                        .collect();
                    let pointers: Vec<_> = children.iter().map(obj::Owned::as_ptr).collect();
                    (
                        None,
                        obj::Owned::fresh(list::new_list_obj_native(&pointers, protocol)),
                    )
                }
            };
            array.registrations.push(registration);
            array.values.push(RefCell::new(Some(value)));
        }
        if array.values.len() != literals.len() {
            return Err(ValueError::CommandProtocolUnavailable(
                "unallocated native literal slot",
            ));
        }
        for slot in 0..array.values.len() {
            if array.original(slot) != Some(source) {
                continue;
            }
            match source_literal_action(protocol, true) {
                NativeSourceLiteralAction::CopySourceString => {
                    let bytes = dict::native_object_bytes(source, protocol)?;
                    let copied = obj::Owned::fresh(obj::new_string_bytes(&bytes));
                    if let Some(registration) = array.registrations[slot].take() {
                        self.native_literal_world.borrow_mut().release(registration);
                    }
                    *array.values[slot].borrow_mut() = Some(copied);
                }
                NativeSourceLiteralAction::RetainSourceCycle => {
                    let registration =
                        array.registrations[slot].ok_or(ValueError::CommandProtocolUnavailable(
                            "unregistered C84 source self-reference",
                        ))?;
                    if !self
                        .native_literal_world
                        .borrow()
                        .registration_value(registration)
                        .is_some_and(|value| value.as_ptr() == source)
                    {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "foreign C84 source registration",
                        ));
                    }
                }
                NativeSourceLiteralAction::UnsupportedSelfReference => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native source self-reference producer",
                    ));
                }
                NativeSourceLiteralAction::Preserve => unreachable!("actual source-pointer match"),
            }
        }
        let array = Rc::new(array);
        self.native_literal_arrays
            .borrow_mut()
            .push(Rc::downgrade(&array));
        Ok(array)
    }
}

pub(super) fn registered_string(
    bytes: &[u8],
    dialect: tcl_registry::InvocationDialect,
) -> Result<obj::Owned, ValueError> {
    let value = obj::Owned::fresh(obj::new_string_bytes(bytes));
    if dialect.native_string_protocol()
        == Some(NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4))
    {
        if let Some(integer) = tcl_runtime_api::native_literal::registered_c84_long(
            bytes,
            (std::mem::size_of::<std::os::raw::c_long>() * 8) as u8,
        ) {
            obj::adopt_native_scalar_cache(
                value.as_ptr(),
                tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(integer),
                dialect
                    .native_scalar_getter_protocol()
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?,
            )?;
        }
    }
    Ok(value)
}

pub(super) fn retire_arrays(arrays: &RefCell<Vec<Weak<NativeRuntimeLiteralArray>>>) {
    let retained: Vec<_> = arrays.borrow().iter().filter_map(Weak::upgrade).collect();
    for array in retained {
        array.retire_registered_members();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_literal_world_capture_has_actual_owner_epoch_and_retirement() {
        for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(version),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("actual C core");
            let epoch = interp
                .native_compiler_cache_epochs(interp.current_ns.get())
                .unwrap()
                .0;
            let first = interp.capture_native_empty_literal_world().expect(version);
            assert!(first.is_current_for(interp.native_command_interpreter, epoch));
            assert!(!first.is_current_for(interp.native_command_interpreter, epoch + 1));
            let second = interp.capture_native_empty_literal_world().unwrap();
            assert!(!first.is_current());
            let source = obj::Owned::fresh(obj::new_string_bytes(b"original source"));
            let array = interp
                .create_native_literal_array(
                    source.as_ptr(),
                    &[NativeRuntimeLiteral::RegisteredBytes {
                        bytes: b"held\0original".to_vec(),
                        namespace: None,
                    }],
                )
                .unwrap();
            assert!(!second.is_current());
            let original = array.original(0).unwrap();
            let references = unsafe { (*original).ref_count };
            let primary = obj::obj_type_ptr(original);
            assert!(interp.capture_native_empty_literal_world().is_none());
            assert_eq!(unsafe { (*original).ref_count }, references);
            assert_eq!(obj::obj_type_ptr(original), primary);
            drop(array);
            let current = interp.capture_native_empty_literal_world().unwrap();
            assert!(current.is_current());
            drop(interp);
            assert!(!current.is_current());
        }
    }

    #[test]
    fn syntax_compiler_reset_publishes_original_without_a_global_literal_owner() {
        let native = include_str!("../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-lifecycle-9.1.0.tsv")
            .lines().find(|line| line.starts_with("0\tcompiled\t")).unwrap()
            .split('\t').collect::<Vec<_>>();
        let mut interp = interp("tcl9.1");
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
        let source = obj::Owned::fresh(obj::new_string_bytes(b"original body"));
        let options = crate::native_return_merge::manufacture(
            &tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral {
                protocol,
                words: vec![
                    tcl_runtime_api::native_return_literal::NativeKnownWordLiteral {
                        pieces: vec![b"-errorcode".to_vec()],
                        composite: false,
                    },
                    tcl_runtime_api::native_return_literal::NativeKnownWordLiteral {
                        pieces: vec![b"ARITH DIVZERO {divide by zero}".to_vec()],
                        composite: false,
                    },
                ],
                code: 0,
                level: 1,
                size: 1,
            },
        )
        .unwrap();
        let declarations = [
            NativeRuntimeLiteral::UnsharedBytes(b"divide by zero".to_vec()),
            NativeRuntimeLiteral::UnsharedOriginal(options),
        ];
        let array = interp
            .create_native_literal_array_with_actions(
                source.as_ptr(),
                &declarations,
                &[
                    NativeRuntimeLiteralAction::Register(0),
                    NativeRuntimeLiteralAction::Register(1),
                    NativeRuntimeLiteralAction::RetainSyntaxErrorInfo {
                        message: 0,
                        options: 1,
                    },
                ],
            )
            .unwrap();
        drop(declarations);
        let original = array.original(0).unwrap();
        assert!(array.registrations[0].is_none());
        assert_eq!(interp.var_get(b"::errorInfo").unwrap(), original);
        assert_eq!(
            unsafe { (*original).ref_count },
            native[2].parse::<isize>().unwrap()
        );
        assert!(interp.exc.borrow().native.info.is_none());
        assert!(interp.exc.borrow().native.code.is_none());
        let replacement = obj::Owned::fresh(obj::new_string_bytes(b"later error"));
        interp
            .var_set(b"::errorInfo", replacement.as_ptr())
            .unwrap();
        assert_eq!(unsafe { (*original).ref_count }, 2);
        let pairs = dict::native_dict_pairs(array.original(1).unwrap(), protocol).unwrap();
        assert!(pairs
            .iter()
            .any(|(key, value)| obj::bytes_of(*key) == b"-errorinfo" && *value == original));
    }

    fn interp(version: &str) -> Interp {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(version));
        interp
    }

    #[test]
    fn c85_folded_logical_original_shares_only_a_live_registration() {
        for registered in [false, true] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect("tcl8.5"),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .expect("actual C85 constructor");
            let source = obj::Owned::fresh(obj::new_string_bytes(b"original body"));
            let mut declarations = Vec::new();
            if registered {
                declarations.push(NativeRuntimeLiteral::RegisteredBytes {
                    bytes: b"1".to_vec(),
                    namespace: None,
                });
            }
            let first = declarations.len();
            declarations.push(NativeRuntimeLiteral::PrivateLogicalBoolean85(true));
            declarations.push(NativeRuntimeLiteral::PrivateLogicalBoolean85(true));
            let array = interp
                .create_native_literal_array(source.as_ptr(), &declarations)
                .unwrap();
            let original = array.original(first).unwrap();
            assert_eq!(Some(original) == array.original(first + 1), registered);
            assert_eq!(
                unsafe { (*original).ref_count },
                if registered { 4 } else { 1 }
            );
            assert!(obj::obj_type_ptr(original).is_null());
        }
    }
    #[test]
    fn native_arrays_share_registered_headers_and_release_global_before_local() {
        for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interp(version);
            let source = obj::Owned::fresh(obj::new_string_bytes(b"source"));
            let declarations = [NativeRuntimeLiteral::RegisteredBytes {
                bytes: b"17\0tail".to_vec(),
                namespace: None,
            }];
            let first = interp
                .create_native_literal_array(source.as_ptr(), &declarations)
                .unwrap();
            let member = obj::Owned::retain(first.original(0).unwrap());
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 3);
            let second = interp
                .create_native_literal_array(source.as_ptr(), &declarations)
                .unwrap();
            assert_eq!(first.original(0), second.original(0));
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 4);
            if version == "tcl8.4" {
                assert!(matches!(
                    obj::native_scalar_cache(member.as_ptr()).unwrap(),
                    Some(tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(17))
                ));
            } else {
                assert!(obj::obj_type_ptr(member.as_ptr()).is_null());
            }
            drop(first);
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 3);
            drop(second);
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 1);
        }
    }
    #[test]
    fn source_finalization_uses_original_pointer_and_all_c84_registered_slots() {
        for version in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interp(version);
            let source = obj::Owned::fresh(obj::new_string_bytes(b"incoming"));
            let literal = NativeRuntimeLiteral::RegisteredBytes {
                bytes: b"x".to_vec(),
                namespace: None,
            };
            let seed = interp
                .create_native_literal_array(source.as_ptr(), &[literal])
                .unwrap();
            let original = obj::Owned::retain(seed.original(0).unwrap());
            let declarations: Vec<_> = [b"0".as_slice(), b"x", b"2", b"3", b"4", b"5", b"6", b"7"]
                .iter()
                .map(|bytes| NativeRuntimeLiteral::RegisteredBytes {
                    bytes: bytes.to_vec(),
                    namespace: None,
                })
                .collect();
            let array = interp
                .create_native_literal_array(original.as_ptr(), &declarations)
                .unwrap();
            if version == "tcl8.4" {
                assert_eq!(array.original(1), Some(original.as_ptr()));
                retire_arrays(&interp.native_literal_arrays);
                assert!((0..8).all(|slot| array.original(slot).is_none()));
                assert!(seed.original(0).is_none());
            } else {
                assert_ne!(array.original(1), Some(original.as_ptr()));
                assert_eq!(obj::bytes_of(array.original(1).unwrap()), b"x");
                assert!(obj::obj_type_ptr(array.original(1).unwrap()).is_null());
            }
        }
    }
    #[test]
    fn c85_refuses_a_non_native_self_producer_without_changing_original_storage() {
        let mut interp = interp("tcl8.5");
        let source = obj::Owned::fresh(obj::new_string_bytes(b"incoming"));
        let literals = [NativeRuntimeLiteral::RegisteredBytes {
            bytes: b"x".to_vec(),
            namespace: None,
        }];
        let seed = interp
            .create_native_literal_array(source.as_ptr(), &literals)
            .unwrap();
        let original = obj::Owned::retain(seed.original(0).unwrap());
        assert!(interp
            .create_native_literal_array(original.as_ptr(), &literals)
            .is_err());
        assert_eq!(seed.original(0), Some(original.as_ptr()));
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 3);
        assert!(obj::obj_type_ptr(original.as_ptr()).is_null());
        assert_eq!(obj::bytes_of(original.as_ptr()), b"x");
    }
    #[test]
    fn c85_hidden_one_word_literal_duplicates_then_unregisters_before_source_publication() {
        let mut interp = interp("tcl8.5");
        let source = obj::Owned::fresh(obj::new_string_bytes(b"incoming"));
        let literals = [NativeRuntimeLiteral::RegisteredBytes {
            bytes: b"x".to_vec(),
            namespace: None,
        }];
        let seed = interp
            .create_native_literal_array(source.as_ptr(), &literals)
            .unwrap();
        let original = obj::Owned::retain(seed.original(0).unwrap());
        let array = interp
            .create_native_literal_array_with_actions(
                original.as_ptr(),
                &literals,
                &[
                    NativeRuntimeLiteralAction::Register(0),
                    NativeRuntimeLiteralAction::Hide(0),
                ],
            )
            .unwrap();
        assert_ne!(array.original(0), Some(original.as_ptr()));
        assert_eq!(obj::bytes_of(array.original(0).unwrap()), b"x");
        assert_eq!(unsafe { (*array.original(0).unwrap()).ref_count }, 1);
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 3);
        assert!(array.registrations[0].is_none());
    }
    #[test]
    fn added_original_literal_keeps_the_same_header_and_exact_local_reference() {
        for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interp(version);
            let source = obj::Owned::fresh(obj::new_string_bytes(b"source"));
            let original = obj::Owned::fresh(obj::new_string_bytes(b"compiler message"));
            let pointer = original.as_ptr();
            let literals = [NativeRuntimeLiteral::UnsharedOriginal(original)];
            let array = interp
                .create_native_literal_array(source.as_ptr(), &literals)
                .unwrap();
            assert_eq!(array.original(0), Some(pointer));
            assert_eq!(unsafe { (*pointer).ref_count }, 2);
            drop(literals);
            assert_eq!(unsafe { (*pointer).ref_count }, 1);
            let retained = obj::Owned::retain(pointer);
            drop(array);
            assert_eq!(unsafe { (*retained.as_ptr()).ref_count }, 1);
            assert_eq!(obj::bytes_of(retained.as_ptr()), b"compiler message");
        }
    }

    #[test]
    fn c91_constant_concat_retains_private_allocated_string_headers() {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        use tcl_syntax::native_string::NativeStringStorageIdentity as Storage;
        let mut interp = interp("tcl9.1");
        let source = obj::Owned::fresh(obj::new_string_bytes(b"original concat body"));
        let array = interp
            .create_native_literal_array(
                source.as_ptr(),
                &[
                    NativeRuntimeLiteral::RegisteredBytes {
                        bytes: Vec::new(),
                        namespace: None,
                    },
                    NativeRuntimeLiteral::UnsharedBytes(Vec::new()),
                    NativeRuntimeLiteral::PrivateConcatString(Vec::new()),
                    NativeRuntimeLiteral::PrivateConcatString(b"A B".to_vec()),
                ],
            )
            .unwrap();
        for index in [2, 3] {
            let original = array.original(index).unwrap();
            let snapshot = obj::native_object_snapshot(original).unwrap();
            assert!(matches!(
                snapshot.cache,
                Cache::String {
                    num_chars: None,
                    unicode: None,
                    ..
                }
            ));
            assert_eq!(snapshot.storage, Some(Storage::Allocated));
            assert_eq!(unsafe { (*original).ref_count }, 1);
            assert!(array.registrations[index].is_none());
        }
        let empty = array.original(1).unwrap();
        assert!(obj::obj_type_ptr(empty).is_null());
        assert_ne!(array.original(0), array.original(1));
        assert_ne!(array.original(1), array.original(2));
    }

    #[test]
    fn private_list_members_are_fresh_originals_not_global_string_registrations() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interp(version);
            let protocol = interp
                .native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            let source = obj::Owned::fresh(obj::new_string_bytes(b"source"));
            let array = interp
                .create_native_literal_array(
                    source.as_ptr(),
                    &[
                        NativeRuntimeLiteral::RegisteredBytes {
                            bytes: b"member".to_vec(),
                            namespace: None,
                        },
                        NativeRuntimeLiteral::PrivateConstantList {
                            members: vec![b"member".to_vec()],
                            protocol,
                        },
                    ],
                )
                .unwrap();
            let elements =
                list::list_elements_native_checked(array.original(1).unwrap(), protocol).unwrap();
            assert_ne!(elements[0], array.original(0).unwrap());
            assert_eq!(unsafe { (*elements[0]).ref_count }, 1);
            assert!(!obj::has_string_rep(array.original(1).unwrap()));
        }
    }
}
