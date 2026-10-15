// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::native_jim_increment::OriginalJimDictionaryRoot;
use super::*;
use crate::dict::{OriginalDictionaryMutation, OriginalDictionarySet};

fn hold_pairs(
    pairs: &[(*mut TclObj, *mut TclObj)],
) -> Result<Vec<(NativeObjectLifetime, NativeObjectLifetime)>, ValueError> {
    pairs
        .iter()
        .map(|&(key, value)| {
            crate::obj::check_native_liveness(key)?;
            crate::obj::check_native_liveness(value)?;
            Ok((
                NativeObjectLifetime::retain(key),
                NativeObjectLifetime::retain(value),
            ))
        })
        .collect()
}

#[derive(Default)]
pub(super) struct LocalProjection {
    loops: Vec<Rc<Each>>,
    dictionary: Option<Dictionary>,
}
struct Dictionary {
    cell: crate::frame::CapturedJimDictionaryCell,
    original: NativeObjectLifetime,
    mutation: OriginalDictionaryMutation,
    backing: Weak<()>,
    pairs: Vec<(*mut TclObj, *mut TclObj)>,
    members: Vec<OriginalJimInventoryMember>,
    _pairs: Vec<(NativeObjectLifetime, NativeObjectLifetime)>,
}
struct Each {
    invocation: Rc<OriginalSourceInvocation>,
    body: Rc<NativeJimSourceOrigin>,
    input: Data,
    current: RefCell<Option<LocalIteration>>,
    completed: std::cell::Cell<usize>,
    finished: std::cell::Cell<bool>,
}
struct LocalIteration {
    index: usize,
    original: NativeObjectLifetime,
    ranged: Option<OriginalJimInventoryMember>,
    published: bool,
}
pub(super) struct LocalLoop {
    owner: Rc<OriginalNamespaceInfoFrame>,
    each: Rc<Each>,
}

impl LocalLoop {
    pub(super) fn capture(
        actual: &Interp,
        owner: Rc<OriginalNamespaceInfoFrame>,
        argv: &[*mut TclObj],
        variables: &[usize],
    ) -> Result<Self, ValueError> {
        owner.check(actual)?;
        let invocation = actual
            .current_original_source_invocation()?
            .ok_or_else(|| unavailable("original NamespaceInfo foreach Source"))?;
        invocation.check(actual, argv)?;
        if argv.len() != 4
            || variables != [1]
            || !Rc::ptr_eq(&invocation.root, &owner.root)
            || owner
                .local
                .borrow()
                .loops
                .last()
                .is_some_and(|last| !last.finished.get())
        {
            return Err(unavailable("original NamespaceInfo foreach sequence"));
        }
        let body = invocation.operands[3]
            .as_ref()
            .filter(|operand| !operand.returned && operand.source.belongs_to(argv[3]))
            .map(|operand| Rc::clone(&operand.source))
            .ok_or_else(|| unavailable("original NamespaceInfo foreach literal body"))?;
        let each = Rc::new(Each {
            input: owner.data(actual, argv[2])?,
            invocation,
            body,
            current: RefCell::new(None),
            completed: std::cell::Cell::new(0),
            finished: std::cell::Cell::new(false),
        });
        owner.local.borrow_mut().loops.push(Rc::clone(&each));
        Ok(Self { owner, each })
    }
    pub(super) fn check_values(
        &self,
        actual: &Interp,
        original: *mut TclObj,
    ) -> Result<(), ValueError> {
        self.owner.check(actual)?;
        self.each.invocation.check_retained(actual)?;
        self.each.input.check(actual, original)
    }
    pub(super) fn assigned(
        &self,
        actual: &Interp,
        group: usize,
        variable: usize,
        selected: Option<(usize, *mut TclObj)>,
    ) -> Result<(), ValueError> {
        self.owner.check(actual)?;
        self.each.invocation.check_retained(actual)?;
        if group != 0
            || variable != 0
            || self.each.finished.get()
            || self.each.current.borrow().is_some()
        {
            return Err(unavailable(
                "original NamespaceInfo foreach assignment sequence",
            ));
        }
        let (index, original) =
            selected.ok_or_else(|| unavailable("original NamespaceInfo foreach padding"))?;
        if index != self.each.completed.get() || self.each.input.member(actual, index)? != original
        {
            return Err(unavailable("original NamespaceInfo foreach yielded member"));
        }
        *self.each.current.borrow_mut() = Some(LocalIteration {
            index,
            original: NativeObjectLifetime::retain(original),
            ranged: None,
            published: false,
        });
        Ok(())
    }
    pub(super) fn body_completed(&self, actual: &Interp, code: Code) -> Result<(), ValueError> {
        self.owner.check(actual)?;
        self.each.invocation.check_retained(actual)?;
        if code != Code::Ok {
            return Ok(());
        }
        let current = self
            .each
            .current
            .borrow_mut()
            .take()
            .ok_or_else(|| unavailable("original NamespaceInfo foreach entered body"))?;
        if !current.published {
            return Err(unavailable(
                "original NamespaceInfo dictionary publication completion",
            ));
        }
        self.each.completed.set(self.each.completed.get() + 1);
        Ok(())
    }
    pub(super) fn finish(&self, actual: &Interp) -> Result<(), ValueError> {
        self.owner.check(actual)?;
        self.each.invocation.check_retained(actual)?;
        self.each.input.check(actual, self.each.input.original()?)?;
        if self.each.finished.get()
            || self.each.current.borrow().is_some()
            || self.each.completed.get() != self.each.input.len()
        {
            return Err(unavailable(
                "original NamespaceInfo complete foreach coverage",
            ));
        }
        self.each.finished.set(true);
        Ok(())
    }
}

