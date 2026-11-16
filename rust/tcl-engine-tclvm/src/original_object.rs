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

//! The original-object door a running host command gets: identity, caches, list backing and members.

use super::original_recovery::{checked_list_backing_receipt, recover_original_result};
use super::value_conversion::{export_storage, export_version, from_vm_object_value};
use super::{
    VmCommandNameCacheReceipt, VmListBackingReceipt, VmNamespaceNameCacheReceipt, VmOriginalObject,
    VmOriginalValue,
};
use std::rc::Rc;
use tcl_engine_api::{EngineError, Value};

impl tcl_engine_api::OriginalObject for VmOriginalObject {
    fn native_c_version(&self) -> Option<tcl_engine_api::NativeCVersion> {
        self.dialect
            .native_scalar_getter_protocol()
            .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
            .map(export_version)
    }
    fn scope_identity(&self) -> (u64, u64) {
        self.interpreter
    }
    fn resident_string(&self) -> Option<(Rc<[u8]>, tcl_engine_api::NativeStringStorageIdentity)> {
        self.value
            .resident_string_bytes()
            .map(|bytes| (bytes, export_storage(&self.value)))
    }
    fn identity(&self) -> usize {
        self.value.native_object_identity()
    }
    fn is_shared(&self) -> bool {
        self.value.native_object_is_shared()
    }
    fn snapshot(&self) -> Result<Value, EngineError> {
        if !self.value.native_object_is_live() {
            return Err(EngineError::ExecutionRefusal(
                "retired original native object header".into(),
            ));
        }
        from_vm_object_value(&self.value)
    }
    fn engine_receipt(&self) -> &dyn std::any::Any {
        self
    }
    fn index_cache(
        &self,
    ) -> Option<(
        tcl_engine_api::NativeIndexCache,
        tcl_engine_api::NativeCVersion,
    )> {
        self.value
            .native_index_cache()
            .map(|(cache, version)| (cache, export_version(version)))
    }
    fn string_cache(&self) -> Option<tcl_engine_api::NativeStringCache> {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as S;
        match self.value.native_object_snapshot().cache {
            S::String {
                protocol,
                num_chars,
                unicode,
            } => protocol
                .tcl_version()
                .map(|version| tcl_engine_api::NativeStringCache::C {
                    origin: export_version(version),
                    num_chars,
                    unicode,
                }),
            S::JimString { num_chars } => {
                Some(tcl_engine_api::NativeStringCache::Jim { num_chars })
            }
            _ => None,
        }
    }
    fn command_name_cache_origin(&self) -> Option<tcl_engine_api::NativeCVersion> {
        self.value
            .native_command_name_cache_origin()
            .map(export_version)
    }
    fn command_name_cache(&self) -> Option<tcl_engine_api::NativeCommandNameCache> {
        let version = self.value.native_command_name_cache_origin()?;
        Some(tcl_engine_api::NativeCommandNameCache::new(
            export_version(version),
            self.interpreter,
            Rc::new(VmCommandNameCacheReceipt {
                cache: self.value.native_command_name_cache(),
                version,
                interpreter: self.interpreter,
            }),
        ))
    }
    fn namespace_name_cache(&self) -> Option<tcl_engine_api::NativeNamespaceNameCache> {
        let cache = self.value.native_namespace_name_cache()?;
        Some(tcl_engine_api::NativeNamespaceNameCache::new(
            export_version(cache.version()),
            self.interpreter,
            Rc::new(VmNamespaceNameCacheReceipt {
                cache,
                interpreter: self.interpreter,
            }),
        ))
    }
    fn list_backing(&self) -> Result<Option<tcl_engine_api::NativeListBacking>, EngineError> {
        let Some(protocol) = self.dialect.native_string_protocol() else {
            return Err(EngineError::ExecutionRefusal(
                "original List issuer unavailable".into(),
            ));
        };
        let Some(backing) = self
            .value
            .native_list_backing_in(protocol)
            .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?
        else {
            return Ok(None);
        };
        let version = protocol.tcl_version().ok_or_else(|| {
            EngineError::ExecutionRefusal("C List backing issuer unavailable".into())
        })?;
        let canonical = backing.canonical_state();
        let sharing = backing.lifetime_view();
        let count = backing.len();
        Ok(Some(tcl_engine_api::NativeListBacking::new(
            export_version(version),
            self.interpreter,
            count,
            canonical,
            Rc::new(move || sharing.native_is_shared()),
            Rc::new(VmListBackingReceipt {
                backing,
                protocol,
                interpreter: self.interpreter,
            }),
        )))
    }
    fn list_backing_matches(
        &self,
        carrier: &tcl_engine_api::NativeListBacking,
    ) -> Result<bool, EngineError> {
        let receipt = checked_list_backing_receipt(carrier, self.dialect, self.interpreter)?;
        Ok(receipt.backing.has_native_header()
            && self.value.native_list_attachment_matches(&receipt.backing))
    }

