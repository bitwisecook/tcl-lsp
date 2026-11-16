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

//! The Jim allocator context an object is bound to: its object pool, result publication, numeric stages and source information.

use super::native_jim_pool::JimAllocationScope;
use super::{
    IntRep, NativeJimObjectContext, NativeJimSource, NativeJimSourceInfo, Value, native_jim_pool,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::scalar_getter::{NativeScalarGetterKind, NativeScalarGetterProtocol};
use tcl_syntax::value::ValueError;

impl NativeJimObjectContext {
    pub(crate) fn new(
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Rc<Self>, tcl_syntax::value::ValueError> {
        if dialect.native_string_protocol() != Some(NativeStringProtocol::Jim084) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original source context",
            ));
        }
        let pool = Rc::new(native_jim_pool::JimPool::new(true));
        let allocation = JimAllocationScope::enter(Some(Rc::clone(&pool)), Rc::new(|| true));
        let empty = Value::new_native_string_bytes(b"".as_slice());
        let context = Rc::new(Self {
            numeric_host: RefCell::new(None),
            live: std::cell::Cell::new(true),
            pool,
            objects: [
                Some(empty.clone()),
                Some(Value::int(1)),
                Some(Value::int(0)),
                Some(empty.clone()),
                None,
                Some(empty),
                Some(Value::new_native_string_bytes(b"unknown".as_slice())),
                Some(Value::new_native_string_bytes(b"jim::defer".as_slice())),
                Some(Value::new_native_string_bytes(b"".as_slice())),
                Some(Value::new_native_string_bytes(b"".as_slice())),
            ]
            .map(RefCell::new),
        });
        for object in &context.objects {
            let object = object.borrow();
            let Some(value) = object.as_ref() else {
                continue;
            };
            value.bind_native_jim_context(&context)?;
        }
        drop(allocation);
        Ok(context)
    }

    pub(crate) fn allocation_pool(&self) -> Rc<native_jim_pool::JimPool> {
        Rc::clone(&self.pool)
    }

    pub(crate) fn select_numeric_host(&self, host: Rc<dyn tcl_platform::Host>) {
        *self.numeric_host.borrow_mut() = Some(host);
    }

    /// Actual retained host target fields, independent of source grammar.
    pub(crate) fn scalar_getter_target(
        &self,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterTarget, ValueError> {
        self.scalar_getter_target_using(&mut None, &Cell::new(None))
    }

    pub(super) fn scalar_getter_target_using<E: From<ValueError>>(
        &self,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        abi: &Cell<Option<tcl_platform::NativeCIntegerAbi>>,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterTarget, E> {
        self.numeric_stage_using(
            check,
            abi,
            tcl_cmd_core::native_numeric::scalar_getter_target,
        )
    }

    pub(super) fn fresh_numeric_conversion(
        &self,
        protocol: NativeScalarGetterProtocol,
        kind: NativeScalarGetterKind,
        original: &[u8],
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterConversion, ValueError> {
        self.fresh_numeric_conversion_using(protocol, kind, original, &mut None, &Cell::new(None))
    }

    pub(super) fn fresh_numeric_conversion_using<E: From<ValueError>>(
        &self,
        protocol: NativeScalarGetterProtocol,
        kind: NativeScalarGetterKind,
        original: &[u8],
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        abi: &Cell<Option<tcl_platform::NativeCIntegerAbi>>,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterConversion, E> {
        self.numeric_stage_using(check, abi, |environment| {
            tcl_cmd_core::native_numeric::fresh_jim_conversion(
                protocol,
                kind,
                original,
                environment,
            )
        })
    }

    pub(super) fn numeric_stage_using<E: From<ValueError>, T>(
        &self,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        abi: &Cell<Option<tcl_platform::NativeCIntegerAbi>>,
        operation: impl FnOnce(&dyn tcl_platform::NumericEnvironment) -> Result<T, ValueError>,
    ) -> Result<T, E> {
        Value::native_stage_check(check)?;
        if !self.is_live() {
            return Err(ValueError::CommandProtocolUnavailable("retired Jim interpreter").into());
        }
        let host = self
            .numeric_host
            .borrow()
            .clone()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let actual = host.numeric_environment();
        Value::native_stage_check(check)?;
        let actual = actual.ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if check.is_none() {
            return operation(actual).map_err(Into::into);
        }
        // The context's own Host supplies the worker. The typed original stage
        // error is retained independently of the neutral forwarding result.
        let mut current = || -> Result<(), E> {
            Value::native_stage_check(check)?;
            if !self.is_live()
                || !self
                    .numeric_host
                    .borrow()
                    .as_ref()
                    .is_some_and(|current| Rc::ptr_eq(current, &host))
            {
                return Err(
                    ValueError::CommandProtocolUnavailable("original Jim numeric context").into(),
                );
            }
            Ok(())
        };
        let (reached, first) = {
            let stage = RefCell::new(&mut current);
            let first = RefCell::<Option<E>>::new(None);
            let neutral = || {
                if first.borrow().is_some() {
                    return Err(tcl_platform::NumericEnvironmentUnavailable::Target);
                }
                match stage.borrow_mut()() {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        *first.borrow_mut() = Some(error);
                        Err(tcl_platform::NumericEnvironmentUnavailable::Target)
                    }
                }
            };
            let checked = tcl_platform::CheckedNumericEnvironment::new(actual, &neutral, abi);
            let reached = operation(&checked);
            let first = first.borrow_mut().take();
            (reached, first)
        };
        if let Some(error) = first {
            return Err(error);
        }
        current()?;
        reached.map_err(Into::into)
    }

    pub(crate) fn is_live(&self) -> bool {
        self.live.get()
    }

    pub(crate) fn defer_object(&self) -> std::cell::Ref<'_, Value> {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Defer)
    }

    pub(super) fn object(
        &self,
        role: tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole,
    ) -> std::cell::Ref<'_, Value> {
        std::cell::Ref::map(self.objects[role as usize].borrow(), |object| {
            object.as_ref().expect("live Jim interpreter object role")
        })
    }

    pub(crate) fn empty_object(&self) -> std::cell::Ref<'_, Value> {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Empty)
    }

    pub(crate) fn null_script_object(&self) -> std::cell::Ref<'_, Value> {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::NullScript)
    }

    pub(crate) fn current_filename_object(&self) -> Value {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::CurrentFilename)
            .clone()
    }

    pub(crate) fn release_object(
        &self,
        role: tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole,
    ) {
        let retired = self.objects[role as usize].borrow_mut().take();
        drop(retired);
    }

    pub(crate) fn retire(&self) {
        self.live.set(false);
        self.pool.retire();
        let host = self.numeric_host.borrow_mut().take();
        drop(host);
    }

    pub(crate) fn with_result_object<R>(&self, observer: impl FnOnce(&Value) -> R) -> R {
        observer(&self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Result))
    }

    pub(crate) fn result_object(&self) -> Value {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Result)
            .clone()
    }

    pub(crate) fn publish_result(&self, value: &Value) {
        // Jim_SetResult owns the incoming result before retiring the previous
        // owner. Its callback still observes the previous indexed header.
        let incoming = value.clone();
        let slot = &self.objects
            [tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Result as usize];
        let retired = {
            let mut current = slot.borrow_mut();
            let observed = current
                .as_ref()
                .map(|original| original.native_lifetime_lease().into_value());
            std::mem::replace(&mut *current, observed)
        };
        drop(retired);
        let observed = slot.replace(Some(incoming));
        drop(observed);
    }

    pub(crate) fn replace_current_filename(
        &self,
        value: &Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let context = self
            .object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::CurrentFilename)
            .native_jim_context()?;
        value.bind_native_jim_context(&context)?;
        let retired = self.objects
            [tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::CurrentFilename as usize]
            .borrow_mut()
            .replace(value.clone());
        drop(retired);
        Ok(())
    }
}