pub(crate) struct OriginalNamespaceInfoDictionaryPath {
    owner: Rc<OriginalNamespaceInfoFrame>,
    each: Rc<Each>,
    invocation: Rc<OriginalSourceInvocation>,
    key: NativeObjectLifetime,
    value: NativeObjectLifetime,
    before: RefCell<Option<OriginalDictionaryMutation>>,
    previous: RefCell<Vec<(*mut TclObj, *mut TclObj)>>,
    previous_holds: RefCell<Vec<(NativeObjectLifetime, NativeObjectLifetime)>>,
}

impl OriginalNamespaceInfoDictionaryPath {
    pub(crate) fn check(&self, actual: &Interp) -> Result<(), ValueError> {
        self.owner.check(actual)?;
        self.invocation.check_retained(actual)?;
        if !self
            .owner
            .local
            .borrow()
            .loops
            .last()
            .is_some_and(|each| Rc::ptr_eq(each, &self.each))
            || self.each.finished.get()
            || !self
                .each
                .current
                .borrow()
                .as_ref()
                .is_some_and(|current| !current.published)
        {
            return Err(unavailable(
                "original NamespaceInfo current dictionary iteration",
            ));
        }
        self.key.checked_ptr()?;
        self.value.checked_ptr()?;
        Ok(())
    }
    pub(crate) fn selected(
        &self,
        actual: &Interp,
        root: &OriginalJimDictionaryRoot,
    ) -> Result<(), ValueError> {
        self.check(actual)?;
        let cell = root
            .direct_cell()
            .ok_or_else(|| unavailable("original NamespaceInfo direct dictionary cell"))?;
        cell.check()?;
        if let Some(previous) = &self.owner.local.borrow().dictionary {
            previous.cell.check()?;
            if previous.original.checked_ptr()? != root.original() {
                return Err(unavailable(
                    "original NamespaceInfo dictionary original header",
                ));
            }
            if !previous.cell.same_place(&cell) {
                return Err(unavailable(
                    "original NamespaceInfo dictionary cell identity",
                ));
            }
            previous.mutation.check(root.original())?;
            crate::dict::original_dictionary_pairs_current(
                root.original(),
                &previous.backing,
                &previous.pairs,
            )?;
        }
        Ok(())
    }
    pub(crate) fn prepared(
        &self,
        actual: &Interp,
        root: &OriginalJimDictionaryRoot,
        prepared: *mut TclObj,
    ) -> Result<(), ValueError> {
        self.selected(actual, root)?;
        let pairs = crate::dict::native_dict_pairs(
            prepared,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )?;
        let state = self.owner.local.borrow();
        if let Some(previous) = &state.dictionary {
            if pairs != previous.pairs {
                return Err(unavailable(
                    "original NamespaceInfo dictionary COW correspondence",
                ));
            }
        } else if !pairs.is_empty() {
            return Err(unavailable(
                "original NamespaceInfo genuine empty dictionary baseline",
            ));
        }
        *self.previous_holds.borrow_mut() = hold_pairs(&pairs)?;
        *self.previous.borrow_mut() = pairs;
        *self.before.borrow_mut() = Some(OriginalDictionaryMutation::capture(prepared)?);
        Ok(())
    }
    pub(crate) fn published(
        &self,
        actual: &Interp,
        root: &OriginalJimDictionaryRoot,
        prepared: *mut TclObj,
        write: &OriginalDictionarySet,
    ) -> Result<(), ValueError> {
        self.check(actual)?;
        let cell = root
            .direct_cell()
            .ok_or_else(|| unavailable("original NamespaceInfo published dictionary cell"))?;
        cell.check()?;
        if cell.root() != prepared {
            return Err(unavailable(
                "original NamespaceInfo selected dictionary publication",
            ));
        }
        let before = self
            .before
            .borrow_mut()
            .take()
            .ok_or_else(|| unavailable("original NamespaceInfo prepared dictionary write"))?;
        let mutation = before.completed_mutation(prepared)?;
        let (ordinal, inserted, key, value) = write.checked_parts(prepared)?;
        if value != self.value.checked_ptr()? {
            return Err(unavailable("original NamespaceInfo written value header"));
        }
        let pairs = crate::dict::native_dict_pairs(
            prepared,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )?;
        let previous = self.previous.borrow();
        if pairs.len() != previous.len() + usize::from(inserted)
            || ordinal > previous.len()
            || (inserted && (ordinal != previous.len() || key != self.key.checked_ptr()?))
            || pairs
                .iter()
                .enumerate()
                .any(|(index, pair)| index != ordinal && previous.get(index) != Some(pair))
        {
            return Err(unavailable(
                "original NamespaceInfo actual dictionary insertion correspondence",
            ));
        }
        let mut iteration = self.each.current.borrow_mut();
        let current = iteration
            .as_mut()
            .ok_or_else(|| unavailable("original NamespaceInfo dictionary yielded member"))?;
        let incoming = if let Some(ranged) = current.ranged.take() {
            ranged
        } else {
            let token = self
                .invocation
                .operands
                .get(3)
                .and_then(Option::as_ref)
                .ok_or_else(|| unavailable("original NamespaceInfo direct key Source"))?;
            if token.source.kind()
                != Some(tcl_syntax::jim_script_objects::JimScriptObjectKind::Source(
                    tcl_lexer::JimScriptTokenKind::Variable,
                ))
                || current.original.checked_ptr()? != self.key.checked_ptr()?
            {
                return Err(unavailable(
                    "original NamespaceInfo native range completion",
                ));
            }
            self.each.input.retained_member(actual, current.index)?
        };
        if incoming.original()? != self.key.checked_ptr()? {
            return Err(unavailable("original NamespaceInfo projected key header"));
        }
        let mut state = self.owner.local.borrow_mut();
        let mut members = state
            .dictionary
            .take()
            .map_or_else(Vec::new, |dictionary| dictionary.members);
        if inserted {
            members.push(incoming);
        } else if members
            .get(ordinal)
            .ok_or_else(|| unavailable("original NamespaceInfo retained first dictionary key"))?
            .original()?
            != key
        {
            return Err(unavailable(
                "original NamespaceInfo first dictionary key identity",
            ));
        }
        let backing = crate::dict::original_dictionary_backing(prepared)?;
        state.dictionary = Some(Dictionary {
            cell,
            original: NativeObjectLifetime::retain(prepared),
            mutation,
            backing,
            _pairs: hold_pairs(&pairs)?,
            pairs,
            members,
        });
        current.published = true;
        Ok(())
    }
}

