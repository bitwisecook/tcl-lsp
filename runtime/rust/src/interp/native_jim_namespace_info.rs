// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual namespace-info procedure, iterator and constructor completion owners.

use super::{
    native_arguments::OriginalArgumentFrame,
    native_jim_lookup::{
        OriginalJimCommandInventoryReturn, OriginalJimInventoryMember, OriginalJimLookupNamespace,
    },
    native_script::OriginalSourceInvocation,
    Code, Interp, ProcDef,
};
use crate::{
    native_source::NativeJimSourceOrigin,
    obj::{NativeObjectLifetime, TclObj},
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use tcl_syntax::value::ValueError;

type Data = Rc<OriginalJimCommandInventoryReturn>;
fn unavailable(stage: &'static str) -> ValueError {
    ValueError::CommandProtocolUnavailable(stage)
}

pub(crate) struct OriginalNamespaceInfoFrame {
    frame: OriginalArgumentFrame,
    procedure: Weak<ProcDef>,
    root: Rc<NativeJimSourceOrigin>,
    namespace: OriginalJimLookupNamespace,
    namespace_id: crate::namespace::NsId,
    receiver: Rc<OriginalSourceInvocation>,
    data: RefCell<Vec<Data>>,
    lmap: RefCell<Option<Rc<Lmap>>>,
    returned: RefCell<Option<Data>>,
    completed_body: RefCell<Option<Data>>,
}
struct Lmap {
    invocation: Rc<OriginalSourceInvocation>,
    body: Rc<NativeJimSourceOrigin>,
    input: Data,
    current: RefCell<Option<Iteration>>,
    output: RefCell<Vec<OriginalJimInventoryMember>>,
    finished: std::cell::Cell<bool>,
}
struct Iteration {
    index: usize,
    original: NativeObjectLifetime,
    output: Option<OriginalJimInventoryMember>,
}

pub(crate) struct OriginalNamespaceInfoEachLoop {
    owner: Rc<OriginalNamespaceInfoFrame>,
    lmap: Rc<Lmap>,
}

pub(super) struct OriginalNamespaceInfoCompletion {
    receiver: Rc<OriginalSourceInvocation>,
    data: Data,
}
impl OriginalNamespaceInfoCompletion {
    pub(super) fn publish(self, actual: &Interp) -> Result<(), ValueError> {
        self.receiver
            .receive_namespace_info_inventory(actual, self.data)
    }
}

impl OriginalNamespaceInfoFrame {
    pub(super) fn capture(
        actual: &Interp,
        procedure: &Rc<ProcDef>,
        receiver: Rc<OriginalSourceInvocation>,
    ) -> Result<Rc<Self>, ValueError> {
        let Some((kind, root)) = procedure.original_jim_namespace_helper_origin(actual)? else {
            return Err(unavailable("original NamespaceInfo installed Source"));
        };
        if kind != tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Info
            || !actual
                .frames
                .borrow()
                .current_original_jim_procedure()?
                .is_some_and(|current| Rc::ptr_eq(&current, procedure))
        {
            return Err(unavailable("original NamespaceInfo entered procedure"));
        }
        Ok(Rc::new(Self {
            frame: OriginalArgumentFrame::capture(actual),
            procedure: Rc::downgrade(procedure),
            root,
            namespace: OriginalJimLookupNamespace::capture(actual, actual.current_ns())?,
            namespace_id: actual.current_ns(),
            receiver,
            data: RefCell::new(Vec::new()),
            lmap: RefCell::new(None),
            returned: RefCell::new(None),
            completed_body: RefCell::new(None),
        }))
    }
    fn check(&self, actual: &Interp) -> Result<(), ValueError> {
        self.frame.check_frame(actual)?;
        self.namespace.check(actual)?;
        let Some((owner, current)) = self
            .procedure
            .upgrade()
            .zip(actual.frames.borrow().current_original_jim_procedure()?)
        else {
            return Err(unavailable("original NamespaceInfo current procedure"));
        };
        if actual.current_ns() != self.namespace_id
            || !Rc::ptr_eq(&owner, &current)
            || !owner
                .original_jim_namespace_helper_origin(actual)?
                .is_some_and(|(kind, root)| {
                    kind == tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Info
                        && Rc::ptr_eq(&root, &self.root)
                })
        {
            return Err(unavailable(
                "original NamespaceInfo current Source and namespace",
            ));
        }
        Ok(())
    }
    pub(super) fn record_data(&self, actual: &Interp, data: Data) -> Result<(), ValueError> {
        self.check(actual)?;
        data.check(actual, data.original()?)?;
        self.data.borrow_mut().push(data);
        Ok(())
    }
    fn data(&self, actual: &Interp, original: *mut TclObj) -> Result<Data, ValueError> {
        self.check(actual)?;
        let data = self
            .data
            .borrow()
            .iter()
            .rev()
            .find(|data| data.has_original_header(original))
            .cloned()
            .ok_or_else(|| unavailable("original NamespaceInfo result production"))?;
        data.check(actual, original)?;
        Ok(data)
    }
    pub(super) fn completion(
        &self,
        actual: &Interp,
    ) -> Result<OriginalNamespaceInfoCompletion, ValueError> {
        self.check(actual)?;
        let data = self
            .returned
            .borrow()
            .as_ref()
            .cloned()
            .ok_or_else(|| unavailable("original NamespaceInfo native return completion"))?;
        if !self
            .completed_body
            .borrow()
            .as_ref()
            .is_some_and(|completed| Rc::ptr_eq(completed, &data))
        {
            return Err(unavailable(
                "original NamespaceInfo completed transformation",
            ));
        }
        data.check(actual, actual.result_obj())?;
        Ok(OriginalNamespaceInfoCompletion {
            receiver: Rc::clone(&self.receiver),
            data,
        })
    }
}

impl OriginalNamespaceInfoEachLoop {
    pub(crate) fn check_values(
        &self,
        actual: &Interp,
        original: *mut TclObj,
    ) -> Result<(), ValueError> {
        self.owner.check(actual)?;
        self.lmap.invocation.check_retained(actual)?;
        self.lmap.input.check(actual, original)
    }
    pub(crate) fn assigned(
        &self,
        actual: &Interp,
        group: usize,
        variable: usize,
        selected: Option<(usize, *mut TclObj)>,
    ) -> Result<(), ValueError> {
        self.owner.check(actual)?;
        self.lmap.invocation.check_retained(actual)?;
        if group != 0
            || variable != 0
            || self.lmap.current.borrow().is_some()
            || self.lmap.finished.get()
        {
            return Err(unavailable(
                "original NamespaceInfo lmap assignment sequence",
            ));
        }
        let (index, original) =
            selected.ok_or_else(|| unavailable("original NamespaceInfo lmap padding"))?;
        if index != self.lmap.output.borrow().len()
            || self.lmap.input.member(actual, index)? != original
        {
            return Err(unavailable("original NamespaceInfo lmap original member"));
        }
        *self.lmap.current.borrow_mut() = Some(Iteration {
            index,
            original: NativeObjectLifetime::retain(original),
            output: None,
        });
        Ok(())
    }
    pub(crate) fn body_completed(&self, actual: &Interp, code: Code) -> Result<(), ValueError> {
        self.owner.check(actual)?;
        self.lmap.invocation.check_retained(actual)?;
        if code != Code::Ok {
            return Ok(());
        }
        let mut iteration = self
            .lmap
            .current
            .borrow_mut()
            .take()
            .ok_or_else(|| unavailable("original NamespaceInfo lmap entered body"))?;
        let output = iteration
            .output
            .take()
            .ok_or_else(|| unavailable("original NamespaceInfo native cat completion"))?;
        if output.original()? != actual.result_obj() {
            return Err(unavailable(
                "original NamespaceInfo lmap body result header",
            ));
        }
        self.lmap.output.borrow_mut().push(output);
        Ok(())
    }
    pub(crate) fn finish(&self, actual: &Interp, original: *mut TclObj) -> Result<(), ValueError> {
        self.owner.check(actual)?;
        self.lmap.invocation.check_retained(actual)?;
        self.lmap.input.check(actual, self.lmap.input.original()?)?;
        if self.lmap.finished.get()
            || self.lmap.current.borrow().is_some()
            || self.lmap.output.borrow().len() != self.lmap.input.len()
        {
            return Err(unavailable("original NamespaceInfo complete lmap coverage"));
        }
        let members = std::mem::take(&mut *self.lmap.output.borrow_mut());
        let data = Rc::new(OriginalJimCommandInventoryReturn::capture_members(
            actual, original, members,
        )?);
        self.owner.record_data(actual, Rc::clone(&data))?;
        *self.owner.completed_body.borrow_mut() = Some(Rc::clone(&data));
        actual.record_original_source_inventory_data(data)?;
        self.lmap.finished.set(true);
        Ok(())
    }
}

impl Interp {
    pub(crate) fn original_namespace_info_each_loop(
        &self,
        argv: &[*mut TclObj],
        collect: bool,
        variables: &[usize],
    ) -> Result<Option<OriginalNamespaceInfoEachLoop>, ValueError> {
        let Some(owner) = self
            .frames
            .borrow()
            .current_original_namespace_info_frame()?
        else {
            return Ok(None);
        };
        if !collect {
            return Ok(None);
        }
        owner.check(self)?;
        let invocation = self
            .current_original_source_invocation()?
            .ok_or_else(|| unavailable("original NamespaceInfo lmap Source invocation"))?;
        invocation.check(self, argv)?;
        if argv.len() != 4 || variables != [1] || owner.lmap.borrow().is_some() {
            return Err(unavailable(
                "original NamespaceInfo lmap shape and sequence",
            ));
        }
        let body = invocation.operands[3]
            .as_ref()
            .filter(|operand| !operand.returned && operand.source.belongs_to(argv[3]))
            .map(|operand| Rc::clone(&operand.source))
            .ok_or_else(|| unavailable("original NamespaceInfo lmap body Source"))?;
        let lmap = Rc::new(Lmap {
            invocation,
            body,
            input: owner.data(self, argv[2])?,
            current: RefCell::new(None),
            output: RefCell::new(Vec::new()),
            finished: std::cell::Cell::new(false),
        });
        *owner.lmap.borrow_mut() = Some(Rc::clone(&lmap));
        Ok(Some(OriginalNamespaceInfoEachLoop { owner, lmap }))
    }
    pub(crate) fn record_original_namespace_info_cat(
        &self,
        construction: &crate::value_ops::OriginalJimStringCatConstruction<'_>,
    ) -> Result<(), ValueError> {
        let Some(owner) = self
            .frames
            .borrow()
            .current_original_namespace_info_frame()?
        else {
            return Ok(());
        };
        owner.check(self)?;
        let (inputs, _, source) = construction.checked_parts(self)?;
        let source =
            source.ok_or_else(|| unavailable("original NamespaceInfo cat Source invocation"))?;
        let invocation = source.invocation();
        invocation.check_retained(self)?;
        invocation.check_suffix(self, &inputs)?;
        let lmap = owner
            .lmap
            .borrow()
            .as_ref()
            .cloned()
            .ok_or_else(|| unavailable("original NamespaceInfo cat iterator"))?;
        if inputs.len() != 2 || !Rc::ptr_eq(&invocation.root, &lmap.body) {
            return Err(unavailable("original NamespaceInfo cat selected body"));
        }
        let mut current = lmap.current.borrow_mut();
        let iteration = current
            .as_mut()
            .ok_or_else(|| unavailable("original NamespaceInfo cat yielded input"))?;
        if iteration.output.is_some() || iteration.original.checked_ptr()? != inputs[1] {
            return Err(unavailable("original NamespaceInfo cat original operand"));
        }
        iteration.output = Some(lmap.input.constructed_member(
            self,
            iteration.index,
            construction,
        )?);
        Ok(())
    }
    pub(crate) fn record_original_namespace_info_return(
        &self,
        argv: &[*mut TclObj],
        code: Code,
    ) -> Result<(), ValueError> {
        let Some(owner) = self
            .frames
            .borrow()
            .current_original_namespace_info_frame()?
        else {
            return Ok(());
        };
        if code != Code::Return {
            return Ok(());
        }
        owner.check(self)?;
        let invocation = self
            .current_original_source_invocation()?
            .ok_or_else(|| unavailable("original NamespaceInfo native return Source"))?;
        invocation.check(self, argv)?;
        if argv.len() != 2
            || !Rc::ptr_eq(&invocation.root, &owner.root)
            || owner.returned.borrow().is_some()
        {
            return Err(unavailable("original NamespaceInfo native return operands"));
        }
        let data = owner.data(self, argv[1])?;
        data.check(self, self.result_obj())?;
        *owner.returned.borrow_mut() = Some(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