impl Drop for NativeJimObjectContext {
    fn drop(&mut self) {
        // Interpreter teardown already consumes these roles when installed.
        // Standalone selected contexts own the same ordered references.
        for object in &mut self.objects {
            drop(object.get_mut().take());
        }
    }
}

impl Value {
    pub(crate) fn bind_native_jim_context(
        &self,
        context: &Rc<NativeJimObjectContext>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        native_jim_pool::validate_allocator(self, &context.pool)?;
        self.check_native_header()?;
        let selected = Rc::downgrade(context);
        let mut retained = self.0.jim_context.borrow_mut();
        if retained
            .as_ref()
            .is_some_and(|previous| !std::rc::Weak::ptr_eq(previous, &selected))
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original object interpreter",
            ));
        }
        *retained = Some(selected);
        Ok(())
    }

    /// Check existing allocator/context correspondence without binding a new source owner.
    pub(crate) fn check_native_jim_context(
        &self,
        context: &Rc<NativeJimObjectContext>,
    ) -> Result<(), ValueError> {
        native_jim_pool::validate_allocator(self, &context.pool)?;
        if !context.is_live()
            || self
                .0
                .jim_context
                .borrow()
                .as_ref()
                .is_some_and(|original| !std::rc::Weak::ptr_eq(original, &Rc::downgrade(context)))
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "foreign original Jim object context",
            ));
        }
        Ok(())
    }

    pub(crate) fn native_jim_context(
        &self,
    ) -> Result<Rc<NativeJimObjectContext>, tcl_syntax::value::ValueError> {
        self.0
            .jim_context
            .borrow()
            .as_ref()
            .and_then(std::rc::Weak::upgrade)
            .filter(|context| context.is_live())
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original source context",
            ))
    }

    /// Pin the actual filename once before native source conversion retires its owner.
    pub(crate) fn pin_native_jim_source_info(
        &self,
        context: &Rc<NativeJimObjectContext>,
    ) -> Result<NativeJimSourceInfo, tcl_syntax::value::ValueError> {
        match &*self.0.intrep.borrow() {
            IntRep::JimSource(source) => {
                if !std::rc::Weak::ptr_eq(&source.context, &Rc::downgrade(context)) {
                    return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim original source interpreter",
                    ));
                }
                Ok(source.info.clone())
            }
            IntRep::JimScript(header) => {
                if !std::rc::Weak::ptr_eq(&header.0.context, &Rc::downgrade(context)) {
                    return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim Script original interpreter",
                    ));
                }
                header.0.source_info()
            }
            _ => Ok(NativeJimSourceInfo {
                filename: context.empty_object().clone(),
                line: 1,
            }),
        }
    }

    /// Adopt Source on a real resident object; Source has no string updater.
    pub(crate) fn install_native_jim_source(
        &self,
        info: NativeJimSourceInfo,
        context: &Rc<NativeJimObjectContext>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.resident_string_bytes().is_none()
            || self.is_same_object(&info.filename)
            || self.native_object_is_shared()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original Source storage",
            ));
        }
        self.bind_native_jim_context(context)?;
        self.replace_primary(IntRep::JimSource(NativeJimSource {
            info,
            context: Rc::downgrade(context),
        }));
        Ok(())
    }

    pub(crate) fn native_jim_source_cache_present(&self) -> bool {
        matches!(*self.0.intrep.borrow(), IntRep::JimSource(_))
    }

    pub(crate) fn source_location(
        &self,
    ) -> Option<tcl_runtime_api::script_source_location::ScriptSourceLocation> {
        self.0.source_location.borrow().clone()
    }

    pub(crate) fn retain_source_location(
        &self,
        location: tcl_runtime_api::script_source_location::ScriptSourceLocation,
    ) {
        *self.0.source_location.borrow_mut() = Some(location);
    }
}