    fn duplicate_native_header(
        &self,
    ) -> Result<Rc<dyn tcl_engine_api::OriginalObject>, EngineError> {
        let protocol = self.dialect.native_string_protocol().ok_or_else(|| {
            EngineError::ExecutionRefusal("native duplicate issuer unavailable".into())
        })?;
        drop(
            self.value
                .native_list_backing_in(protocol)
                .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?,
        );
        let duplicate = self.value.duplicate_native_object_in(protocol);
        Ok(Rc::new(Self {
            value: VmOriginalValue::Header(duplicate.native_lifetime_lease()),
            dialect: self.dialect,
            interpreter: self.interpreter,
        }))
    }
    fn list_members(
        &self,
    ) -> Result<Option<(Vec<Rc<dyn tcl_engine_api::OriginalObject>>, bool)>, EngineError> {
        Ok(self
            .value
            .cached_list_representation()
            .map(|(items, canonical)| {
                let members = (0..items.len())
                    .map(|index| {
                        Rc::new(Self {
                            value: VmOriginalValue::ListMember {
                                backing: items.lifetime_view(),
                                index,
                            },
                            dialect: self.dialect,
                            interpreter: self.interpreter,
                        }) as Rc<dyn tcl_engine_api::OriginalObject>
                    })
                    .collect();
                (members, canonical)
            }))
    }
    fn dictionary_members(
        &self,
    ) -> Result<
        Option<(
            Vec<(
                Rc<dyn tcl_engine_api::OriginalObject>,
                Rc<dyn tcl_engine_api::OriginalObject>,
            )>,
            Option<usize>,
        )>,
        EngineError,
    > {
        Ok(self
            .value
            .with_cached_dictionary_representation(|entries, buckets| {
                let members = entries
                    .iter()
                    .map(|(key, value)| {
                        (
                            Rc::new(Self {
                                value: VmOriginalValue::Header(key.native_lifetime_lease()),
                                dialect: self.dialect,
                                interpreter: self.interpreter,
                            })
                                as Rc<dyn tcl_engine_api::OriginalObject>,
                            Rc::new(Self {
                                value: VmOriginalValue::Header(value.native_lifetime_lease()),
                                dialect: self.dialect,
                                interpreter: self.interpreter,
                            })
                                as Rc<dyn tcl_engine_api::OriginalObject>,
                        )
                    })
                    .collect();
                (members, Some(buckets))
            }))
    }
    fn apply(
        &self,
        value: &tcl_engine_api::OriginalObjectResult,
        string_mutation: tcl_engine_api::ResidentStringMutation,
    ) -> Result<(), EngineError> {
        let mut reached = value;
        loop {
            reached = match reached {
                tcl_engine_api::OriginalObjectResult::Resident { value, .. } => value,
                tcl_engine_api::OriginalObjectResult::Shared(value) => value,
                _ => break,
            };
        }
        if let tcl_engine_api::OriginalObjectResult::RetainedList(backing) = reached
            && !self.list_backing_matches(backing)?
        {
            return Err(EngineError::ExecutionRefusal(
                "retired original List attachment".into(),
            ));
        }
        let donor = recover_original_result(value, self.dialect, self.interpreter)?;
        self.value
            .adopt_native_object_representation_with_string_mutation(
                &donor,
                self.dialect,
                string_mutation,
            )
            .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
    }
}
