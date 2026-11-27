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

//! Recovering a host command's original results into VM values: leaves, residents, list and dictionary children and the name and string caches.

use super::value_conversion::{
    ActualScalarImport, export_version, import_storage, import_version, import_vm_scalar,
    to_vm_value_using,
};
use super::{
    VmCommandNameCacheReceipt, VmListBackingReceipt, VmNamespaceNameCacheReceipt, VmOriginalObject,
};
use std::rc::Rc;
use tcl_engine_api::{EngineError, OriginalObject, Value};
use tcl_vm::Completion;

pub(super) fn recover_original_result(
    result: &tcl_engine_api::OriginalObjectResult,
    dialect: tcl_native_model::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<tcl_vm::Value, EngineError> {
    recover_original_result_in(
        result,
        dialect,
        interpreter,
        &mut std::collections::BTreeMap::new(),
        None,
    )
}

pub(super) fn recover_original_completion(
    completion: &tcl_engine_api::OriginalObjectCompletion,
    dialect: tcl_native_model::InvocationDialect,
    interpreter: (u64, u64),
    actual: &ActualScalarImport<'_>,
) -> Result<Completion<tcl_vm::Value>, EngineError> {
    let mut memo = std::collections::BTreeMap::new();
    let result = recover_original_result_in(
        &completion.result,
        dialect,
        interpreter,
        &mut memo,
        Some(actual),
    )?;
    let options = recover_original_result_in(
        &completion.options,
        dialect,
        interpreter,
        &mut memo,
        Some(actual),
    )?;
    Ok(Completion::new(completion.code, result, options))
}

enum OriginalRecoveryWork<'a> {
    Visit(&'a tcl_engine_api::OriginalObjectResult),
    List(usize, bool),
    Dictionary(usize, Option<usize>),
    Resident(&'a Rc<[u8]>, tcl_engine_api::NativeStringStorageIdentity),
    Shared(usize),
}

fn recover_original_leaf(
    result: &tcl_engine_api::OriginalObjectResult,
    dialect: tcl_native_model::InvocationDialect,
    interpreter: (u64, u64),
    actual: Option<&ActualScalarImport<'_>>,
) -> Result<Option<tcl_vm::Value>, EngineError> {
    use tcl_engine_api::OriginalObjectResult as R;
    Ok(Some(match result {
        R::Value(value) => to_vm_value_using(value, dialect, actual)?,
        R::Index { cache, origin } => tcl_vm::Value::from_native_index_cache(
            cache.clone(),
            import_version(*origin),
            dialect,
            None,
        )
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?,
        R::String(cache) => recover_native_string_cache(cache, dialect, None)?,
        R::CommandName(_) => {
            return Err(EngineError::ExecutionRefusal(
                "native command-name cache requires resident string storage".into(),
            ));
        }
        R::NamespaceName(cache) => {
            recover_native_namespace_name_cache(cache, dialect, interpreter, None)?
        }
        R::Original(original) => recover_original_handle(original, dialect, interpreter)?,
        R::RetainedList(carrier) => {
            let receipt = checked_list_backing(carrier, dialect, interpreter)?;
            tcl_vm::Value::from_retained_native_list_backing(&receipt.backing, receipt.protocol)
                .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?
        }
        R::Shared(_) | R::List { .. } | R::Dictionary { .. } | R::Resident { .. } => {
            return Ok(None);
        }
    }))
}

fn recover_original_resident(
    value: &tcl_engine_api::OriginalObjectResult,
    string: &Rc<[u8]>,
    storage: tcl_engine_api::NativeStringStorageIdentity,
    dialect: tcl_native_model::InvocationDialect,
    interpreter: (u64, u64),
    actual: Option<&ActualScalarImport<'_>>,
) -> Result<Option<tcl_vm::Value>, EngineError> {
    use tcl_engine_api::OriginalObjectResult as R;
    let value = match value {
        R::Value(Value::NativeScalar(cache)) => import_vm_scalar(
            cache,
            dialect,
            Some((Rc::clone(string), import_storage(storage))),
            actual,
        )?,
        R::CommandName(cache) => recover_native_command_name_cache(
            cache,
            dialect,
            interpreter,
            Rc::clone(string),
            import_storage(storage),
        )?,
        R::NamespaceName(cache) => recover_native_namespace_name_cache(
            cache,
            dialect,
            interpreter,
            Some((Rc::clone(string), import_storage(storage))),
        )?,
        R::String(cache) => recover_native_string_cache(
            cache,
            dialect,
            Some((Rc::clone(string), import_storage(storage))),
        )?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

pub(super) fn recover_original_result_in(
    result: &tcl_engine_api::OriginalObjectResult,
    dialect: tcl_native_model::InvocationDialect,
    interpreter: (u64, u64),
    memo: &mut std::collections::BTreeMap<usize, tcl_vm::Value>,
    actual: Option<&ActualScalarImport<'_>>,
) -> Result<tcl_vm::Value, EngineError> {
    use OriginalRecoveryWork as Work;
    use tcl_engine_api::OriginalObjectResult as R;
    if let Some(actual) = actual {
        actual.check()?;
    }
    let reached = (|| {
        let mut work = vec![Work::Visit(result)];
        let mut values = Vec::new();
        let mut active = std::collections::BTreeSet::new();
        while let Some(next) = work.pop() {
            if let Some(actual) = actual {
                actual.check()?;
            }
            if let Work::Visit(result) = &next
                && let Some(value) = recover_original_leaf(result, dialect, interpreter, actual)?
            {
                if let Some(actual) = actual {
                    actual.check()?;
                }
                values.push(value);
                continue;
            }
            match next {
                Work::Visit(R::Shared(node)) => {
                    let identity = Rc::as_ptr(node) as usize;
                    if let Some(value) = memo.get(&identity) {
                        values.push(value.clone());
                    } else {
                        if !active.insert(identity) {
                            return Err(EngineError::ExecutionRefusal(
                                "cyclic callback object graph".into(),
                            ));
                        }
                        work.push(Work::Shared(identity));
                        work.push(Work::Visit(node));
                    }
                }
                Work::Visit(R::List { items, canonical }) => {
                    work.push(Work::List(items.len(), *canonical));
                    work.extend(items.iter().rev().map(Work::Visit));
                }
                Work::Visit(R::Dictionary { entries, buckets }) => {
                    work.push(Work::Dictionary(entries.len(), *buckets));
                    for (key, value) in entries.iter().rev() {
                        work.push(Work::Visit(value));
                        work.push(Work::Visit(key));
                    }
                }
                Work::Visit(R::Resident {
                    value,
                    string,
                    storage,
                }) => {
                    if let Some(recovered) = recover_original_resident(
                        value,
                        string,
                        *storage,
                        dialect,
                        interpreter,
                        actual,
                    )? {
                        values.push(recovered);
                    } else {
                        work.push(Work::Resident(string, *storage));
                        work.push(Work::Visit(value));
                    }
                }
                Work::Visit(_) => unreachable!("original leaf already recovered"),
                Work::List(count, canonical) => {
                    recover_original_list_children(&mut values, count, canonical, dialect)?;
                }
                Work::Dictionary(count, buckets) => {
                    let protocol = dialect.native_string_protocol().ok_or_else(|| {
                        EngineError::ExecutionRefusal(
                            "callback Dictionary updater issuer unavailable".into(),
                        )
                    })?;
                    let value = recover_dictionary_children(&mut values, count, buckets, protocol)?;
                    values.push(value);
                }
                Work::Resident(string, storage) => {
                    recover_original_resident_child(&mut values, string, storage)?;
                }
                Work::Shared(identity) => {
                    memo.insert(
                        identity,
                        values.last().expect("visited callback graph node").clone(),
                    );
                    active.remove(&identity);
                }
            }
        }
        Ok(values.pop().expect("visited callback result"))
    })();
    if let Some(actual) = actual {
        actual.check()?;
    }
    reached
}

fn recover_original_list_children(
    values: &mut Vec<tcl_vm::Value>,
    count: usize,
    canonical: bool,
    dialect: tcl_native_model::InvocationDialect,
) -> Result<(), EngineError> {
    let start = values
        .len()
        .checked_sub(count)
        .expect("visited callback List children");
    let items = values.split_off(start);
    let protocol = dialect.native_string_protocol().ok_or_else(|| {
        EngineError::ExecutionRefusal("callback List updater issuer unavailable".into())
    })?;
    values.push(tcl_vm::Value::list_with_native_canonical_in(
        items, canonical, protocol,
    ));
    Ok(())
}

fn recover_original_resident_child(
    values: &mut Vec<tcl_vm::Value>,
    string: &Rc<[u8]>,
    storage: tcl_engine_api::NativeStringStorageIdentity,
) -> Result<(), EngineError> {
    let value = values
        .pop()
        .expect("visited resident callback representation");
    values.push(
        value
            .with_resident_string_bytes_and_storage(Rc::clone(string), import_storage(storage))
            .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?,
    );
    Ok(())
}

pub(super) fn checked_list_backing_receipt(
    carrier: &tcl_engine_api::NativeListBacking,
    dialect: tcl_native_model::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<&VmListBackingReceipt, EngineError> {
    let receipt = carrier
        .engine_receipt()
        .downcast_ref::<VmListBackingReceipt>()
        .ok_or_else(|| {
            EngineError::ExecutionRefusal("foreign native List backing authority".into())
        })?;
    let protocol = dialect
        .native_string_protocol()
        .ok_or_else(|| EngineError::ExecutionRefusal("native List issuer unavailable".into()))?;
    if receipt.interpreter != interpreter
        || carrier.scope_identity() != interpreter
        || receipt.protocol != protocol
        || protocol.tcl_version().map(export_version) != Some(carrier.origin())
        || carrier.member_count() != receipt.backing.len()
        || !Rc::ptr_eq(
            &carrier.canonical_state(),
            &receipt.backing.canonical_state(),
        )
        || !receipt.backing.is_whole_backing()
    {
        return Err(EngineError::ExecutionRefusal(
            "native List backing scope, issuer or shape mismatch".into(),
        ));
    }
    Ok(receipt)
}

pub(super) fn checked_list_backing(
    carrier: &tcl_engine_api::NativeListBacking,
    dialect: tcl_native_model::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<&VmListBackingReceipt, EngineError> {
    let receipt = checked_list_backing_receipt(carrier, dialect, interpreter)?;
    if !receipt.backing.has_native_header() {
        return Err(EngineError::ExecutionRefusal(
            "retired native List backing".into(),
        ));
    }
    Ok(receipt)
}

fn recover_original_handle(
    original: &Rc<dyn tcl_engine_api::OriginalObject>,
    dialect: tcl_native_model::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<tcl_vm::Value, EngineError> {
    let Some(receipt) = original.engine_receipt().downcast_ref::<VmOriginalObject>() else {
        return Err(EngineError::ExecutionRefusal(
            "foreign original-object engine authority".into(),
        ));
    };
    if receipt.interpreter != interpreter
        || receipt.native_c_version()
            != dialect
                .native_scalar_getter_protocol()
                .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
                .map(export_version)
    {
        return Err(EngineError::ExecutionRefusal(
            "foreign original-object interpreter or native issuer".into(),
        ));
    }
    Ok((*receipt.value).clone())
}

pub(super) fn recover_dictionary_children(
    values: &mut Vec<tcl_vm::Value>,
    count: usize,
    buckets: Option<usize>,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<tcl_vm::Value, EngineError> {
    let start = values
        .len()
        .checked_sub(count * 2)
        .expect("visited callback Dictionary children");
    let children = values.split_off(start);
    let mut children = children.into_iter();
    let entries = (0..count)
        .map(|_| {
            (
                children.next().expect("key"),
                children.next().expect("value"),
            )
        })
        .collect();
    tcl_vm::Value::native_dictionary_constructor(entries, buckets, protocol)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}

fn recover_native_namespace_name_cache(
    carrier: &tcl_engine_api::NativeNamespaceNameCache,
    dialect: tcl_native_model::InvocationDialect,
    interpreter: (u64, u64),
    resident: Option<(
        Rc<[u8]>,
        tcl_syntax::native_string::NativeStringStorageIdentity,
    )>,
) -> Result<tcl_vm::Value, EngineError> {
    let receipt = carrier
        .engine_receipt()
        .downcast_ref::<VmNamespaceNameCacheReceipt>()
        .ok_or_else(|| {
            EngineError::ExecutionRefusal("foreign namespace-name cache authority".into())
        })?;
    let cache = &receipt.cache;
    if carrier.scope_identity() != interpreter
        || receipt.interpreter != interpreter
        || (cache.interpreter().owner, cache.interpreter().interpreter) != interpreter
        || carrier.origin() != export_version(cache.version())
        || dialect
            .native_namespace_name_protocol()
            .is_none_or(|protocol| protocol.recipe().version() != cache.version())
    {
        return Err(EngineError::ExecutionRefusal(
            "foreign namespace-name cache interpreter or origin".into(),
        ));
    }
    tcl_vm::Value::from_native_namespace_name_cache(cache.clone(), dialect, resident)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}

fn recover_native_command_name_cache(
    carrier: &tcl_engine_api::NativeCommandNameCache,
    dialect: tcl_native_model::InvocationDialect,
    interpreter: (u64, u64),
    resident: Rc<[u8]>,
    storage: tcl_syntax::native_string::NativeStringStorageIdentity,
) -> Result<tcl_vm::Value, EngineError> {
    let receipt = carrier
        .engine_receipt()
        .downcast_ref::<VmCommandNameCacheReceipt>()
        .ok_or_else(|| {
            EngineError::ExecutionRefusal("foreign command-name cache authority".into())
        })?;
    if carrier.scope_identity() != interpreter
        || receipt.interpreter != interpreter
        || carrier.origin() != export_version(receipt.version)
        || !dialect
            .native_command_name_protocol()
            .is_some_and(|protocol| protocol.accepts_cache_origin(receipt.version))
        || receipt.cache.as_ref().is_some_and(|cache| {
            (cache.interpreter.owner, cache.interpreter.interpreter) != interpreter
                || cache.version != receipt.version
        })
    {
        return Err(EngineError::ExecutionRefusal(
            "foreign command-name cache interpreter or origin".into(),
        ));
    }
    let value = tcl_vm::Value::new_native_string_bytes(Rc::clone(&resident))
        .with_resident_string_bytes_and_storage(resident, storage)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?;
    if let Some(cache) = &receipt.cache {
        value.install_native_command_name_cache(cache.clone(), dialect)
    } else {
        value.install_unresolved_native_command_name_cache(dialect)
    }
    .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?;
    Ok(value)
}

fn recover_native_string_cache(
    cache: &tcl_engine_api::NativeStringCache,
    dialect: tcl_native_model::InvocationDialect,
    resident: Option<(
        Rc<[u8]>,
        tcl_syntax::native_string::NativeStringStorageIdentity,
    )>,
) -> Result<tcl_vm::Value, EngineError> {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as S;
    let cache = match cache {
        tcl_engine_api::NativeStringCache::C {
            origin,
            num_chars,
            unicode,
        } => S::String {
            protocol: tcl_syntax::native_string::NativeStringProtocol::C(import_version(*origin)),
            num_chars: *num_chars,
            unicode: unicode.clone(),
        },
        tcl_engine_api::NativeStringCache::Jim { num_chars } => S::JimString {
            num_chars: *num_chars,
        },
    };
    tcl_vm::Value::from_native_string_cache(cache, dialect, resident)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}