impl Interp {
    pub(crate) fn original_namespace_info_dictionary_path(
        &self,
        name: *mut TclObj,
        keys: &[*mut TclObj],
        value: Option<*mut TclObj>,
        arguments: &super::super::native_arguments::OriginalArguments,
    ) -> Result<Option<Rc<OriginalNamespaceInfoDictionaryPath>>, ValueError> {
        let Some(owner) = self
            .frames
            .borrow()
            .current_original_namespace_info_frame()?
        else {
            return Ok(None);
        };
        owner.check(self)?;
        let invocation = self
            .current_original_source_invocation()?
            .ok_or_else(|| unavailable("original NamespaceInfo dictionary Source"))?;
        invocation.check_retained(self)?;
        let each = owner
            .local
            .borrow()
            .loops
            .last()
            .cloned()
            .ok_or_else(|| unavailable("original NamespaceInfo dictionary foreach"))?;
        if keys.len() != 1 || value.is_none() || !Rc::ptr_eq(&invocation.root, &each.body) {
            return Err(unavailable(
                "original NamespaceInfo dictionary Source shape",
            ));
        }
        // The actual command worker supplies its retained selector, without reading it.
        let selector = arguments
            .original(1)
            .map_err(|_| unavailable("original NamespaceInfo dictionary selector header"))?;
        invocation.check_suffix(self, &[selector, name, keys[0], value.unwrap()])?;
        let result = Rc::new(OriginalNamespaceInfoDictionaryPath {
            owner,
            each,
            invocation,
            key: NativeObjectLifetime::retain(keys[0]),
            value: NativeObjectLifetime::retain(value.unwrap()),
            before: RefCell::new(None),
            previous: RefCell::new(Vec::new()),
            previous_holds: RefCell::new(Vec::new()),
        });
        result.check(self)?;
        Ok(Some(result))
    }
    pub(crate) fn record_original_namespace_info_range(
        &self,
        source: Option<&super::super::OriginalSourceConstruction>,
        construction: &tcl_cmd_core::string::NativeJimStringRangeConstruction<'_, *mut TclObj>,
    ) -> Result<(), ValueError> {
        let Some(owner) = self
            .frames
            .borrow()
            .current_original_namespace_info_frame()?
        else {
            return Ok(());
        };
        owner.check(self)?;
        let source = source.ok_or_else(|| unavailable("original NamespaceInfo range Source"))?;
        source.check(self)?;
        let invocation = source.invocation();
        let inputs = construction.operands().map(|original| *original);
        invocation.check_suffix(self, &inputs)?;
        let each = owner
            .local
            .borrow()
            .loops
            .last()
            .cloned()
            .ok_or_else(|| unavailable("original NamespaceInfo range foreach"))?;
        if !invocation.root.is_child_of(&each.body) {
            return Err(unavailable("original NamespaceInfo range body Source"));
        }
        let mut iteration = each.current.borrow_mut();
        let current = iteration
            .as_mut()
            .ok_or_else(|| unavailable("original NamespaceInfo range yielded member"))?;
        if current.ranged.is_some()
            || current.published
            || current.original.checked_ptr()? != inputs[0]
        {
            return Err(unavailable("original NamespaceInfo range original input"));
        }
        current.ranged =
            Some(
                each.input
                    .ranged_member(self, current.index, source, construction)?,
            );
        Ok(())
    }
    pub(crate) fn record_original_namespace_info_dictionary_keys(
        &self,
        argv: &[*mut TclObj],
        result: *mut TclObj,
    ) -> Result<(), ValueError> {
        let Some(owner) = self
            .frames
            .borrow()
            .current_original_namespace_info_frame()?
        else {
            return Ok(());
        };
        owner.check(self)?;
        let invocation = self
            .current_original_source_invocation()?
            .ok_or_else(|| unavailable("original NamespaceInfo keys Source"))?;
        invocation.check(self, argv)?;
        let mut local = owner.local.borrow_mut();
        if argv.len() != 3
            || !Rc::ptr_eq(&invocation.root, &owner.root)
            || local.loops.is_empty()
            || local.loops.iter().any(|each| !each.finished.get())
        {
            return Err(unavailable(
                "original NamespaceInfo dictionary foreach completion",
            ));
        }
        let Some(dictionary) = local.dictionary.take() else {
            if local.loops.iter().any(|each| each.input.len() != 0)
                || !crate::dict::native_dict_pairs(
                    argv[2],
                    tcl_syntax::native_string::NativeStringProtocol::Jim084,
                )?
                .is_empty()
            {
                return Err(unavailable(
                    "original NamespaceInfo observed empty dictionary completion",
                ));
            }
            let data = Rc::new(OriginalJimCommandInventoryReturn::capture_members(
                self,
                result,
                Vec::new(),
            )?);
            drop(local);
            owner.record_data(self, Rc::clone(&data))?;
            *owner.completed_body.borrow_mut() = Some(Rc::clone(&data));
            return self.record_original_source_inventory_data(data);
        };
        dictionary.cell.check()?;
        if dictionary.cell.root() != argv[2] || dictionary.original.checked_ptr()? != argv[2] {
            return Err(unavailable(
                "original NamespaceInfo keys original dictionary",
            ));
        }
        dictionary.mutation.check(argv[2])?;
        crate::dict::original_dictionary_pairs_current(
            argv[2],
            &dictionary.backing,
            &dictionary.pairs,
        )?;
        let data = Rc::new(OriginalJimCommandInventoryReturn::capture_members(
            self,
            result,
            dictionary.members,
        )?);
        drop(local);
        owner.record_data(self, Rc::clone(&data))?;
        *owner.completed_body.borrow_mut() = Some(Rc::clone(&data));
        self.record_original_source_inventory_data(data)
    }
}
