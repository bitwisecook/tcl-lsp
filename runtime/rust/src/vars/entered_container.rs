// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reached variable-table continuations under the original operation.
//! Classification and link steps remain in the shared variable resolver.

use super::*;
use crate::{
    frame::{CapturedJimLinkRoot, JimLinkStoreError},
    interp::{native_operation_currency::NativeOperationCurrency, Interp},
};
use std::{cell::RefCell, rc::Weak};
use tcl_cmd_core::CmdError;
use tcl_syntax::value::ValueError;

#[derive(Clone)]
struct TableObservation {
    ledger: Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
    frame: Option<Weak<()>>,
}

fn current(currency: &NativeOperationCurrency) -> Result<(), JimLinkStoreError> {
    currency
        .ensure_current_or_refuse()
        .map_err(|first| JimLinkStoreError::Command(CmdError::from_execution_refusal(first)))
}
fn same_table(
    interpreter: &Interp,
    home: VarHome,
    original: &TableObservation,
) -> Result<(), ValueError> {
    let frames = interpreter.frames.borrow();
    let namespaces = interpreter.namespaces();
    let frame_current = match (home, original.frame.as_ref()) {
        (VarHome::Frame(level), Some(original)) => frames
            .original_activation_owner_at(level)
            .is_some_and(|current| {
                original.strong_count() != 0 && Weak::ptr_eq(original, &current)
            }),
        (VarHome::Namespace(_), None) => true,
        _ => false,
    };
    let live = frame_current
        && match home {
            VarHome::Frame(level) => frames.original_variable_table_at(level),
            VarHome::Namespace(namespace)
                if !namespaces.namespace_variables_are_deleted(namespace) =>
            {
                namespaces.original_variable_table_at(namespace)
            }
            _ => None,
        }
        .is_some_and(|table| {
            original.ledger.strong_count() != 0
                && Weak::ptr_eq(&original.ledger, &table.original_table_ledger())
        });
    if live {
        Ok(())
    } else {
        Err(ValueError::CommandProtocolUnavailable(
            "captured variable container table",
        ))
    }
}
fn prepare_table(
    interpreter: &Interp,
    home: VarHome,
    currency: &NativeOperationCurrency,
) -> Result<TableObservation, JimLinkStoreError> {
    current(currency)?;
    let (original, selected) = match home {
        VarHome::Frame(level) => {
            let frames = interpreter.frames.borrow();
            let (ledger, selected) = frames
                .capture_container_model_at(level)
                .ok_or(VarError::NameProtocolUnavailable)?;
            (
                TableObservation {
                    ledger,
                    frame: Some(
                        frames
                            .original_activation_owner_at(level)
                            .ok_or(VarError::NameProtocolUnavailable)?,
                    ),
                },
                selected,
            )
        }
        VarHome::Namespace(namespace) => {
            let (ledger, selected) = interpreter
                .namespaces()
                .capture_container_model_at(namespace)
                .ok_or(VarError::NameProtocolUnavailable)?;
            (
                TableObservation {
                    ledger,
                    frame: None,
                },
                selected,
            )
        }
    };
    if let Some(selected) = selected {
        let reached = selected.finish(|| {
            currency.ensure_current_or_refuse().map_err(|_| {
                ValueError::CommandProtocolUnavailable("entered variable container transition")
            })?;
            same_table(interpreter, home, &original)
        });
        current(currency)?;
        reached?;
    }
    same_table(interpreter, home, &original)?;
    current(currency)?;
    Ok(original)
}
struct EnteredLink {
    home: VarHome,
    table: TableObservation,
    root: CapturedJimLinkRoot,
    link: Link,
}
impl EnteredLink {
    fn check(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<(), JimLinkStoreError> {
        current(currency)?;
        same_table(interpreter, self.home, &self.table)?;
        self.root.check_link(&self.link, currency)
    }
}

pub(crate) struct EnteredVariablePlace {
    place: Place,
    table: TableObservation,
    links: Vec<EnteredLink>,
    root: Option<CapturedJimLinkRoot>,
}
impl EnteredVariablePlace {
    pub(crate) fn capture(
        interpreter: &Interp,
        current_ns: NsId,
        level: Option<usize>,
        name: &[u8],
        element: Option<&[u8]>,
        create: bool,
        currency: &NativeOperationCurrency,
    ) -> Result<Self, JimLinkStoreError> {
        current(currency)?;
        let key = element
            .map(|key| separate_element(&interpreter.namespaces(), name, key))
            .transpose()?;
        let mut place = {
            let frames = interpreter.frames.borrow();
            let ns = interpreter.namespaces();
            match level.map_or_else(
                || classify_using(&frames, &ns, current_ns, name, true),
                |level| classify_at_using(&frames, &ns, name, level, true),
            ) {
                Resolved::Place(place) => place,
                Resolved::Error(error) => return Err(error.into()),
            }
        };
        let mut links = Vec::new();
        let mut selected = prepare_table(interpreter, place.home, currency)?;
        for _ in 0..LINK_LIMIT {
            if place.array_cell.is_some() || place.scalar_entry.is_some() {
                break;
            }
            for link in &links {
                EnteredLink::check(link, interpreter, currency)?;
            }
            let link = {
                let frames = interpreter.frames.borrow();
                let ns = interpreter.namespaces();
                selected_link_using(&frames, &ns, &place, true)
            };
            let Some(link) = link else { break };
            let root = {
                let frames = interpreter.frames.borrow();
                let ns = interpreter.namespaces();
                selected_table(&frames, &ns, place.home, true)
                    .ok_or(VarError::NameProtocolUnavailable)?
                    .capture_jim_link_root(&place.name, place.elem.clone())?
            };
            let observation = EnteredLink {
                home: place.home,
                table: selected.clone(),
                root,
                link: link.clone(),
            };
            if link.array_cell.is_none() && link.array_identity.is_some() {
                prepare_table(interpreter, link.home, currency)?;
            }
            observation.check(interpreter, currency)?;
            let self_link =
                link.home == place.home && link.name == place.name && link.elem.is_none();
            let next = {
                let frames = interpreter.frames.borrow();
                let ns = interpreter.namespaces();
                follow_selected_link_using(
                    &frames,
                    &ns,
                    Place {
                        home: place.home,
                        name: place.name.clone(),
                        elem: place.elem.clone(),
                        array_cell: place.array_cell.clone(),
                        scalar_entry: place.scalar_entry.clone(),
                    },
                    link,
                    false,
                    true,
                )?
            };
            if self_link {
                break;
            }
            links.push(observation);
            place = next.expect("non-self original variable link");
            selected = prepare_table(interpreter, place.home, currency)?;
        }
        if let Some(key) = key {
            if place.elem.is_some() {
                return Err(VarError::IsScalar.into());
            }
            place.elem = Some(key);
        }
        let root = if place.array_cell.is_some() || place.scalar_entry.is_some() {
            None
        } else {
            let mut frames = interpreter.frames.borrow_mut();
            let mut ns = interpreter.namespaces_mut();
            let table = table_mut(&mut frames, &mut ns, place.home);
            if create {
                Some(table.capture_byte_store_root(&place.name, place.elem.clone())?)
            } else if table.cell(&place.name).is_some() {
                Some(table.capture_jim_link_root(&place.name, place.elem.clone())?)
            } else {
                None
            }
        };
        let result = Self {
            place,
            table: selected,
            links,
            root,
        };
        result.check(interpreter, currency)?;
        Ok(result)
    }
    pub(crate) fn check(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<(), JimLinkStoreError> {
        current(currency)?;
        same_table(interpreter, self.place.home, &self.table)?;
        for link in &self.links {
            link.check(interpreter, currency)?;
        }
        if let Some(root) = &self.root {
            root.check(currency)?;
        }
        Ok(())
    }
    pub(crate) fn read(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
        default: bool,
    ) -> Result<Option<*mut TclObj>, JimLinkStoreError> {
        self.check(interpreter, currency)?;
        let reached = match read_retained(&self.place) {
            Some(reached) => reached.map_err(Into::into),
            None => self
                .root
                .as_ref()
                .map_or(Ok(None), |root| root.read(currency, default)),
        };
        self.check(interpreter, currency)?;
        reached
    }
    pub(crate) fn exists(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<bool, JimLinkStoreError> {
        self.check(interpreter, currency)?;
        let reached = match exists_retained(&self.place) {
            Some(defined) => Ok(defined),
            None => self
                .root
                .as_ref()
                .map_or(Ok(false), |root| root.exists(currency)),
        };
        self.check(interpreter, currency)?;
        reached
    }
    pub(crate) fn is_array(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<bool, JimLinkStoreError> {
        self.check(interpreter, currency)?;
        let reached = if self.place.elem.is_some() {
            Ok(false)
        } else if let Some(array) = &self.place.array_cell {
            Ok(array.is_live())
        } else if let Some(entry) = &self.place.scalar_entry {
            entry
                .capture_receiver(None, false)
                .map(|value| value.is_some_and(|cell| cell.is_array()))
                .map_err(Into::into)
        } else {
            self.root
                .as_ref()
                .map_or(Ok(false), |root| root.is_array(currency))
        };
        self.check(interpreter, currency)?;
        reached
    }
    pub(crate) fn array_names(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<Option<Vec<Vec<u8>>>, JimLinkStoreError> {
        self.check(interpreter, currency)?;
        let reached = if self.place.elem.is_some() {
            Ok(None)
        } else if let Some(entry) = &self.place.scalar_entry {
            entry.array_names().map_err(Into::into)
        } else if let Some(array) = &self.place.array_cell {
            Ok(Some(array.elements()))
        } else {
            self.root
                .as_ref()
                .map_or(Ok(None), |root| root.array_names(currency))
        };
        self.check(interpreter, currency)?;
        reached
    }
    pub(crate) fn ensure_array(
        &mut self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<(), JimLinkStoreError> {
        self.check(interpreter, currency)?;
        if self.place.elem.is_some() {
            return Err(VarError::IsScalar.into());
        }
        let reached = if let Some(array) = &self.place.array_cell {
            if array.is_live() {
                Ok(())
            } else {
                Err(VarError::DeletedArray.into())
            }
        } else if let Some(entry) = &self.place.scalar_entry {
            entry.ensure_array().map_err(Into::into)
        } else {
            self.root
                .as_mut()
                .ok_or(VarError::NameProtocolUnavailable)?
                .ensure_array(currency)
        };
        self.check(interpreter, currency)?;
        reached
    }
    pub(crate) fn into_store(
        self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<CapturedByteStore, JimLinkStoreError> {
        self.check(interpreter, currency)?;
        if let (Some(array), Some(element)) = (&self.place.array_cell, &self.place.elem) {
            return Ok(CapturedByteStore::Retained(
                array.capture_receiver(element.clone()),
            ));
        }
        if let Some(entry) = self.place.scalar_entry {
            return Ok(CapturedByteStore::Retained(
                entry
                    .capture_receiver(self.place.elem, true)?
                    .ok_or(VarError::NameProtocolUnavailable)?,
            ));
        }
        Ok(CapturedByteStore::Generic {
            home: self.place.home,
            root: self.root.ok_or(VarError::NameProtocolUnavailable)?,
        })
    }
}
pub(crate) fn capture_byte_store_entered(
    interpreter: &Interp,
    current_ns: NsId,
    level: Option<usize>,
    name: &[u8],
    element: Option<&[u8]>,
    currency: &NativeOperationCurrency,
) -> Result<CapturedByteStore, JimLinkStoreError> {
    EnteredVariablePlace::capture(
        interpreter,
        current_ns,
        level,
        name,
        element,
        true,
        currency,
    )?
    .into_store(interpreter, currency)
}
