// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Interpreter literal registrations and retained bytecode object-array leases.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use tcl_bytecode::{LiteralTable, NativeLiteralAction, NativeLiteralAllocation};
#[cfg(test)]
use tcl_runtime_api::ROOT_NS;
use tcl_runtime_api::native_command_name::{NativeCommandNamePriming, NativeLiteralContext};
use tcl_runtime_api::{ByteNamespacePath, NsId};
use tcl_syntax::native_string::NativeStringProtocol;

use crate::Value;

#[derive(Clone, Debug)]
pub(crate) struct NativeLiteralUnavailable(&'static str);

impl NativeLiteralUnavailable {
    pub(crate) const fn unavailable(reason: &'static str) -> Self {
        Self(reason)
    }
    pub(crate) const fn uninitialized() -> Self {
        Self("native bytecode literal object array is not initialized")
    }
}

pub(crate) type NativeLiteralPoolReceipt = Result<Rc<NativeLiteralPool>, NativeLiteralUnavailable>;

/// Reached interpreter effects during chronological compiler array construction.
pub(crate) enum NativeLiteralEffect<'a> {
    PrimeCommandName(&'a NativeCommandNamePriming, &'a Value),
    /// C9.1 publishes the actual compile error fields when resetting its result.
    PublishSyntax {
        message: &'a Value,
        options: &'a Value,
    },
}

impl std::fmt::Display for NativeLiteralUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

use tcl_runtime_api::native_literal::NativeLiteralKey as GlobalKey;

#[derive(Default)]
pub(crate) struct NativeLiteralWorld {
    shared: Rc<RefCell<tcl_runtime_api::native_literal::NativeLiteralWorld<Value>>>,
    pools: Vec<Weak<NativeLiteralPool>>,
}
impl NativeLiteralWorld {
    pub(crate) fn invalidate_command_literal(
        &self,
        protocol: NativeStringProtocol,
        namespace: NsId,
        name: &[u8],
    ) {
        if protocol
            .tcl_version()
            .is_none_or(|version| version < tcl_dialect::TclVersion::V8_6)
        {
            return;
        }
        let key = GlobalKey {
            protocol,
            namespace: Some(namespace),
            original: tcl_core_types::c_string_extent(name).to_vec(),
        };
        for original in self.shared.borrow().key_values(&key) {
            original.retire_native_command_name_cache();
        }
    }
    fn register(
        &mut self,
        key: GlobalKey,
        protocol: NativeStringProtocol,
    ) -> Result<(usize, Value), NativeLiteralUnavailable> {
        let original = key.original.clone();
        self.shared.borrow_mut().register(
            key,
            |value| {
                value
                    .native_string_bytes(protocol)
                    .map(|bytes| bytes.to_vec())
                    .map_err(|_| {
                        NativeLiteralUnavailable("native literal string updater is unavailable")
                    })
            },
            || new_registered_string(&original, protocol),
        )
    }
    fn release(&mut self, index: usize) {
        self.shared.borrow_mut().release(index);
    }
}

impl Drop for NativeLiteralWorld {
    fn drop(&mut self) {
        // C84 TclDeleteLiteralTable force-releases every remaining global
        // registration, including its local array owners. Clear the retained
        // slots before those global objects are released so a one-self-literal
        // Bytecode cache can survive as native Bytecode without a Rust Rc cycle.
        for pool in self.pools.iter().filter_map(Weak::upgrade) {
            if pool.protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) {
                continue;
            }
            for (slot, registration) in pool.values.iter().zip(&pool.registrations) {
                if registration.is_some() {
                    let retired = slot.borrow_mut().take();
                    drop(retired);
                }
            }
        }
    }
}

// Original TclRegisterLiteral namespace partitions differ by native release.
fn command_partition(
    protocol: NativeStringProtocol,
    namespace: NsId,
    fully_qualified: bool,
) -> Option<NsId> {
    tcl_runtime_api::native_literal::command_literal_partition(protocol, namespace, fully_qualified)
}

// Tcl8.4 TclRegisterLiteral primes only canonical native-long decimal spelling.
// TclLooksLikeInt accepts the numeric CString prefix, including counted NUL tails;
// its absolute-value parser excludes LONG_MIN. This is producer initialization,
// independent of primitive getter acceptance or semantic numeric type hints.
fn new_registered_string(
    bytes: &[u8],
    protocol: NativeStringProtocol,
) -> Result<Value, NativeLiteralUnavailable> {
    if protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4)
        && let Some(value) = tcl_runtime_api::native_literal::registered_c84_long(
            bytes,
            u8::try_from(std::mem::size_of::<std::os::raw::c_long>() * 8)
                .expect("native C long width fits u8"),
        )
    {
        return Value::from_native_scalar_cache_with_storage(
            tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(value),
            None,
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
            Some((
                Rc::from(bytes),
                tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
            )),
        )
        .map_err(|_| {
            NativeLiteralUnavailable("native registered integer literal storage is unavailable")
        });
    }
    Ok(Value::new_native_string_bytes(bytes))
}

/// One native `ByteCode` local array, retained across activations and clones.
/// The lease drops global registration references before local object owners.
pub(crate) struct NativeLiteralPool {
    values: Vec<RefCell<Option<Value>>>,
    protocol: NativeStringProtocol,
    registrations: Vec<Option<usize>>,
    world: Weak<RefCell<NativeLiteralWorld>>,
}

