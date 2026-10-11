// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original shallow Jim local publication, independent of target resolution.
use super::*;
use crate::{
    frame::{PreparedJimAliasCell, RetiredVariableParts, WeakJimVariableCell},
    interp::{native_operation_currency::NativeOperationCurrency, Interp},
};
use tcl_cmd_core::CmdError;
use tcl_syntax::value::ValueError;

pub(crate) struct PreparedJimAliasLocal {
    home: VarHome,
    table: entered_container::TableObservation,
    frame: std::rc::Weak<()>,
    global: bool,
    cell: PreparedJimAliasCell,
}
impl PreparedJimAliasLocal {
    fn error(interpreter: &Interp, error: ValueError) -> CmdError {
        CmdError::from_execution_refusal(interpreter.clone().refuse_completion_value_access(error))
    }
    pub(crate) fn from_cache(
        interpreter: &Interp,
        original: &WeakJimVariableCell,
        global: bool,
        currency: &NativeOperationCurrency,
    ) -> Result<Option<Self>, CmdError> {
        currency.ensure_current_or_refuse()?;
        let reached = (|| {
            let frames = interpreter.frames.borrow();
            let namespaces = interpreter.namespaces();
            let home = original
                .home()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original cached Jim local home",
                ))?;
            let observation =
                entered_container::TableObservation::capture(&frames, &namespaces, home).ok_or(
                    ValueError::CommandProtocolUnavailable("original Jim local table"),
                )?;
            let Some(cell) = original.prepare_alias_local()? else {
                return Ok(None);
            };
            let result = Self {
                home,
                table: observation,
                frame: frames.original_jim_frame_incarnation(global),
                global,
                cell,
            };
            if !table(&frames, &namespaces, home)
                .is_some_and(|table| result.cell.matches_table(table))
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original cached Jim local table",
                ));
            }
            Ok(Some(result))
        })();
        currency.ensure_current_or_refuse()?;
        let result = reached.map_err(|error| Self::error(interpreter, error))?;
        if let Some(result) = &result {
            result.check(interpreter, currency)?;
        }
        Ok(result)
    }
    pub(crate) fn from_bytes(
        interpreter: &Interp,
        local: &[u8],
        currency: &NativeOperationCurrency,
    ) -> Result<Option<Self>, CmdError> {
        currency.ensure_current_or_refuse()?;
        let reached = (|| {
            let mut frames = interpreter.frames.borrow_mut();
            let mut namespaces = interpreter.namespaces_mut();
            let Resolved::Place(place) =
                classify(&frames, &namespaces, interpreter.current_ns(), local)
            else {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim shallow local selection",
                ));
            };
            let observation =
                entered_container::TableObservation::capture(&frames, &namespaces, place.home)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original Jim local table",
                    ))?;
            let global = local.starts_with(b"::");
            let frame = frames.original_jim_frame_incarnation(global);
            let Some(cell) = table_mut(&mut frames, &mut namespaces, place.home)
                .prepare_jim_alias_local(&place.name)?
            else {
                return Ok(None);
            };
            Ok(Some(Self {
                home: place.home,
                table: observation,
                frame,
                global,
                cell,
            }))
        })();
        currency.ensure_current_or_refuse()?;
        let result = reached.map_err(|error| Self::error(interpreter, error))?;
        if let Some(result) = &result {
            result.check(interpreter, currency)?;
        }
        Ok(result)
    }
    pub(crate) fn check(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<(), CmdError> {
        currency.ensure_current_or_refuse()?;
        let reached = (|| {
            entered_container::same_table(interpreter, self.home, &self.table)?;
            let frames = interpreter.frames.borrow();
            let namespaces = interpreter.namespaces();
            if !frames.owns_original_jim_frame_incarnation(&self.frame)
                || !table(&frames, &namespaces, self.home)
                    .is_some_and(|table| self.cell.matches_table(table))
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim local frame or binding",
                ));
            }
            self.cell.check()
        })();
        currency.ensure_current_or_refuse()?;
        reached.map_err(|error| Self::error(interpreter, error))
    }
    pub(crate) fn global(&self) -> bool {
        self.global
    }
    pub(crate) fn was_missing(&self) -> bool {
        self.cell.was_missing()
    }
    pub(crate) fn detach(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<(), CmdError> {
        self.check(interpreter, currency)?;
        let reached = self.cell.detach();
        let retired = reached.map_err(|error| Self::error(interpreter, error))?;
        retired.finish(|| self.check(interpreter, currency))
    }
    pub(crate) fn begin_assignment(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<RetiredVariableParts, CmdError> {
        self.check(interpreter, currency)?;
        self.cell
            .begin_assignment()
            .map_err(|error| Self::error(interpreter, error))
    }
    pub(crate) fn assign(
        &self,
        interpreter: &Interp,
        incoming: obj::Owned,
        currency: &NativeOperationCurrency,
    ) -> Result<(), CmdError> {
        if let Err(first) = self.check(interpreter, currency) {
            drop(incoming);
            let _ = currency.ensure_current_or_refuse();
            return Err(first);
        }
        match self.cell.assign(incoming) {
            Ok(()) => self.check(interpreter, currency),
            Err((error, incoming)) => {
                let first = Self::error(interpreter, error);
                drop(incoming);
                let _ = currency.ensure_current_or_refuse();
                Err(first)
            }
        }
    }
    pub(crate) fn mark_link(
        &self,
        interpreter: &Interp,
        link: Link,
        frame: tcl_runtime_api::jim_call_frame::JimCallFrameStorageReference,
        currency: &NativeOperationCurrency,
    ) -> Result<(), CmdError> {
        self.check(interpreter, currency)?;
        match self.cell.mark_link(link, frame) {
            Ok(()) => self.check(interpreter, currency),
            Err((error, link)) => {
                let first = Self::error(interpreter, error);
                drop(link);
                let _ = currency.ensure_current_or_refuse();
                Err(first)
            }
        }
    }
    pub(crate) fn cache_cell(&self) -> WeakJimVariableCell {
        self.cell.weak_cell(self.home)
    }
    pub(crate) fn retain_key(
        &self,
        interpreter: &Interp,
        original: *mut TclObj,
        currency: &NativeOperationCurrency,
    ) -> Result<(), CmdError> {
        self.check(interpreter, currency)?;
        let frames = interpreter.frames.borrow();
        let namespaces = interpreter.namespaces();
        table(&frames, &namespaces, self.home)
            .expect("checked original Jim table")
            .retain_jim_key(self.cell.name(), original);
        drop(namespaces);
        drop(frames);
        self.check(interpreter, currency)
    }
    pub(crate) fn clear_retiring(&self) {
        self.cell.clear_retiring();
    }
}

impl Drop for PreparedJimAliasLocal {
    fn drop(&mut self) {
        self.cell.clear_retiring();
    }
}
/// Jim retains its target frame/name for later recursive lookup, rather than
/// taking a target variable's native role. Only actual table/holder birth is
/// observed here; a legal target-cell replacement remains late binding.
pub(crate) struct CapturedJimAliasTarget {
    home: VarHome,
    table: entered_container::TableObservation,
    link: Option<Link>,
}
impl CapturedJimAliasTarget {
    pub(crate) fn capture(
        interpreter: &Interp,
        name: &[u8],
        level: usize,
        currency: &NativeOperationCurrency,
    ) -> Result<Self, CmdError> {
        currency.ensure_current_or_refuse()?;
        let reached = (|| {
            let frames = interpreter.frames.borrow();
            let namespaces = interpreter.namespaces();
            let Resolved::Place(place) = classify_at(&frames, &namespaces, name, level) else {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim target home",
                ));
            };
            let observation =
                entered_container::TableObservation::capture(&frames, &namespaces, place.home)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original Jim target table",
                    ))?;
            let link = Link {
                original_jim_target: None,
                native_scalar_entry: None,
                native_element_entry: None,
                array_identity: None,
                array_cell: None,
                home: place.home,
                name: place.name,
                elem: None,
            };
            Ok(Self {
                home: place.home,
                table: observation,
                link: Some(link),
            })
        })();
        currency.ensure_current_or_refuse()?;
        let result = reached.map_err(|error| PreparedJimAliasLocal::error(interpreter, error))?;
        result.check(interpreter, currency)?;
        Ok(result)
    }
    pub(crate) fn check(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<(), CmdError> {
        currency.ensure_current_or_refuse()?;
        let reached = entered_container::same_table(interpreter, self.home, &self.table);
        currency.ensure_current_or_refuse()?;
        reached.map_err(|error| PreparedJimAliasLocal::error(interpreter, error))
    }
    pub(crate) fn take_link(&mut self) -> Link {
        self.link.take().expect("actual original target metadata")
    }
}
