// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite Import observations from the actual entered Source and native loops.

use super::{
    native_arguments::OriginalArgumentFrame, native_script::OriginalSourceInvocation, Code, Interp,
    ProcDef,
};
use crate::{
    native_source::NativeJimSourceOrigin,
    obj::{NativeObjectLifetime, TclObj},
};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    rc::{Rc, Weak},
};
use tcl_syntax::value::{ValueError, ValueOps};

type Origin = Rc<NativeJimSourceOrigin>;
fn unavailable(stage: &'static str) -> ValueError {
    ValueError::CommandProtocolUnavailable(stage)
}

pub(crate) struct OriginalJimImportTraversal {
    frame: OriginalArgumentFrame,
    procedure: Weak<ProcDef>,
    root: Origin,
    namespace: super::native_jim_lookup::OriginalJimLookupNamespace,
    current_namespace: crate::namespace::NsId,
    state: RefCell<Traversal>,
}

#[derive(Default)]
struct Traversal {
    outer_body: Option<Origin>,
    outer_finished: bool,
    pattern_active: bool,
    inner: Option<Rc<Inner>>,
}

struct Inner {
    body: Origin,
    state: RefCell<InnerState>,
}
#[derive(Default)]
struct InnerState {
    finished: bool,
    candidate: Option<Candidate>,
}
struct Candidate {
    original: NativeObjectLifetime,
    alias: NativeObjectLifetime,
    seen: BTreeSet<Vec<u8>>,
    walk: Option<Rc<Walk>>,
    published: bool,
}
struct Walk {
    condition: Origin,
    body: Origin,
    state: RefCell<WalkState>,
}
#[derive(Default)]
struct WalkState {
    outcome: Option<bool>,
    continuation: bool,
    complete: bool,
}

pub(crate) struct OriginalJimImportEachLoop {
    traversal: Rc<OriginalJimImportTraversal>,
    invocation: Rc<OriginalSourceInvocation>,
    role: EachRole,
}
enum EachRole {
    Patterns,
    Commands(Rc<Inner>),
}

pub(crate) struct OriginalJimImportWhile {
    traversal: Rc<OriginalJimImportTraversal>,
    invocation: Rc<OriginalSourceInvocation>,
    inner: Rc<Inner>,
    walk: Rc<Walk>,
}

fn operand_source(
    source: &OriginalSourceInvocation,
    index: usize,
    original: *mut TclObj,
) -> Result<Origin, ValueError> {
    source
        .operands
        .get(index)
        .and_then(Option::as_ref)
        .filter(|operand| !operand.returned && operand.source.belongs_to(original))
        .map(|operand| Rc::clone(&operand.source))
        .ok_or_else(|| unavailable("original Import literal Source operand"))
}

impl OriginalJimImportTraversal {
    pub(super) fn capture(
        actual: &Interp,
        procedure: &Rc<ProcDef>,
    ) -> Result<Option<Rc<Self>>, ValueError> {
        let Some(root) = procedure.original_jim_import_origin(actual)? else {
            return Ok(None);
        };
        let current = actual.frames.borrow().current_original_jim_procedure()?;
        if !current.is_some_and(|current| Rc::ptr_eq(&current, procedure))
            || actual
                .frames
                .borrow()
                .current_original_jim_variadic()?
                .is_none()
        {
            return Err(unavailable(
                "original Import entered procedure and Rest List",
            ));
        }
        Ok(Some(Rc::new(Self {
            frame: OriginalArgumentFrame::capture(actual),
            procedure: Rc::downgrade(procedure),
            root,
            namespace: super::native_jim_lookup::OriginalJimLookupNamespace::capture(
                actual,
                actual.current_ns(),
            )?,
            current_namespace: actual.current_ns(),
            state: RefCell::new(Traversal::default()),
        })))
    }

    fn check(&self, actual: &Interp) -> Result<(), ValueError> {
        self.frame.check_frame(actual)?;
        self.namespace.check(actual)?;
        if actual.current_ns() != self.current_namespace {
            return Err(unavailable("original Import current namespace"));
        }
        let current = actual.frames.borrow().current_original_jim_procedure()?;
        let Some((owner, current)) = self.procedure.upgrade().zip(current) else {
            return Err(unavailable("original Import traversal procedure"));
        };
        if !Rc::ptr_eq(&owner, &current)
            || !owner
                .original_jim_import_origin(actual)?
                .is_some_and(|root| Rc::ptr_eq(&root, &self.root))
        {
            return Err(unavailable("original Import traversal Source owner"));
        }
        Ok(())
    }