impl Drop for NativeLiteralPool {
    fn drop(&mut self) {
        if let Some(world) = self.world.upgrade() {
            let mut world = world.borrow_mut();
            for &index in self.registrations.iter().flatten() {
                world.release(index);
            }
        }
    }
}

struct LiteralCreation<'a> {
    world: &'a Rc<RefCell<NativeLiteralWorld>>,
    namespace: NsId,
    source_namespace: &'a ByteNamespacePath,
    context: Option<&'a NativeLiteralContext>,
}

impl NativeLiteralPool {
    #[cfg(test)]
    pub(crate) fn reference_owners(&self, original: &Value) -> (usize, usize) {
        let local = self
            .values
            .iter()
            .filter(|value| {
                value
                    .borrow()
                    .as_ref()
                    .is_some_and(|value| value.is_same_object(original))
            })
            .count();
        let global = self.world.upgrade().map_or(0, |world| {
            world
                .borrow()
                .shared
                .borrow()
                .registered_values()
                .filter(|value| value.is_same_object(original))
                .count()
        });
        (local, global)
    }

    #[cfg(test)]
    pub(crate) fn create(
        world: &Rc<RefCell<NativeLiteralWorld>>,
        table: &LiteralTable,
        protocol: Option<NativeStringProtocol>,
        namespace: NsId,
        source_namespace: &ByteNamespacePath,
    ) -> Result<Rc<Self>, NativeLiteralUnavailable> {
        Self::create_with_actions(
            world,
            table,
            protocol,
            namespace,
            source_namespace,
            None,
            |_| {
                Err(NativeLiteralUnavailable(
                    "native command-name priming authority is unavailable",
                ))
            },
        )
    }

    /// Execute original allocation/cache actions under an independently validated context.
    pub(crate) fn create_with_actions(
        world: &Rc<RefCell<NativeLiteralWorld>>,
        table: &LiteralTable,
        protocol: Option<NativeStringProtocol>,
        namespace: NsId,
        source_namespace: &ByteNamespacePath,
        context: Option<&NativeLiteralContext>,
        mut effect: impl FnMut(NativeLiteralEffect<'_>) -> Result<(), NativeLiteralUnavailable>,
    ) -> Result<Rc<Self>, NativeLiteralUnavailable> {
        let protocol = protocol.ok_or(NativeLiteralUnavailable(
            "native literal constructor issuer is unavailable",
        ))?;
        let mut pool = Self {
            values: Vec::new(),
            protocol,
            registrations: Vec::new(),
            world: Rc::downgrade(world),
        };
        let input = LiteralCreation {
            world,
            namespace,
            source_namespace,
            context,
        };
        for action in table.native_actions() {
            if let NativeLiteralAction::Register(index) = action {
                if *index != pool.values.len() {
                    return Err(NativeLiteralUnavailable(
                        "native literal allocation order is inconsistent",
                    ));
                }
                let literal = table.entries().get(*index).ok_or(NativeLiteralUnavailable(
                    "native literal allocation has no object-array entry",
                ))?;
                let value = pool.allocate_literal(literal, &input)?;
                pool.values.push(RefCell::new(Some(value)));
            } else {
                pool.apply_literal_action(action, &input, &mut effect)?;
            }
        }
        let pool = Rc::new(pool);
        world.borrow_mut().pools.push(Rc::downgrade(&pool));
        Ok(pool)
    }

    fn apply_literal_action(
        &mut self,
        action: &NativeLiteralAction,
        input: &LiteralCreation<'_>,
        effect: &mut impl FnMut(NativeLiteralEffect<'_>) -> Result<(), NativeLiteralUnavailable>,
    ) -> Result<(), NativeLiteralUnavailable> {
        match action {
            NativeLiteralAction::RetainSyntaxErrorInfo { options, message } => {
                if self.protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1) {
                    return Err(NativeLiteralUnavailable(
                        "C91 Syntax original error-info issuer",
                    ));
                }
                let message_slot = self.values.get(*message).ok_or(NativeLiteralUnavailable(
                    "Syntax message before registration",
                ))?;
                let message = message_slot.borrow();
                let original_message = message
                    .as_ref()
                    .ok_or(NativeLiteralUnavailable("retired Syntax message slot"))?;
                let options_slot = self.values.get(*options).ok_or(NativeLiteralUnavailable(
                    "Syntax options before registration",
                ))?;
                let mut options = options_slot.borrow_mut();
                let original_options = options
                    .as_ref()
                    .ok_or(NativeLiteralUnavailable("retired Syntax options slot"))?;
                let updated = original_options
                    .native_dictionary_set_member(
                        Value::new_native_string_bytes(b"-errorinfo".as_slice()),
                        original_message.clone(),
                        self.protocol,
                    )
                    .map_err(|_| NativeLiteralUnavailable("Syntax original error-info member"))?;
                *options = Some(updated);
                effect(NativeLiteralEffect::PublishSyntax {
                    message: original_message,
                    options: options.as_ref().expect("installed Syntax options"),
                })?;
            }
            NativeLiteralAction::Hide(index) => {
                if self.protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_5) {
                    return Err(NativeLiteralUnavailable(
                        "native literal hiding recipe is unavailable",
                    ));
                }
                let value = self.values.get_mut(*index).ok_or(NativeLiteralUnavailable(
                    "native literal hiding precedes its object allocation",
                ))?;
                let value = value
                    .get_mut()
                    .as_mut()
                    .ok_or(NativeLiteralUnavailable("retired native literal slot"))?;
                let mut duplicate = value.duplicate_native_object_in(self.protocol);
                if let Some(resident) = value.resident_string_bytes() {
                    let storage = value.resident_string_storage_identity().ok_or(
                        NativeLiteralUnavailable(
                            "native hidden literal storage identity is unavailable",
                        ),
                    )?;
                    duplicate = duplicate
                        .with_resident_string_bytes_and_storage(
                            Rc::<[u8]>::from(resident.as_ref()),
                            storage,
                        )
                        .map_err(|_| {
                            NativeLiteralUnavailable(
                                "native hidden literal resident copy is unavailable",
                            )
                        })?;
                }
                *value = duplicate;
                if let Some(registration) = self.registrations[*index].take() {
                    input.world.borrow_mut().release(registration);
                }
            }
            _ => self.prime_literal_action(action, input, effect)?,
        }
        Ok(())
    }

