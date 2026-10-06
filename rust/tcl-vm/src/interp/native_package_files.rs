// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual package-record objects and source-inventory scopes.

use super::Vm;
use crate::Value;
use tcl_core_types::NameBytes;
use tcl_syntax::value::ValueError;

impl Vm {
    pub(crate) fn record_package_entry(&mut self, name: &[u8]) {
        let recipe = self
            .native_invocation_dialect()
            .native_package_protocol()
            .and_then(|protocol| {
                tcl_runtime_api::native_hash_abi::supported_backend_hash_abi(Some(0))
                    .and_then(|abi| protocol.hash_recipe(abi))
            });
        self.package_state.package_entry_order.select_recipe(recipe);
        self.package_state.package_entry_order.insert(name);
    }

    pub(crate) fn package_file_inventory_active(&self) -> bool {
        self.package_state.package_file_inventory_active
    }

    #[cfg(test)]
    pub(crate) fn with_package_file_object<R>(
        &self,
        name: &[u8],
        callback: impl FnOnce(&Value) -> R,
    ) -> R {
        callback(
            self.package_state
                .package_files
                .get(name)
                .expect("retained package file header"),
        )
    }

    pub(crate) fn package_file_object(&self, name: &[u8]) -> Value {
        self.package_state
            .package_files
            .get(name)
            .cloned()
            .unwrap_or_else(Value::empty)
    }

    pub(crate) fn begin_package_initialization(&mut self) -> bool {
        let Some(name) = self
            .native_invocation_dialect()
            .native_package_protocol()
            .and_then(|p| p.initialization_package())
        else {
            return false;
        };
        self.package_state.package_file_inventory_active = true;
        self.package_state
            .package_file_scopes
            .push(NameBytes::from(name));
        true
    }

    pub(crate) fn end_package_initialization(&mut self, entered: bool) {
        if entered {
            self.package_state.package_file_scopes.pop();
        }
    }

    pub(crate) fn take_package_file_scope(&mut self) -> Vec<NameBytes> {
        std::mem::take(&mut self.package_state.package_file_scopes)
    }

    pub(crate) fn restore_package_file_scope(&mut self, scope: Vec<NameBytes>) {
        self.package_state.package_file_scopes = scope;
    }

    pub(crate) fn enter_package_source_path(&mut self, path: &[u8]) {
        self.package_state
            .package_source_paths
            .push(tcl_core_types::c_string_extent(path).to_vec());
    }

    pub(crate) fn leave_package_source_path(&mut self) {
        self.package_state.package_source_paths.pop();
    }

    pub(crate) fn record_package_loader_origin(
        &mut self,
        name: &NameBytes,
        version: &NameBytes,
    ) -> Result<(), ValueError> {
        let origin = self
            .package_state
            .package_loader_origins
            .get(&(name.clone(), version.clone()))
            .cloned();
        if let Some(origin) = origin {
            self.record_package_source_file(&origin)?;
        }
        Ok(())
    }

    pub(crate) fn record_package_source_file(&mut self, filename: &[u8]) -> Result<(), ValueError> {
        if !self
            .native_invocation_dialect()
            .native_package_protocol()
            .is_some_and(|p| p.tracks_files())
        {
            return Ok(());
        }
        let Some(name) = self.package_state.package_file_scopes.last().cloned() else {
            return Ok(());
        };
        let strings = self
            .native_invocation_dialect()
            .native_string_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "package file string producer",
            ))?;
        let header = self
            .package_state
            .package_files
            .entry(name)
            .or_insert_with(|| Value::native_list_constructor(Vec::new(), strings));
        // Native Tcl appends directly to its assocdata-owned original header.
        // A guest-retained result makes that operation fatal, rather than COW.
        if header.native_object_is_shared() {
            return Err(ValueError::NativeFatalCondition(
                tcl_syntax::raw_string::NativeFatalCondition::SharedPackageFileListMutation,
            ));
        }
        let file = Value::from_native_string_bytes(tcl_core_types::c_string_extent(filename));
        *header = Value::native_list_append_elements(Some(header), &[file], strings)?;
        Ok(())
    }

    pub(crate) fn settle_package_source_completion(
        &mut self,
        mut completion: tcl_core_types::Completion<Value>,
    ) -> tcl_core_types::Completion<Value> {
        use tcl_registry::completion::CompletionCode;
        use tcl_registry::completion_route::{
            InvocationCompletionRoute as Route, ReturnCompletionRoute,
        };
        if completion.code != tcl_core_types::Code::Return {
            return completion;
        }
        let jim = self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        let (code, level) = if jim {
            (
                self.jim_errors.pending_return.code,
                self.jim_errors.pending_return.level,
            )
        } else {
            (
                self.native_c_return_state.code,
                self.native_c_return_state.level,
            )
        };
        let route = Route::Return(ReturnCompletionRoute {
            eventual_code: CompletionCode::from_int(code),
            remaining_level: level.max(0) as u64,
        });
        match tcl_registry::source_file::completion_route(self.native_invocation_dialect(), route) {
            Route::Return(pending) => {
                let level = pending.remaining_level as i64;
                if jim {
                    self.jim_errors.pending_return.level = level;
                } else {
                    self.settle_native_c_return_level(level);
                }
                completion.options = crate::command::with_return_level(&completion.options, level);
            }
            Route::Tcl(code) => {
                completion.code = tcl_core_types::Code::from_int(code.as_int() as i32);
                if jim {
                    self.jim_errors.pending_return.code = 0;
                    self.jim_errors.pending_return.level = 0;
                } else {
                    self.settle_native_c_return_level(0);
                }
            }
            _ => {
                return self
                    .refuse_host_command("source completion protocol is unavailable".into());
            }
        }
        completion
    }
}