    pub(super) fn normal_completion(&self, actual: &Interp) -> Result<(), ValueError> {
        self.check(actual)?;
        if !self.state.borrow().outer_finished {
            return Err(unavailable("original Import outer iterator completion"));
        }
        Ok(())
    }

    fn current_inner(&self) -> Result<Rc<Inner>, ValueError> {
        self.state
            .borrow()
            .inner
            .as_ref()
            .cloned()
            .ok_or_else(|| unavailable("original Import command iterator"))
    }
}

impl OriginalJimImportEachLoop {
    pub(crate) fn check_values(
        &self,
        actual: &Interp,
        values: *mut TclObj,
    ) -> Result<(), ValueError> {
        self.traversal.check(actual)?;
        self.invocation.check_retained(actual)?;
        match &self.role {
            EachRole::Patterns => {
                if actual.frames.borrow().current_original_jim_variadic()? != Some(values) {
                    return Err(unavailable("original Import iterator Rest List"));
                }
            }
            EachRole::Commands(_) => {
                actual
                    .original_source_command_return(2, values)?
                    .ok_or_else(|| unavailable("original Import inventory Command operand"))?
                    .check_inventory(actual, values)?;
            }
        }
        Ok(())
    }

    pub(crate) fn assigned(
        &self,
        actual: &Interp,
        group: usize,
        variable: usize,
        original: Option<*mut TclObj>,
    ) -> Result<(), ValueError> {
        self.traversal.check(actual)?;
        self.invocation.check_retained(actual)?;
        if group != 0 || variable != 0 {
            return Err(unavailable("original Import iterator assignment"));
        }
        let original = original.ok_or_else(|| unavailable("original Import iterator padding"))?;
        crate::obj::check_native_liveness(original)?;
        match &self.role {
            EachRole::Patterns => {
                let mut state = self.traversal.state.borrow_mut();
                if state.pattern_active
                    && !state
                        .inner
                        .as_ref()
                        .is_some_and(|inner| inner.state.borrow().finished)
                {
                    return Err(unavailable("original Import previous pattern completion"));
                }
                state.pattern_active = true;
                state.inner = None;
            }
            EachRole::Commands(inner) => {
                let mut state = inner.state.borrow_mut();
                if state.finished
                    || state
                        .candidate
                        .as_ref()
                        .is_some_and(|candidate| !candidate.published)
                {
                    return Err(unavailable(
                        "original Import previous candidate publication",
                    ));
                }
                state.candidate = Some(Candidate {
                    original: NativeObjectLifetime::retain(original),
                    alias: NativeObjectLifetime::retain(original),
                    seen: BTreeSet::new(),
                    walk: None,
                    published: false,
                });
            }
        }
        Ok(())
    }