    fn prime_literal_action(
        &mut self,
        action: &NativeLiteralAction,
        input: &LiteralCreation<'_>,
        effect: &mut impl FnMut(NativeLiteralEffect<'_>) -> Result<(), NativeLiteralUnavailable>,
    ) -> Result<(), NativeLiteralUnavailable> {
        match action {
            NativeLiteralAction::PrimeExpressionBoolean84(index) => {
                if self.protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4) {
                    return Err(NativeLiteralUnavailable("C84 Boolean literal issuer"));
                }
                let slot = self.values.get(*index).ok_or(NativeLiteralUnavailable(
                    "Boolean getter before literal registration",
                ))?;
                let borrowed = slot.borrow();
                let original = borrowed
                    .as_ref()
                    .ok_or(NativeLiteralUnavailable("retired Boolean literal slot"))?;
                original
                    .native_scalar_getter(
                        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
                        tcl_syntax::scalar_getter::NativeScalarGetterKind::Boolean,
                    )
                    .map_err(|_| NativeLiteralUnavailable("C84 Boolean literal conversion"))?;
            }
            NativeLiteralAction::AdoptExpressionNumber {
                index,
                version,
                value,
            } => {
                if self.protocol.tcl_version() != Some(*version) {
                    return Err(NativeLiteralUnavailable(
                        "native constant cache transfer issuer",
                    ));
                }
                let slot = self.values.get(*index).ok_or(NativeLiteralUnavailable(
                    "constant cache transfer before registration",
                ))?;
                let borrowed = slot.borrow();
                let original = borrowed
                    .as_ref()
                    .ok_or(NativeLiteralUnavailable("retired constant literal slot"))?;
                original
                    .adopt_native_expression_number_if_untyped(value.number(), *version)
                    .map_err(|_| {
                        NativeLiteralUnavailable("native constant cache transfer unavailable")
                    })?;
            }
            NativeLiteralAction::PrimeCommandName { index, receipt } => {
                if input.context != Some(&receipt.context)
                    || self.protocol.tcl_version() != Some(receipt.version)
                {
                    return Err(NativeLiteralUnavailable(
                        "native command-name priming context is stale or foreign",
                    ));
                }
                let slot = self.values.get(*index).ok_or(NativeLiteralUnavailable(
                    "native command-name priming precedes its object allocation",
                ))?;
                let borrowed = slot.borrow();
                let value = borrowed
                    .as_ref()
                    .ok_or(NativeLiteralUnavailable("retired native literal slot"))?;
                effect(NativeLiteralEffect::PrimeCommandName(receipt, value))?;
            }
            _ => unreachable!("non-priming literal action"),
        }
        Ok(())
    }

    fn allocate_literal(
        &mut self,
        literal: &tcl_bytecode::NativeStringLiteral,
        input: &LiteralCreation<'_>,
    ) -> Result<Value, NativeLiteralUnavailable> {
        Ok(match literal.allocation() {
            NativeLiteralAllocation::RegisteredData => {
                self.register(input.world, literal.bytes(), None, self.protocol)?
            }
            NativeLiteralAllocation::RegisteredCommand {
                namespace: selected,
                fully_qualified,
            } => {
                if selected != input.source_namespace {
                    return Err(NativeLiteralUnavailable(
                        "native command literal namespace receipt differs from its bytecode owner",
                    ));
                }
                let scope = command_partition(self.protocol, input.namespace, *fully_qualified);
                self.register(input.world, literal.bytes(), scope, self.protocol)?
            }
            NativeLiteralAllocation::RegisteredNativeCommand {
                context: selected,
                fully_qualified,
            } => {
                if input.context != Some(selected)
                    || selected.namespace_token != u64::from(input.namespace.0)
                    || &selected.namespace_path != input.source_namespace
                {
                    return Err(NativeLiteralUnavailable(
                        "native command literal has no matching namespace-token authority",
                    ));
                }
                let scope = command_partition(self.protocol, input.namespace, *fully_qualified);
                self.register(input.world, literal.bytes(), scope, self.protocol)?
            }
            _ => self.allocate_private_literal(literal, input)?,
        })
    }

