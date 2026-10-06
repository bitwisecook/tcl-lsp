// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original TclOO method records retained by table and reached call-chain owners.
use super::*;
use std::ops::Deref;

pub(super) type MethodOwner = Rc<RefCell<Method>>;
#[derive(Default)]
pub(super) struct MethodSlot(Option<MethodOwner>);
impl Clone for MethodSlot {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl MethodSlot {
    pub(super) fn duplicate_declaration(&self) -> Result<Self, tcl_syntax::value::ValueError> {
        let method = self.get();
        method
            .map(|method| {
                method
                    .duplicate_native_payload()
                    .map(|copied| Self(Some(Rc::new(RefCell::new(copied)))))
            })
            .transpose()
            .map(|slot| slot.unwrap_or_default())
    }
    pub(super) fn is_some(&self) -> bool {
        self.0.is_some()
    }
    pub(super) fn take(&mut self) -> Option<MethodOwner> {
        self.0.take()
    }
    pub(super) fn owner(&self) -> Option<MethodOwner> {
        self.0.clone()
    }
    pub(super) fn get(&self) -> Option<Method> {
        self.0.as_ref().map(|owner| owner.borrow().clone())
    }
    pub(super) fn install(&mut self, method: Method) -> Option<MethodOwner> {
        // Constructors and destructors have no name-table entry to replace.
        self.0.replace(Rc::new(RefCell::new(method)))
    }
}

#[derive(Default)]
pub(super) struct MethodTable(BTreeMap<Vec<u8>, MethodOwner>, bool);
impl Clone for MethodTable {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1)
    }
}
impl Deref for MethodTable {
    type Target = BTreeMap<Vec<u8>, MethodOwner>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl MethodTable {
    pub(super) fn duplicate_declarations(&self) -> Result<Self, tcl_syntax::value::ValueError> {
        self.0
            .iter()
            .map(|(name, owner)| {
                let method = owner.borrow().clone();
                Ok((
                    name.clone(),
                    Rc::new(RefCell::new(method.duplicate_native_payload()?)),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map(|table| Self(table, self.1))
    }
    pub(super) fn get(&self, key: &[u8]) -> Option<Method> {
        self.0.get(key).map(|owner| owner.borrow().clone())
    }
    pub(super) fn owner(&self, key: &[u8]) -> Option<MethodOwner> {
        self.0.get(key).cloned()
    }
    pub(super) fn mark_allocated(&mut self) {
        self.1 = true;
    }
    pub(super) fn allocated(&self) -> bool {
        self.1
    }
    pub(super) fn insert(&mut self, key: Vec<u8>, method: Method) -> Option<Method> {
        self.1 = true;
        if let Some(owner) = self.0.get(&key) {
            Some(owner.replace(method))
        } else {
            self.0.insert(key, Rc::new(RefCell::new(method)));
            None
        }
    }
    pub(super) fn remove(&mut self, key: &[u8]) -> Option<MethodOwner> {
        self.0.remove(key)
    }
    pub(super) fn insert_owner(&mut self, key: Vec<u8>, owner: MethodOwner) {
        self.1 = true;
        self.0.insert(key, owner);
    }
}

pub(super) fn retain_owners(state: &OoState, mut chain: Vec<CallStep>) -> Vec<CallStep> {
    for step in &mut chain {
        step.method_owner = if step.method.is_empty() {
            state
                .classes
                .get(&step.provider)
                .and_then(|class| class.constructor.owner())
        } else if step.method == b"<destructor>" {
            state
                .classes
                .get(&step.provider)
                .and_then(|class| class.destructor.owner())
        } else if step.is_object {
            state
                .objects
                .get(&step.provider)
                .and_then(|object| object.methods.owner(&step.method))
        } else {
            state
                .classes
                .get(&step.provider)
                .and_then(|class| class.methods.owner(&step.method))
        };
    }
    chain
}

use std::rc::Weak;
use tcl_registry::native_tcloo_method_cache::{
    NativeTclOoMethodCacheProtocol, NativeTclOoMethodCacheStamp,
};

pub(crate) struct NativeMethodChain {
    origin: Weak<()>,
    stamp: NativeTclOoMethodCacheStamp,
    pub(super) steps: Rc<Vec<CallStep>>,
}
// One native call context owns one chain reference; next frames share that context.
struct NativeMethodContext {
    _chain: Rc<NativeMethodChain>,
}
#[derive(Clone)]
pub(super) struct ReachedMethodChain {
    steps: Rc<Vec<CallStep>>,
    _context: Option<Rc<NativeMethodContext>>,
}
impl ReachedMethodChain {
    fn from_cached(chain: Rc<NativeMethodChain>) -> Self {
        Self {
            steps: Rc::clone(&chain.steps),
            _context: Some(Rc::new(NativeMethodContext { _chain: chain })),
        }
    }
}
impl Deref for ReachedMethodChain {
    type Target = Vec<CallStep>;
    fn deref(&self) -> &Self::Target {
        &self.steps
    }
}
impl From<Vec<CallStep>> for ReachedMethodChain {
    fn from(steps: Vec<CallStep>) -> Self {
        Self {
            steps: Rc::new(steps),
            _context: None,
        }
    }
}
impl From<Rc<Vec<CallStep>>> for ReachedMethodChain {
    fn from(steps: Rc<Vec<CallStep>>) -> Self {
        Self {
            steps,
            _context: None,
        }
    }
}

struct CacheEntry {
    _original: obj::Owned,
    chain: Option<Rc<NativeMethodChain>>,
}
#[derive(Default)]
pub(super) struct NativeMethodWorld {
    origin: Rc<()>,
    foundation: u64,
    epochs: BTreeMap<OoId, u64>,
    class_flags: BTreeMap<OoId, bool>,
    tables: BTreeMap<OoId, BTreeMap<Vec<u8>, CacheEntry>>,
}
impl NativeMethodWorld {
    pub(super) fn instance_table_created(&mut self, target: OoId) {
        self.class_flags.insert(target, false);
    }
    pub(super) fn method_created(&mut self, target: OoId, class: bool) {
        if class {
            self.foundation = self
                .foundation
                .checked_add(1)
                .expect("native OO Foundation epoch exhausted");
        } else {
            self.bump_object(target);
            self.class_flags.insert(target, false);
        }
    }
    pub(super) fn structure_changed(
        &mut self,
        target: OoId,
        class: bool,
        dependents: bool,
        representative_mixins: bool,
    ) {
        if !class {
            self.bump_object(target);
        } else if dependents {
            self.foundation = self
                .foundation
                .checked_add(1)
                .expect("native OO Foundation epoch exhausted");
        } else if representative_mixins {
            self.bump_object(target);
            self.tables.remove(&target);
        }
    }
    fn bump_object(&mut self, target: OoId) {
        let epoch = self.epochs.entry(target).or_default();
        *epoch = epoch
            .checked_add(1)
            .expect("native OO object epoch exhausted");
    }
    pub(super) fn retire(&mut self, target: OoId) {
        self.tables.remove(&target);
        self.epochs.remove(&target);
        self.class_flags.remove(&target);
    }
}
fn selected_stamp(
    state: &OoState,
    target: OoId,
    external: bool,
    protocol: NativeTclOoMethodCacheProtocol,
) -> Option<(OoId, NativeTclOoMethodCacheStamp)> {
    let object = state.objects.get(&target)?;
    let class_cache = state
        .native_methods
        .class_flags
        .get(&target)
        .copied()
        .unwrap_or_else(|| {
            !object.methods.allocated() && object.mixins.is_empty() && object.filters.is_empty()
        });
    let owner = if class_cache { object.class } else { target };
    let receiver = state.objects.get(&owner)?;
    Some((
        owner,
        NativeTclOoMethodCacheStamp {
            creation: receiver.creation_id,
            foundation: state.native_methods.foundation,
            object: state
                .native_methods
                .epochs
                .get(&owner)
                .copied()
                .unwrap_or(0),
            flags: protocol.flags(external, class_cache),
        },
    ))
}
fn valid(
    state: &OoState,
    chain: &NativeMethodChain,
    stamp: NativeTclOoMethodCacheStamp,
    protocol: NativeTclOoMethodCacheProtocol,
) -> bool {
    chain
        .origin
        .ptr_eq(&Rc::downgrade(&state.native_methods.origin))
        && protocol.reusable(chain.stamp, stamp)
}
impl OoState {
    pub(super) fn recompute_native_method_class_cache(&mut self, target: OoId) {
        if let Some(object) = self.objects.get(&target) {
            self.native_methods.class_flags.insert(
                target,
                object.methods.is_empty() && object.mixins.is_empty() && object.filters.is_empty(),
            );
        }
    }
}

impl Interp {
    pub(super) fn lookup_native_method_chain(
        &self,
        target: OoId,
        method: &[u8],
        original: *mut TclObj,
        external: bool,
    ) -> Option<ReachedMethodChain> {
        let protocol = self
            .native_invocation_dialect()
            .native_tcloo_method_cache_protocol()?;
        let state = self.oo.borrow();
        let (owner, stamp) = selected_stamp(&state, target, external, protocol)?;
        if let Some(chain) = obj::native_method_name::chain(original) {
            if valid(&state, &chain, stamp, protocol) {
                return Some(ReachedMethodChain::from_cached(chain));
            }
            obj::native_method_name::clear(original);
        }
        let chain = state
            .native_methods
            .tables
            .get(&owner)?
            .get(method)?
            .chain
            .clone()?;
        if valid(&state, &chain, stamp, protocol) {
            return Some(ReachedMethodChain::from_cached(chain));
        }
        drop(state);
        let retired = self
            .oo
            .borrow_mut()
            .native_methods
            .tables
            .get_mut(&owner)?
            .get_mut(method)?
            .chain
            .take();
        drop(retired);
        None
    }
    pub(super) fn remember_native_method_chain(
        &self,
        target: OoId,
        method: &[u8],
        original: *mut TclObj,
        external: bool,
        steps: Rc<Vec<CallStep>>,
    ) -> Result<ReachedMethodChain, tcl_syntax::value::ValueError> {
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
        else {
            return Ok(steps.into());
        };
        let chain = {
            let state = self.oo.borrow();
            let Some((_, stamp)) = selected_stamp(&state, target, external, protocol) else {
                return Ok(steps.into());
            };
            Rc::new(NativeMethodChain {
                origin: Rc::downgrade(&state.native_methods.origin),
                stamp,
                steps: Rc::clone(&steps),
            })
        };
        obj::native_method_name::install(original, Rc::clone(&chain), protocol.strings())?;
        let mut state = self.oo.borrow_mut();
        let Some((owner, _)) = selected_stamp(&state, target, external, protocol) else {
            return Ok(steps.into());
        };
        let table = state.native_methods.tables.entry(owner).or_default();
        if let Some(entry) = table.get_mut(method) {
            entry.chain = Some(Rc::clone(&chain));
        } else {
            table.insert(
                method.to_vec(),
                CacheEntry {
                    _original: obj::Owned::retain(original),
                    chain: Some(Rc::clone(&chain)),
                },
            );
        }
        Ok(ReachedMethodChain::from_cached(chain))
    }
    pub(super) fn native_method_created(&mut self, target: OoId, class: bool) {
        if self
            .native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
            .is_some()
        {
            self.oo
                .borrow_mut()
                .native_methods
                .method_created(target, class);
        }
    }
    pub(super) fn native_method_structure_changed(&mut self, target: OoId, class: bool) {
        if self
            .native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
            .is_none()
        {
            return;
        }
        let mut state = self.oo.borrow_mut();
        let dependents = state.classes.iter().any(|(id, c)| {
            *id != target && (c.supers.contains(&target) || c.mixins.contains(&target))
        }) || state
            .objects
            .values()
            .any(|o| o.class == target || o.mixins.contains(&target));
        let representative_mixins = state
            .objects
            .get(&target)
            .is_some_and(|o| !o.mixins.is_empty());
        state
            .native_methods
            .structure_changed(target, class, dependents, representative_mixins);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn interp(version: &str) -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap()
    }
    fn source(interp: &mut Interp, text: &[u8]) {
        assert_eq!(
            interp.eval_str(text),
            Code::Ok,
            "{text:?}: {:?}",
            interp.result_bytes()
        );
    }
    fn call(interp: &mut Interp, object: OoId, original: &obj::Owned) {
        let invoked = obj::Owned::fresh(obj::new_string_bytes(b"obj"));
        let words = [invoked.as_ptr(), original.as_ptr()];
        assert_eq!(
            interp.oo_invoke_with_original(object, b"m", &[], true, Some(b"obj"), Some(&words)),
            Code::Ok,
            "{:?}",
            interp.result_bytes()
        );
    }
    #[test]
    fn original_method_primary_and_table_keys_share_the_actual_chain() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interp(version);
            source(
                &mut interp,
                b"oo::class create C {method m {} {return FIRST}}",
            );
            source(&mut interp, b"C create obj");
            let object = interp.oo_resolve_object(b"obj");
            let original = obj::Owned::fresh(obj::new_string_bytes(b"m"));
            call(&mut interp, object, &original);
            assert!(obj::native_method_name::is_cached(original.as_ptr()));
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 2);
            let chain = obj::native_method_name::chain(original.as_ptr()).unwrap();
            let weak = Rc::downgrade(&chain);
            drop(chain);
            assert_eq!(weak.strong_count(), 2);
            let reached = interp
                .lookup_native_method_chain(object, b"m", original.as_ptr(), true)
                .unwrap();
            assert_eq!(weak.strong_count(), 3);
            let next_frame = reached.clone();
            assert_eq!(weak.strong_count(), 3);
            drop(reached);
            drop(next_frame);
            assert_eq!(weak.strong_count(), 2);
            let equal = obj::Owned::fresh(obj::new_string_bytes(b"m"));
            call(&mut interp, object, &equal);
            assert!(!obj::native_method_name::is_cached(equal.as_ptr()));
            assert_eq!(unsafe { (*equal.as_ptr()).ref_count }, 1);
            let duplicate = obj::Owned::fresh(obj::duplicate(original.as_ptr()));
            assert!(Rc::ptr_eq(
                &obj::native_method_name::chain(original.as_ptr()).unwrap(),
                &obj::native_method_name::chain(duplicate.as_ptr()).unwrap()
            ));
            assert_eq!(weak.strong_count(), 3);
            source(&mut interp, b"rename obj renamed");
            call(&mut interp, object, &original);
            assert!(Rc::ptr_eq(
                &weak.upgrade().unwrap(),
                &obj::native_method_name::chain(original.as_ptr()).unwrap()
            ));
            source(&mut interp, b"oo::define C method m {} {return SECOND}");
            call(&mut interp, object, &original);
            assert_eq!(interp.result_bytes(), b"SECOND");
            assert!(!Rc::ptr_eq(
                &weak.upgrade().unwrap(),
                &obj::native_method_name::chain(original.as_ptr()).unwrap()
            ));
            source(
                &mut interp,
                b"oo::objdefine renamed method m {} {return LOCAL}",
            );
            call(&mut interp, object, &original);
            assert_eq!(
                obj::native_method_name::chain(original.as_ptr())
                    .unwrap()
                    .stamp
                    .flags,
                1
            );
            source(&mut interp, b"oo::objdefine renamed deletemethod m");
            call(&mut interp, object, &original);
            assert_eq!(interp.result_bytes(), b"SECOND");
            assert_eq!(
                obj::native_method_name::chain(original.as_ptr())
                    .unwrap()
                    .stamp
                    .flags,
                1
            );
            source(&mut interp, b"oo::objdefine renamed mixin -clear");
            call(&mut interp, object, &original);
            assert!(obj::native_method_name::chain(original.as_ptr()).is_none());
            let state = interp.oo.borrow();
            let class = state.objects.get(&object).unwrap().class;
            let chain = state.native_methods.tables[&class][b"m".as_slice()]
                .chain
                .as_ref()
                .unwrap();
            assert_eq!(chain.stamp.flags, 0x4001);
        }
    }
    #[test]
    fn reached_method_records_distinguish_replacement_from_recreation() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for (mutation, expected) in [
                ("oo::define B deletemethod m", b"BASE".as_slice()),
                (
                    "oo::define B method m {} {return REPLACED}",
                    b"REPLACED".as_slice(),
                ),
                (
                    "oo::define B deletemethod m; oo::define B method m {} {return NEW_RECORD}",
                    b"BASE".as_slice(),
                ),
            ] {
                let mut interp = interp(version);
                source(
                    &mut interp,
                    b"oo::class create B {method m {} {return BASE}}",
                );
                source(
                    &mut interp,
                    format!(
                        "oo::class create C {{superclass B; method m {{}} {{{mutation}; next}}}}"
                    )
                    .as_bytes(),
                );
                source(&mut interp, b"C create obj");
                let object = interp.oo_resolve_object(b"obj");
                call(
                    &mut interp,
                    object,
                    &obj::Owned::fresh(obj::new_string_bytes(b"m")),
                );
                assert_eq!(interp.result_bytes(), expected);
            }
        }
    }

