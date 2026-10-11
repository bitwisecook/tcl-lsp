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

#[derive(Clone, Debug)]
pub(super) struct TableObservation {
    ledger: Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
    frame: Option<Weak<()>>,
}

impl TableObservation {
    pub(super) fn capture(
        frames: &FrameStack,
        namespaces: &Namespaces,
        home: VarHome,
    ) -> Option<Self> {
        let (table, frame) = match home {
            VarHome::Frame(level) => (
                frames.original_variable_table_at(level)?,
                Some(frames.original_activation_owner_at(level)?),
            ),
            VarHome::Namespace(namespace)
                if !namespaces.namespace_variables_are_deleted(namespace) =>
            {
                (namespaces.original_variable_table_at(namespace)?, None)
            }
            _ => return None,
        };
        Some(Self {
            ledger: table.original_table_ledger(),
            frame,
        })
    }
}

fn current(currency: &NativeOperationCurrency) -> Result<(), JimLinkStoreError> {
    currency
        .ensure_current_or_refuse()
        .map_err(|first| JimLinkStoreError::Command(CmdError::from_execution_refusal(first)))
}
pub(super) fn same_table(
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
    pub(crate) fn check_owner(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<(), JimLinkStoreError> {
        current(currency)?;
        same_table(interpreter, self.place.home, &self.table)?;
        for link in &self.links {
            link.check(interpreter, currency)?;
        }
        Ok(())
    }
    pub(crate) fn check_removal_current(
        &self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.check_owner(interpreter, currency).map_err(|error| {
            if let JimLinkStoreError::Command(error) = &error {
                if let Some(first) = error.native_execution_refusal() {
                    return first.clone();
                }
            }
            interpreter.clone().refuse_completion_value_access(
                ValueError::CommandProtocolUnavailable("original variable removal owner"),
            )
        })
    }
    pub(crate) fn trace_home(&self, interpreter: &Interp) -> TraceHome {
        let frames = interpreter.frames.borrow();
        let ns = interpreter.namespaces();
        let table = selected_table(&frames, &ns, self.place.home, true);
        let (namespace, level) = match self.place.home {
            VarHome::Namespace(namespace) => (Some(namespace), None),
            VarHome::Frame(level) => (None, Some(level)),
        };
        let binding = self
            .place
            .array_cell
            .as_ref()
            .map(crate::frame::RetainedArrayCell::identity)
            .or_else(|| {
                self.place
                    .scalar_entry
                    .as_ref()
                    .map(|entry| entry.identity())
            })
            .or_else(|| table.and_then(|table| table.trace_binding_identity(&self.place.name)));
        let selected_member = self.place.elem.as_ref().and_then(|element| {
            let identity = if let Some(array) = &self.place.array_cell {
                array.trace_member_identity(element)
            } else if let Some(entry) = &self.place.scalar_entry {
                entry.trace_member_identity(element)
            } else {
                table.and_then(|table| table.native_trace_element_identity(binding?, element))
            }?;
            Some((element.clone(), identity))
        });
        TraceHome {
            binding_id: binding,
            selected_member,
            ns: namespace,
            level,
            base: self.place.name.clone(),
            link_elem: self.place.elem.clone(),
        }
    }
    pub(crate) fn begin_array_destruction(
        &mut self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<Option<crate::frame::RetainedArrayCell>, JimLinkStoreError> {
        self.check(interpreter, currency)?;
        if self.place.elem.is_some() {
            return Ok(None);
        }
        let (array, old) = if let Some(entry) = &self.place.scalar_entry {
            let receiver = entry.capture_receiver(None, false)?;
            let result = receiver.as_ref().map_or_else(
                || (None, crate::frame::RetiredVariableParts::default()),
                |receiver| receiver.publish_array_destruction(),
            );
            drop(receiver);
            result
        } else {
            let mut frames = interpreter.frames.borrow_mut();
            let mut ns = interpreter.namespaces_mut();
            table_mut(&mut frames, &mut ns, self.place.home)
                .publish_array_destruction(&self.place.name)
        };
        let reached = old.finish(|| self.check_owner(interpreter, currency));
        if let Err(first) = reached {
            // This already selected detached generation must be retired on
            // refusal. It cannot be abandoned while optional traces stop.
            if let Some(array) = array {
                array.finish_destruction();
            }
            self.check_owner(interpreter, currency)?;
            return Err(first);
        }
        Ok(array)
    }
    pub(crate) fn remove(
        &mut self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
        require_defined_root: bool,
    ) -> Result<bool, JimLinkStoreError> {
        self.check(interpreter, currency)?;
        let defined = !require_defined_root
            || self.place.elem.is_some()
            || self.exists(interpreter, currency)?;
        let removed = if let Some(entry) = &self.place.scalar_entry {
            let receiver = entry.capture_receiver(self.place.elem.clone(), false)?;
            let reached = receiver
                .as_ref()
                .map_or(Ok(false), |receiver| receiver.unset_checked(currency));
            drop(receiver);
            self.check_owner(interpreter, currency)?;
            reached?
        } else if let (Some(array), Some(element)) = (&self.place.array_cell, &self.place.elem) {
            let (removed, old) = array.publish_remove(element);
            old.finish(|| self.check_owner(interpreter, currency))?;
            removed
        } else if let Some(root) = self
            .root
            .as_mut()
            .filter(|root| root.is_dictionary_element())
        {
            let reached = root.remove_dictionary_element(currency);
            self.check_owner(interpreter, currency)?;
            reached?
        } else if self.root.is_none() {
            false
        } else {
            if self.place.elem.is_none() {
                let old = {
                    let mut frames = interpreter.frames.borrow_mut();
                    let mut ns = interpreter.namespaces_mut();
                    table_mut(&mut frames, &mut ns, self.place.home)
                        .take_array_default(&self.place.name)
                };
                drop(old);
                // Default retirement is an earlier actual stage; a callback may
                // withdraw the original receiver before the removal is reached.
                self.check(interpreter, currency)?;
            }
            let (removed, old) = {
                let mut frames = interpreter.frames.borrow_mut();
                let mut ns = interpreter.namespaces_mut();
                let table = table_mut(&mut frames, &mut ns, self.place.home);
                match self.place.elem.as_deref() {
                    Some(element) => table.publish_remove_elem(&self.place.name, element),
                    None => table.publish_remove_after_default(&self.place.name),
                }
            };
            old.finish(|| self.check_owner(interpreter, currency))?;
            removed
        };
        let removed = removed && defined;
        if removed && self.place.elem.is_none() {
            interpreter
                .frames
                .borrow_mut()
                .invalidate_jim_variable_frame(self.place.home);
        }
        self.check_owner(interpreter, currency)?;
        Ok(removed)
    }
    pub(crate) fn remove_jim_primary(
        &mut self,
        interpreter: &Interp,
        currency: &NativeOperationCurrency,
    ) -> Result<bool, JimLinkStoreError> {
        self.check(interpreter, currency)?;
        if self.place.elem.is_some() {
            return Ok(false);
        }
        let primary = {
            let frames = interpreter.frames.borrow();
            let ns = interpreter.namespaces();
            selected_table(&frames, &ns, self.place.home, true)
                .is_some_and(|table| table.has_jim_primary(&self.place.name))
        };
        if primary {
            self.remove(interpreter, currency, false)
        } else {
            Ok(false)
        }
    }
    fn check_retained_value_role(&self) -> Result<(), JimLinkStoreError> {
        if let Some(entry) = &self.place.scalar_entry {
            entry.check_owned_value_role()?;
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
        self.check_retained_value_role()?;
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
        self.check_retained_value_role()?;
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
        self.check_retained_value_role()?;
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
        self.check_retained_value_role()?;
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
        self.check_retained_value_role()?;
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