    fn allocate_private_literal(
        &mut self,
        literal: &tcl_bytecode::NativeStringLiteral,
        input: &LiteralCreation<'_>,
    ) -> Result<Value, NativeLiteralUnavailable> {
        Ok(match literal.allocation() {
            NativeLiteralAllocation::Unshared => {
                self.registrations.push(None);
                Value::new_native_string_bytes(literal.bytes())
            }
            NativeLiteralAllocation::PrivateReturnOptions(recipe) => {
                if recipe.protocol != self.protocol {
                    return Err(NativeLiteralUnavailable(
                        "private Return literal origin mismatch",
                    ));
                }
                self.registrations.push(None);
                crate::native_return_merge::manufacture(recipe)
                    .map_err(|_| NativeLiteralUnavailable("private Return literal manufacture"))?
            }
            NativeLiteralAllocation::PrivateInteger(value) => {
                if self
                    .protocol
                    .tcl_version()
                    .is_none_or(|version| version < tcl_dialect::TclVersion::V9_1)
                {
                    return Err(NativeLiteralUnavailable(
                        "native private Integer compiler recipe is unavailable",
                    ));
                }
                self.registrations.push(None);
                Value::int(*value)
            }
            NativeLiteralAllocation::PrivateOriginal => {
                return Err(NativeLiteralUnavailable(
                    "native private original literal has no supplied object producer",
                ));
            }
            NativeLiteralAllocation::PrivateConstantList {
                members,
                protocol: selected,
            } => {
                if *selected != self.protocol
                    || self
                        .protocol
                        .tcl_version()
                        .is_none_or(|version| version < tcl_dialect::TclVersion::V8_6)
                {
                    return Err(NativeLiteralUnavailable(
                        "native private constant List compiler recipe is unavailable",
                    ));
                }
                self.registrations.push(None);
                let children = members
                    .iter()
                    .map(|bytes| Value::new_native_string_bytes(bytes.clone()))
                    .collect();
                Value::native_list_constructor(children, self.protocol)
            }
            _ => self.allocate_expression_literal(literal, input)?,
        })
    }