    thread_local! {
        static ENTERED_PROCEDURE_QUERY: RefCell<Option<Rc<crate::interp::ProcDef>>> = const { RefCell::new(None) };
    }

    fn observe_replaced_procedure(interp: &mut Interp, _: OoId, _: &[*mut TclObj]) -> Code {
        use tcl_runtime_api::native_procedure_roles::NativeProcedureRoleOwner;
        ENTERED_PROCEDURE_QUERY.with(|query| {
            let query = query.borrow();
            let procedure = query.as_ref().unwrap();
            // The old ProcedureMethod remains entered after replacement; its
            // one clientData role and the proc core's execution role both live.
            assert_eq!(procedure.native_procedure_role_ledger().references(), 2);
            assert!(procedure.body.checked_ptr().is_ok());
        });
        interp.set_result_bytes(b"OBSERVED");
        Code::Ok
    }

    #[test]
    fn replacement_retires_old_client_data_after_the_actual_entered_method() {
        use tcl_runtime_api::native_procedure_roles::NativeProcedureRoleOwner;
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interp(version);
            source(&mut interp, b"oo::class create C {method m {} {oo::define C method m {} {return NEW}; list OLD [my observe] [my m]}}");
            let class = interp.oo_resolve_object(b"C");
            let query = match interp.oo.borrow().classes[&class]
                .methods
                .get(b"m")
                .unwrap()
            {
                Method::Body { procedure } => procedure,
                _ => panic!("actual authored procedure method"),
            };
            let declaration = query.declaration();
            assert_eq!(declaration.native_procedure_role_ledger().references(), 1);
            let retired = interp
                .oo
                .borrow_mut()
                .classes
                .get_mut(&class)
                .unwrap()
                .methods
                .insert(
                    b"observe".to_vec(),
                    Method::Builtin(observe_replaced_procedure),
                );
            drop(retired);
            ENTERED_PROCEDURE_QUERY.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&declaration)));
            source(&mut interp, b"C create obj");
            let object = interp.oo_resolve_object(b"obj");
            let name = obj::Owned::fresh(obj::new_string_bytes(b"m"));
            call(&mut interp, object, &name);
            assert_eq!(interp.result_bytes(), b"OLD OBSERVED NEW", "{version}");
            assert!(declaration.native_procedure_role_ledger().is_retired());
            assert!(declaration.body.checked_ptr().is_err());
            assert!(query.enter_method().is_err());
            call(&mut interp, object, &name);
            assert_eq!(interp.result_bytes(), b"NEW", "{version}");
            drop(ENTERED_PROCEDURE_QUERY.with(|slot| slot.borrow_mut().take()));
        }
    }

    #[test]
    fn compiled_private_name_and_self_helper_omit_unused_public_head_literals() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interp(version);
            source(
                &mut interp,
                b"proc named {} {set held info; info locals; set held}",
            );
            source(&mut interp, b"named");
            assert_eq!(interp.result_bytes(), b"info");
            assert!(
                obj::obj_type_ptr(interp.result_obj()).is_null(),
                "{version}: omitted public info head must not prime the shared data literal"
            );
            let declaration = interp.proc_def(b"named").unwrap();
            assert!(crate::interp::native_body_artifact::cache_snapshot(
                declaration.body.checked_ptr().unwrap()
            )
            .is_some());

            source(
                &mut interp,
                b"oo::class create C {method m {} {set held self; self namespace; set held}}",
            );
            source(&mut interp, b"C create obj");
            let object = interp.oo_resolve_object(b"obj");
            call(
                &mut interp,
                object,
                &obj::Owned::fresh(obj::new_string_bytes(b"m")),
            );
            assert_eq!(interp.result_bytes(), b"self");
            assert!(
                obj::obj_type_ptr(interp.result_obj()).is_null(),
                "{version}: SELF emits no public self head literal"
            );
            let class = interp.oo_resolve_object(b"C");
            let declaration = match interp.oo.borrow().classes[&class]
                .methods
                .get(b"m")
                .unwrap()
            {
                Method::Body { procedure } => procedure.declaration(),
                _ => panic!("actual authored procedure method"),
            };
            assert!(crate::interp::native_body_artifact::cache_snapshot(
                declaration.body.checked_ptr().unwrap()
            )
            .is_some());
        }
    }
}