    pub(crate) fn body_completed(&self, actual: &Interp, code: Code) -> Result<(), ValueError> {
        self.traversal.check(actual)?;
        self.invocation.check_retained(actual)?;
        if code == Code::Break {
            return Err(unavailable(
                "original Import interrupted inventory traversal",
            ));
        }
        if code != Code::Ok {
            return Ok(());
        }
        match &self.role {
            EachRole::Patterns => {
                if !self
                    .traversal
                    .state
                    .borrow()
                    .inner
                    .as_ref()
                    .is_some_and(|inner| inner.state.borrow().finished)
                {
                    return Err(unavailable("original Import pattern body completion"));
                }
            }
            EachRole::Commands(inner) => {
                if !inner
                    .state
                    .borrow()
                    .candidate
                    .as_ref()
                    .is_some_and(|candidate| candidate.published)
                {
                    return Err(unavailable("original Import candidate body completion"));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn finish(&self, actual: &Interp) -> Result<(), ValueError> {
        self.traversal.check(actual)?;
        self.invocation.check_retained(actual)?;
        match &self.role {
            EachRole::Patterns => {
                let mut state = self.traversal.state.borrow_mut();
                if state.pattern_active
                    && !state
                        .inner
                        .as_ref()
                        .is_some_and(|inner| inner.state.borrow().finished)
                {
                    return Err(unavailable("original Import final pattern completion"));
                }
                state.outer_finished = true;
            }
            EachRole::Commands(inner) => {
                let mut state = inner.state.borrow_mut();
                if state
                    .candidate
                    .as_ref()
                    .is_some_and(|candidate| !candidate.published)
                {
                    return Err(unavailable("original Import final candidate publication"));
                }
                state.finished = true;
            }
        }
        Ok(())
    }
}

impl OriginalJimImportWhile {
    pub(crate) fn condition_start(&self, actual: &Interp) -> Result<(), ValueError> {
        self.traversal.check(actual)?;
        self.invocation.check_retained(actual)?;
        if !self
            .inner
            .state
            .borrow()
            .candidate
            .as_ref()
            .and_then(|candidate| candidate.walk.as_ref())
            .is_some_and(|walk| Rc::ptr_eq(walk, &self.walk))
        {
            return Err(unavailable("original Import current candidate loop"));
        }
        let mut state = self.walk.state.borrow_mut();
        if state.complete || state.outcome.is_some() {
            return Err(unavailable("original Import condition sequence"));
        }
        state.continuation = false;
        Ok(())
    }
    pub(crate) fn condition_completed(
        &self,
        actual: &Interp,
        value: bool,
    ) -> Result<(), ValueError> {
        self.traversal.check(actual)?;
        self.invocation.check_retained(actual)?;
        let mut state = self.walk.state.borrow_mut();
        if state.outcome.take() != Some(value) {
            return Err(unavailable("original Import Alias condition producer"));
        }
        if !value {
            state.complete = true;
        }
        Ok(())
    }
    pub(crate) fn body_completed(&self, actual: &Interp, code: Code) -> Result<(), ValueError> {
        self.traversal.check(actual)?;
        self.invocation.check_retained(actual)?;
        if code == Code::Ok && !self.walk.state.borrow().continuation {
            return Err(unavailable("original Import alias-prefix continuation"));
        }
        Ok(())
    }
}

impl Interp {
    pub(crate) fn original_import_each_loop(
        &self,
        argv: &[*mut TclObj],
        collect: bool,
        variables: &[usize],
    ) -> Result<Option<OriginalJimImportEachLoop>, ValueError> {
        let Some(traversal) = self
            .frames
            .borrow()
            .current_original_jim_import_traversal()?
        else {
            return Ok(None);
        };
        let Some(invocation) = self.current_original_source_invocation()? else {
            return Ok(None);
        };
        traversal.check(self)?;
        invocation.check(self, argv)?;
        if collect || argv.len() != 4 || variables != [1] {
            return Err(unavailable("original Import native foreach shape"));
        }
        let body = operand_source(&invocation, 3, argv[3])?;
        let role = {
            let mut state = traversal.state.borrow_mut();
            if Rc::ptr_eq(&invocation.root, &traversal.root) {
                if state.outer_body.is_some() || state.outer_finished {
                    return Err(unavailable("original Import outer iterator sequence"));
                }
                if self.frames.borrow().current_original_jim_variadic()? != Some(argv[2]) {
                    return Err(unavailable("original Import outer Rest operand"));
                }
                state.outer_body = Some(body);
                EachRole::Patterns
            } else if state
                .outer_body
                .as_ref()
                .is_some_and(|outer| Rc::ptr_eq(outer, &invocation.root))
                && state.pattern_active
            {
                if state.inner.is_some() {
                    return Err(unavailable("original Import inner iterator sequence"));
                }
                let inner = Rc::new(Inner {
                    body,
                    state: RefCell::new(InnerState::default()),
                });
                state.inner = Some(Rc::clone(&inner));
                EachRole::Commands(inner)
            } else {
                return Err(unavailable("original Import iterator Source role"));
            }
        };
        Ok(Some(OriginalJimImportEachLoop {
            traversal,
            invocation,
            role,
        }))
    }

    pub(crate) fn original_import_while(
        &self,
        argv: &[*mut TclObj],
    ) -> Result<Option<OriginalJimImportWhile>, ValueError> {
        let Some(traversal) = self
            .frames
            .borrow()
            .current_original_jim_import_traversal()?
        else {
            return Ok(None);
        };
        let Some(invocation) = self.current_original_source_invocation()? else {
            return Ok(None);
        };
        traversal.check(self)?;
        invocation.check(self, argv)?;
        if argv.len() != 3 {
            return Err(unavailable("original Import native while shape"));
        }
        let inner = traversal.current_inner()?;
        if !Rc::ptr_eq(&invocation.root, &inner.body) {
            return Err(unavailable("original Import while Source role"));
        }
        let walk = Rc::new(Walk {
            condition: operand_source(&invocation, 1, argv[1])?,
            body: operand_source(&invocation, 2, argv[2])?,
            state: RefCell::new(WalkState::default()),
        });
        let mut state = inner.state.borrow_mut();
        let candidate = state
            .candidate
            .as_mut()
            .ok_or_else(|| unavailable("original Import while candidate"))?;
        if candidate.walk.is_some() || candidate.published {
            return Err(unavailable("original Import while sequence"));
        }
        candidate.walk = Some(Rc::clone(&walk));
        drop(state);
        Ok(Some(OriginalJimImportWhile {
            traversal,
            invocation,
            inner,
            walk,
        }))
    }

    pub(crate) fn original_import_alias_exists(
        &self,
        original: *mut TclObj,
        exists: bool,
    ) -> Result<(), ValueError> {
        let Some(traversal) = self
            .frames
            .borrow()
            .current_original_jim_import_traversal()?
        else {
            return Ok(());
        };
        let Some(invocation) = self.current_original_source_invocation()? else {
            return Ok(());
        };
        traversal.check(self)?;
        let inner = traversal.current_inner()?;
        let mut state = inner.state.borrow_mut();
        let candidate = state
            .candidate
            .as_mut()
            .ok_or_else(|| unavailable("original Import Alias condition candidate"))?;
        let walk = candidate
            .walk
            .as_ref()
            .ok_or_else(|| unavailable("original Import Alias condition loop"))?;
        if !invocation.root.is_child_of(&walk.condition)
            || candidate.alias.checked_ptr()? != original
        {
            return Err(unavailable(
                "original Import Alias condition Source operand",
            ));
        }
        let mut condition = walk.state.borrow_mut();
        if condition.outcome.is_some() || condition.complete {
            return Err(unavailable("original Import duplicate Alias condition"));
        }
        if exists {
            let resident = self
                .native_object_snapshot(&original)?
                .resident
                .ok_or_else(|| unavailable("Jim import reached alias spelling"))?;
            if !candidate.seen.insert(resident.to_vec()) {
                return Err(unavailable("Jim namespace import cyclic alias chain"));
            }
        }
        condition.outcome = Some(exists);
        Ok(())
    }

    pub(crate) fn original_import_alias_prefix(
        &self,
        original: *mut TclObj,
        prefix: *mut TclObj,
    ) -> Result<(), ValueError> {
        let Some(traversal) = self
            .frames
            .borrow()
            .current_original_jim_import_traversal()?
        else {
            return Ok(());
        };
        let Some(invocation) = self.current_original_source_invocation()? else {
            return Ok(());
        };
        traversal.check(self)?;
        let inner = traversal.current_inner()?;
        let mut state = inner.state.borrow_mut();
        let candidate = state
            .candidate
            .as_mut()
            .ok_or_else(|| unavailable("original Import prefix candidate"))?;
        let walk = candidate
            .walk
            .as_ref()
            .ok_or_else(|| unavailable("original Import prefix loop"))?;
        if !invocation.root.is_child_of(&walk.body) || candidate.alias.checked_ptr()? != original {
            return Err(unavailable("original Import prefix Source operand"));
        }
        let mut condition = walk.state.borrow_mut();
        if condition.complete || condition.outcome.is_some() || condition.continuation {
            return Err(unavailable("original Import prefix sequence"));
        }
        crate::obj::check_native_liveness(prefix)?;
        candidate.alias = NativeObjectLifetime::retain(prefix);
        condition.continuation = true;
        Ok(())
    }

    pub(crate) fn original_import_alias_publication(
        &self,
        argv: &[*mut TclObj],
        completed: bool,
    ) -> Result<(), ValueError> {
        let Some(traversal) = self
            .frames
            .borrow()
            .current_original_jim_import_traversal()?
        else {
            return Ok(());
        };
        let Some(invocation) = self.current_original_source_invocation()? else {
            return Ok(());
        };
        traversal.check(self)?;
        invocation.check(self, argv)?;
        let inner = traversal.current_inner()?;
        let mut state = inner.state.borrow_mut();
        let candidate = state
            .candidate
            .as_mut()
            .ok_or_else(|| unavailable("original Import publication candidate"))?;
        if !Rc::ptr_eq(&invocation.root, &inner.body)
            || argv.len() != 3
            || candidate.original.checked_ptr()? != argv[2]
            || candidate.published
            || !candidate
                .walk
                .as_ref()
                .is_some_and(|walk| walk.state.borrow().complete)
        {
            return Err(unavailable("original Import finite candidate publication"));
        }
        if completed {
            candidate.published = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