    fn allocate_expression_literal(
        &mut self,
        literal: &tcl_bytecode::NativeStringLiteral,
        input: &LiteralCreation<'_>,
    ) -> Result<Value, NativeLiteralUnavailable> {
        Ok(match literal.allocation() {
            NativeLiteralAllocation::PrivateConcatString => {
                if self.protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1) {
                    return Err(NativeLiteralUnavailable(
                        "native constant concat String issuer",
                    ));
                }
                let original = Value::from_native_string_cache(
                    tcl_syntax::native_object::NativeObjectCacheSnapshot::String {
                        protocol: self.protocol,
                        num_chars: None,
                        unicode: None,
                    },
                    tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1),
                    Some((
                        Rc::from(literal.bytes()),
                        tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                    )),
                )
                .map_err(|_| NativeLiteralUnavailable("native concat String backing"))?;
                self.registrations.push(None);
                original
            }
            NativeLiteralAllocation::PrivateLogicalBoolean85(_) => {
                if self.protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5) {
                    return Err(NativeLiteralUnavailable("native C85 logical fold origin"));
                }
                let key = GlobalKey {
                    protocol: self.protocol,
                    namespace: None,
                    original: literal.bytes().to_vec(),
                };
                let (registration, original) =
                    input.world.borrow_mut().register(key, self.protocol)?;
                input.world.borrow_mut().release(registration);
                self.registrations.push(None);
                original
            }
            NativeLiteralAllocation::PrivateExpressionNumber { version, value } => {
                if *version < tcl_dialect::TclVersion::V8_5
                    || self.protocol.tcl_version() != Some(*version)
                {
                    return Err(NativeLiteralUnavailable(
                        "native folded expression literal origin",
                    ));
                }
                self.registrations.push(None);
                Value::from_native_scalar_cache(
                    tcl_syntax::scalar_getter::NativeScalarCache::Number(value.number()),
                    None,
                    tcl_registry::InvocationDialect::for_version(*version),
                )
                .map_err(|_| NativeLiteralUnavailable("native folded expression header"))?
            }
            _ => unreachable!("non-expression private literal"),
        })
    }

    fn register(
        &mut self,
        world: &Rc<RefCell<NativeLiteralWorld>>,
        bytes: &[u8],
        namespace: Option<NsId>,
        protocol: NativeStringProtocol,
    ) -> Result<Value, NativeLiteralUnavailable> {
        // Jim's retained procedure tokens are not C interpreter-global literals.
        if protocol.is_jim084() {
            self.registrations.push(None);
            return Ok(Value::new_native_string_bytes(bytes));
        }
        let key = GlobalKey {
            protocol,
            namespace,
            original: bytes.to_vec(),
        };
        let (index, value) = world.borrow_mut().register(key, protocol)?;
        self.registrations.push(Some(index));
        Ok(value)
    }

    /// Finalize an original source's object array before installing its Bytecode primary.
    /// A shared analytical artifact keeps its own leases; this new actual array
    /// acquires distinct local leases and substitutes only identical source objects.
    fn finalize_original_source(
        self: &Rc<Self>,
        source: &Value,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<Self>, tcl_syntax::value::ValueError> {
        use tcl_runtime_api::native_literal::{NativeSourceLiteralAction, source_literal_action};
        if protocol != self.protocol {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "foreign literal allocation protocol",
            ));
        }
        if !self.values.iter().any(|slot| {
            slot.borrow()
                .as_ref()
                .is_some_and(|value| value.is_same_object(source))
        }) {
            return Ok(self.clone());
        }
        if source_literal_action(protocol, true) == NativeSourceLiteralAction::RetainSourceCycle {
            let world = self.world.upgrade().ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "retired C84 source literal registration world",
                ),
            )?;
            let world = world.borrow();
            let this_pool = Rc::downgrade(self);
            let cleanup_retains_pool = world
                .pools
                .iter()
                .any(|pool| Weak::ptr_eq(pool, &this_pool));
            let source_slots_registered = self.source_slots_registered(&world, source);
            if cleanup_retains_pool && source_slots_registered {
                return Ok(self.clone());
            }
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "C84 source literal lacks its actual registered cleanup owner",
            ));
        }
        if source_literal_action(protocol, true) != NativeSourceLiteralAction::CopySourceString {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native source/literal self-reference ownership",
            ));
        }
        let bytes = source
            .native_string_bytes(protocol)
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?;
        let world = self.world.upgrade().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired literal registration world",
            ),
        )?;
        let mut prepared = Self {
            values: Vec::with_capacity(self.values.len()),
            protocol,
            registrations: Vec::with_capacity(self.registrations.len()),
            world: self.world.clone(),
        };
        for (slot, registration) in self.values.iter().zip(&self.registrations) {
            let borrowed = slot.borrow();
            let value = borrowed.as_ref().ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "retired native literal slot",
                ),
            )?;
            if value.is_same_object(source) {
                prepared
                    .values
                    .push(RefCell::new(Some(Value::new_native_string_bytes(
                        bytes.clone(),
                    ))));
                prepared.registrations.push(None);
            } else if let Some(index) = registration {
                let original = world
                    .borrow_mut()
                    .shared
                    .borrow_mut()
                    .acquire_registration(*index)
                    .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "retired literal registration",
                    ))?;
                prepared.values.push(RefCell::new(Some(original)));
                prepared.registrations.push(Some(*index));
            } else {
                prepared.values.push(RefCell::new(Some(value.clone())));
                prepared.registrations.push(None);
            }
        }
        let prepared = Rc::new(prepared);
        world.borrow_mut().pools.push(Rc::downgrade(&prepared));
        Ok(prepared)
    }

    fn source_slots_registered(&self, world: &NativeLiteralWorld, source: &Value) -> bool {
        let registrations = world.shared.borrow();
        self.values
            .iter()
            .zip(&self.registrations)
            .all(|(slot, index)| {
                let slot = slot.borrow();
                let Some(value) = slot.as_ref() else {
                    return false;
                };
                !value.is_same_object(source)
                    || index.is_some_and(|index| {
                        registrations
                            .registration_value(index)
                            .is_some_and(|registered| registered.is_same_object(value))
                    })
            })
    }

    pub(crate) fn value(&self, index: usize) -> Option<Value> {
        self.values.get(index)?.borrow().clone()
    }

    #[cfg(test)]
    pub(crate) fn with_original<R>(
        &self,
        index: usize,
        observe: impl FnOnce(&Value) -> R,
    ) -> Option<R> {
        let original = self.values.get(index)?.borrow();
        Some(observe(original.as_ref()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::number::Number;
    use tcl_syntax::scalar_getter::{NativeScalarCache, NativeScalarGetterKind};

    fn pool(
        world: &Rc<RefCell<NativeLiteralWorld>>,
        table: &LiteralTable,
        version: tcl_dialect::TclVersion,
    ) -> Rc<NativeLiteralPool> {
        NativeLiteralPool::create(
            world,
            table,
            Some(NativeStringProtocol::C(version)),
            ROOT_NS,
            &ByteNamespacePath::root(),
        )
        .unwrap()
    }

    #[test]
    fn c91_constant_concat_retains_private_allocated_string_headers() {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        use tcl_syntax::native_string::NativeStringStorageIdentity as Storage;
        let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
        let mut table = LiteralTable::new();
        let registered = table.intern_bytes(b"");
        let empty = table.register_unshared(b"");
        let whitespace = table.register_private_concat_string(b"");
        let text = table.register_private_concat_string(b"A B");
        let owner = pool(&world, &table, tcl_dialect::TclVersion::V9_1);
        for index in [whitespace, text] {
            owner.with_original(index, |original| {
                assert!(matches!(
                    original.native_object_snapshot().cache,
                    Cache::String {
                        num_chars: None,
                        unicode: None,
                        ..
                    }
                ));
                assert_eq!(
                    original.resident_string_storage_identity(),
                    Some(Storage::Allocated)
                );
                assert_eq!(owner.reference_owners(original), (1, 0));
            });
        }
        owner.with_original(empty, |original| {
            assert!(matches!(
                original.native_object_snapshot().cache,
                Cache::None
            ));
            assert_eq!(
                original.resident_string_storage_identity(),
                Some(Storage::CanonicalEmpty)
            );
            assert_eq!(owner.reference_owners(original), (1, 0));
        });
        assert!(
            !owner
                .value(registered)
                .unwrap()
                .is_same_object(&owner.value(empty).unwrap())
        );
        assert!(
            !owner
                .value(empty)
                .unwrap()
                .is_same_object(&owner.value(whitespace).unwrap())
        );
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version != tcl_dialect::TclVersion::V9_1)
        {
            assert!(
                NativeLiteralPool::create(
                    &world,
                    &table,
                    Some(NativeStringProtocol::C(version)),
                    ROOT_NS,
                    &ByteNamespacePath::root()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn c85_folded_logical_original_shares_only_a_live_registration() {
        for registered in [false, true] {
            let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
            let mut table = LiteralTable::new();
            if registered {
                table.intern_bytes(b"1");
            }
            let first = table.register_private_logical_boolean85(true);
            let second = table.register_private_logical_boolean85(true);
            let pool = pool(&world, &table, tcl_dialect::TclVersion::V8_5);
            pool.with_original(first, |original| {
                assert_eq!(
                    pool.reference_owners(original),
                    if registered { (3, 1) } else { (1, 0) }
                );
                assert_eq!(original.native_scalar_cache(), None);
                pool.with_original(second, |other| {
                    assert_eq!(original.is_same_object(other), registered);
                })
                .unwrap();
            })
            .unwrap();
        }
    }

    #[test]
    fn local_array_lease_preserves_global_cache_and_retires_registration_before_guest_object() {
        let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
        let mut table = LiteralTable::new();
        table.intern_bytes(b"17");
        let first = pool(&world, &table, tcl_dialect::TclVersion::V9_0);
        let value = first.value(0).unwrap();
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        value
            .native_scalar_probe(dialect, NativeScalarGetterKind::Wide)
            .unwrap()
            .unwrap();
        let second = pool(&world, &table, tcl_dialect::TclVersion::V9_0);
        let same = second.value(0).unwrap();
        assert_eq!(
            same.native_object_identity(),
            value.native_object_identity()
        );
        assert_eq!(
            same.native_scalar_cache(),
            Some(NativeScalarCache::Number(Number::Int(17)))
        );
        let activation = Rc::clone(&first);
        assert_eq!(
            world
                .borrow()
                .shared
                .borrow()
                .registration_owners(0)
                .unwrap(),
            2
        );
        drop(first);
        drop(second);
        assert!(
            world
                .borrow()
                .shared
                .borrow()
                .registration_owners(0)
                .is_some()
        );
        drop(activation);
        assert!(
            world
                .borrow()
                .shared
                .borrow()
                .registration_owners(0)
                .is_none()
        );
        let third = pool(&world, &table, tcl_dialect::TclVersion::V9_0);
        assert_ne!(
            third.value(0).unwrap().native_object_identity(),
            value.native_object_identity()
        );
        assert_eq!(
            value.native_scalar_cache(),
            Some(NativeScalarCache::Number(Number::Int(17)))
        );
    }

    #[test]
    fn private_constant_list_retains_distinct_original_members_and_declines_older_compiler() {
        let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
        let mut table = LiteralTable::new();
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
        table.register_private_constant_list(&[b"A", b"A"], protocol);
        let owner = pool(&world, &table, tcl_dialect::TclVersion::V8_6);
        let first = owner.value(0).unwrap();
        let second = owner.value(0).unwrap();
        assert_eq!(
            first.native_object_identity(),
            second.native_object_identity()
        );
        let (members, _) = first.cached_list_representation().unwrap();
        assert_ne!(
            members[0].native_object_identity(),
            members[1].native_object_identity()
        );
        assert!(
            NativeLiteralPool::create(
                &world,
                &table,
                Some(NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5)),
                ROOT_NS,
                &ByteNamespacePath::root()
            )
            .is_err()
        );
    }

    #[test]
    fn command_registration_partitions_match_the_five_native_engines() {
        for version in [
            tcl_dialect::TclVersion::V8_4,
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            for fully_qualified in [false, true] {
                let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
                let bytes: &[u8] = if fully_qualified { b"::head" } else { b"head" };
                let mut command = LiteralTable::new();
                command.intern_command_bytes(bytes, &ByteNamespacePath::root(), fully_qualified);
                let first = pool(&world, &command, version);
                let other = NativeLiteralPool::create(
                    &world,
                    &command,
                    Some(NativeStringProtocol::C(version)),
                    NsId(9),
                    &ByteNamespacePath::root(),
                )
                .unwrap();
                let mut data = LiteralTable::new();
                data.intern_bytes(bytes);
                let data = pool(&world, &data, version);
                let original = first.value(0).unwrap().native_object_identity();
                assert_eq!(
                    other.value(0).unwrap().native_object_identity() == original,
                    fully_qualified || version == tcl_dialect::TclVersion::V8_4,
                );
                assert_eq!(
                    data.value(0).unwrap().native_object_identity() == original,
                    version == tcl_dialect::TclVersion::V8_4
                        || (version == tcl_dialect::TclVersion::V8_5 && fully_qualified),
                );
            }
        }
    }

    #[test]
    fn c84_registered_long_survives_int_then_changes_only_at_wide() {
        let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
        let mut table = LiteralTable::new();
        table.intern_bytes(b"17\0suffix");
        let owner = pool(&world, &table, tcl_dialect::TclVersion::V8_4);
        let original = owner.value(0).unwrap();
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
        assert_eq!(
            original.native_scalar_cache(),
            Some(NativeScalarCache::Tcl84Long(17))
        );
        original
            .native_scalar_probe(dialect, NativeScalarGetterKind::Int)
            .unwrap()
            .unwrap();
        assert_eq!(
            original.native_scalar_cache(),
            Some(NativeScalarCache::Tcl84Long(17))
        );
        original
            .native_scalar_probe(dialect, NativeScalarGetterKind::Wide)
            .unwrap()
            .unwrap();
        assert_eq!(
            original.native_scalar_cache(),
            Some(NativeScalarCache::Number(Number::Int(17)))
        );
        assert_eq!(
            owner
                .value(0)
                .unwrap()
                .resident_string_bytes()
                .unwrap()
                .as_ref(),
            b"17\0suffix"
        );
    }

    #[test]
    fn native_opaque_literal_bytes_and_c84_initial_cache_match_captured_producers() {
        let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
        for (version, expected) in [
            (tcl_dialect::TclVersion::V8_4, true),
            (tcl_dialect::TclVersion::V8_5, false),
        ] {
            let mut table = LiteralTable::new();
            table.intern_bytes(b"\xff\0tail");
            table.intern_bytes(b"17\0suffix");
            table.intern_bytes(b"-9223372036854775808");
            let owner = pool(&world, &table, version);
            assert_eq!(
                owner
                    .value(0)
                    .unwrap()
                    .resident_string_bytes()
                    .unwrap()
                    .as_ref(),
                b"\xff\0tail"
            );
            assert_eq!(
                owner.value(1).unwrap().native_scalar_cache().is_some(),
                expected
            );
            assert_eq!(owner.value(2).unwrap().native_scalar_cache(), None);
        }
    }
}

/// Closed actual DIRECT operand producer; the control plan owns no guest literal object.
#[derive(Clone, Copy)]
pub(crate) struct NativeDirectSourceOperands {
    protocol: NativeStringProtocol,
    execution_policy: Option<crate::compiled::NativeCompilerPolicy>,
}
impl NativeDirectSourceOperands {
    fn new(
        asm: &tcl_bytecode::FunctionAsm,
        protocol: NativeStringProtocol,
        selected: tcl_registry::native_eval_object::NativeEvalObjectProtocol,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        if !selected.permits_direct_source_operands(purpose, protocol)
            || !asm.plain_command_dispatch
            || asm.literals.entries().iter().any(|literal| {
                matches!(
                    literal.allocation(),
                    NativeLiteralAllocation::PrivateConstantList { .. }
                        | NativeLiteralAllocation::PrivateInteger(_)
                )
            })
            || asm
                .literals
                .native_actions()
                .iter()
                .any(|action| !matches!(action, NativeLiteralAction::Register(_)))
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native DIRECT source operand plan",
            ));
        }
        Ok(Self {
            protocol,
            execution_policy: None,
        })
    }

    pub(crate) fn is_current(self, policy: &crate::compiled::NativeCompilerPolicy) -> bool {
        self.execution_policy.as_ref() == Some(policy)
    }

    /// Each reached source word creates its own original string, with no pool cache donation.
    pub(crate) fn value(self, literal: &tcl_bytecode::NativeStringLiteral) -> Value {
        debug_assert_eq!(
            self.protocol,
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4)
        );
        Value::new_native_string_bytes(literal.bytes())
    }
}

impl crate::Vm {
    /// Retain a plain source control plan without creating native compiled objects or pools.
    pub(crate) fn direct_source_unit(
        &self,
        asm: Rc<tcl_bytecode::FunctionAsm>,
        namespace: ByteNamespacePath,
        strings: NativeStringProtocol,
        selected: tcl_registry::native_eval_object::NativeEvalObjectProtocol,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
    ) -> Result<crate::compiled::CompiledUnit, tcl_syntax::value::ValueError> {
        let mut operands = NativeDirectSourceOperands::new(&asm, strings, selected, purpose)?;
        operands.execution_policy = Some(self.native_compiler_policy());
        let mut unit = crate::compiled::CompiledUnit::new(
            asm,
            namespace,
            self.profile_generation(),
            self.trace_deopt_epoch(),
            None,
            self.native_interpreter_identity(),
            crate::compiled::CompilerProvenance::NativeDirect(self.compiler_generation()),
        );
        unit.compiled_local_layout = None;
        unit.direct_source_operands = Some(operands);
        Ok(unit)
    }
}

pub(crate) type NativeLocalNameTable = tcl_runtime_api::native_literal::NativeLocalNameTable<Value>;

pub(crate) fn create_local_names(
    world: &Rc<RefCell<NativeLiteralWorld>>,
    names: &[Option<tcl_core_types::NameBytes>],
    protocol: NativeStringProtocol,
) -> Result<Rc<NativeLocalNameTable>, NativeLiteralUnavailable> {
    let shared = Rc::clone(&world.borrow().shared);
    NativeLocalNameTable::create(
        &shared,
        names,
        protocol,
        |value| {
            value
                .native_string_bytes(protocol)
                .map(|bytes| bytes.to_vec())
                .map_err(|_| NativeLiteralUnavailable("native canonical local name string updater"))
        },
        |bytes| new_registered_string(bytes, protocol),
    )
}

impl crate::compiled::CompiledUnit {
    /// Prepare the authentic original source array before publishing a Bytecode cache.
    pub(crate) fn finalize_original_source_pool(
        mut self,
        source: &Value,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let pool = self.literal_pool.as_ref().map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native original source literal pool",
            )
        })?;
        self.literal_pool = Ok(pool.finalize_original_source(source, protocol)?);
        Ok(self)
    }
}

#[cfg(test)]
mod source_finalization_tests {
    use super::*;
    #[test]
    fn source_identity_replacement_drops_cache_instead_of_duplicating_its_primary() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
            let protocol = NativeStringProtocol::C(version);
            let source = Value::from_native_byte_array(
                Rc::from(&b"abc"[..]),
                tcl_registry::InvocationDialect::for_version(version),
            )
            .unwrap();
            let pool = Rc::new(NativeLiteralPool {
                values: vec![RefCell::new(Some(source.clone()))],
                protocol,
                registrations: vec![None],
                world: Rc::downgrade(&world),
            });
            let prepared = pool.finalize_original_source(&source, protocol).unwrap();
            let replacement = prepared.value(0).unwrap();
            assert!(!replacement.is_same_object(&source));
            assert!(replacement.byte_array_representation().is_none());
            assert_eq!(
                replacement.resident_string_bytes().unwrap().as_ref(),
                b"abc"
            );
            assert!(source.byte_array_representation().is_some());
            assert!(pool.value(0).unwrap().is_same_object(&source));
        }
    }
    #[test]
    fn c84_registered_self_literal_is_retained_until_the_actual_world_retires() {
        let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4);
        let mut table = LiteralTable::new();
        table.intern_bytes(b"x");
        let pool = NativeLiteralPool::create(
            &world,
            &table,
            Some(protocol),
            ROOT_NS,
            &ByteNamespacePath::root(),
        )
        .unwrap();
        let source = pool.value(0).unwrap();
        let weak_source = source.downgrade_native_object();
        let prepared = pool.finalize_original_source(&source, protocol).unwrap();
        assert!(Rc::ptr_eq(&pool, &prepared));
        assert_eq!(pool.reference_owners(&source), (1, 1));
        drop(source);
        assert!(weak_source.upgrade().is_some());
        drop(world);
        // TclDeleteLiteralTable withdraws actual local and global registrations;
        // retaining the immutable control plan does not retain those objects.
        assert!(pool.value(0).is_none());
        assert!(weak_source.upgrade().is_none());
    }

    #[test]
    fn direct_source_operands_are_fresh_and_never_receive_registered_integer_caches() {
        use tcl_registry::native_eval_object::EvalObjectPurpose;
        let selected = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4)
            .native_eval_object_protocol()
            .unwrap();
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4);
        let mut asm = tcl_bytecode::FunctionAsm {
            plain_command_dispatch: true,
            ..tcl_bytecode::FunctionAsm::default()
        };
        asm.literals.intern_bytes(b"17\0suffix\xff");
        let receipt =
            NativeDirectSourceOperands::new(&asm, protocol, selected, EvalObjectPurpose::Eval)
                .unwrap();
        let literal = &asm.literals.entries()[0];
        let first = receipt.value(literal);
        let second = receipt.value(literal);
        assert!(!first.is_same_object(&second));
        assert_eq!(first.native_scalar_cache(), None);
        assert_eq!(
            first.resident_string_bytes().unwrap().as_ref(),
            literal.bytes()
        );
        assert!(
            NativeDirectSourceOperands::new(
                &asm,
                protocol,
                selected,
                EvalObjectPurpose::ControlBody
            )
            .is_err()
        );
        assert!(
            NativeDirectSourceOperands::new(
                &asm,
                NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
                selected,
                EvalObjectPurpose::Eval,
            )
            .is_err()
        );
        asm.literals
            .register_private_constant_list(&[b"x"], protocol);
        assert!(
            NativeDirectSourceOperands::new(&asm, protocol, selected, EvalObjectPurpose::Eval)
                .is_err()
        );
    }

    #[test]
    fn c84_source_registration_and_cleanup_are_independent_of_array_size() {
        for (width, source_index) in [(2, 0), (2, 1), (3, 0), (3, 1), (3, 2), (8, 4)] {
            let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
            let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4);
            let mut table = LiteralTable::new();
            for index in 0..width {
                if index == source_index {
                    table.intern_bytes(b"seed");
                } else {
                    table.intern_bytes(format!("extra-{index}").as_bytes());
                }
            }
            let pool = NativeLiteralPool::create(
                &world,
                &table,
                Some(protocol),
                ROOT_NS,
                &ByteNamespacePath::root(),
            )
            .unwrap();
            let source = pool.value(source_index).unwrap();
            let original = source.downgrade_native_object();
            let prepared = pool.finalize_original_source(&source, protocol).unwrap();
            assert!(Rc::ptr_eq(&pool, &prepared));
            assert_eq!(pool.reference_owners(&source), (1, 1));
            drop(source);
            drop(world);
            assert!(original.upgrade().is_none());
            for index in 0..width {
                assert!(pool.value(index).is_none());
            }
        }
    }

    #[test]
    fn foreign_protocol_refuses_even_without_a_matching_source() {
        let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
        let mut table = LiteralTable::new();
        table.intern_bytes(b"x");
        let pool = NativeLiteralPool::create(
            &world,
            &table,
            Some(NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6)),
            ROOT_NS,
            &ByteNamespacePath::root(),
        )
        .unwrap();
        assert!(
            pool.finalize_original_source(
                &Value::new_native_string_bytes(b"y".as_slice()),
                NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
            )
            .is_err()
        );
    }

    #[test]
    fn older_matching_source_declines_but_equal_bytes_with_distinct_identity_are_preserved() {
        let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
        let source = Value::new_native_string_bytes(b"x".as_slice());
        let other = Value::new_native_string_bytes(b"x".as_slice());
        for version in [tcl_dialect::TclVersion::V8_4, tcl_dialect::TclVersion::V8_5] {
            let protocol = NativeStringProtocol::C(version);
            let pool = Rc::new(NativeLiteralPool {
                values: vec![RefCell::new(Some(source.clone()))],
                protocol,
                registrations: vec![None],
                world: Rc::downgrade(&world),
            });
            assert!(
                pool.finalize_original_source(&source, NativeStringProtocol::C(version))
                    .is_err()
            );
            let unchanged = pool
                .finalize_original_source(&other, NativeStringProtocol::C(version))
                .unwrap();
            assert!(Rc::ptr_eq(&unchanged, &pool));
        }
    }
}
