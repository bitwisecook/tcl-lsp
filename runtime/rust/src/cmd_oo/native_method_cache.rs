// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original TclOO method records retained by table and reached call-chain owners.
use super::*;
use std::ops::Deref;
use tcl_registry::native_tcloo_method_definition::NativeTclooMethodReportRole;

/// A Method record retains its current native name independently of the table key.
pub(super) struct NativeMethodRecord {
    payload: RefCell<Method>,
    name: RefCell<Option<obj::Owned>>,
    private: std::cell::Cell<bool>,
}
impl NativeMethodRecord {
    fn unnamed(method: Method) -> Self {
        Self {
            private: std::cell::Cell::new(false),
            payload: RefCell::new(method),
            name: RefCell::new(None),
        }
    }
    fn named(original: *mut TclObj, method: Method) -> Self {
        Self {
            private: std::cell::Cell::new(false),
            payload: RefCell::new(method),
            name: RefCell::new(Some(obj::Owned::retain(original))),
        }
    }
    pub(super) fn set_private(&self, private: bool) {
        self.private.set(private);
    }
    pub(super) fn is_private(&self) -> bool {
        self.private.get()
    }
    pub(super) fn original_name(&self) -> Option<*mut TclObj> {
        self.name.borrow().as_ref().map(obj::Owned::as_ptr)
    }
}
impl Deref for NativeMethodRecord {
    type Target = RefCell<Method>;
    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}
pub(super) type MethodOwner = Rc<NativeMethodRecord>;
#[derive(Default)]
pub(super) struct MethodSlot(Option<MethodOwner>);
impl Clone for MethodSlot {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl MethodSlot {
    pub(super) fn duplicate_declaration(&self) -> Result<Self, tcl_syntax::value::ValueError> {
        self.get()
            .map(|method| {
                method
                    .duplicate_native_payload()
                    .map(|copied| Self(Some(Rc::new(NativeMethodRecord::unnamed(copied)))))
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
        self.0.replace(Rc::new(NativeMethodRecord::unnamed(method)))
    }
}

/// Declaration deletion releases the Method role before its independent hash key.
pub(super) struct RetiredMethodEntry {
    _record: MethodOwner,
    _key: Rc<obj::Owned>,
}
/// Rename publishes the destination before releasing the old hash key outside OO borrows.
pub(super) struct RetiredMethodName {
    _record_name: obj::Owned,
    _table_key: Rc<obj::Owned>,
}
#[derive(Default)]
pub(super) struct MethodTable {
    records: BTreeMap<Vec<u8>, MethodOwner>,
    allocated: bool,
    keys: BTreeMap<Vec<u8>, Rc<obj::Owned>>,
}
impl Clone for MethodTable {
    fn clone(&self) -> Self {
        // A Rust observation shares the existing native key role; it is not a new declaration.
        Self {
            records: self.records.clone(),
            allocated: self.allocated,
            keys: self.keys.clone(),
        }
    }
}
impl Deref for MethodTable {
    type Target = BTreeMap<Vec<u8>, MethodOwner>;
    fn deref(&self) -> &Self::Target {
        &self.records
    }
}
impl MethodTable {
    pub(super) fn duplicate_declarations(&self) -> Result<Self, tcl_syntax::value::ValueError> {
        let mut copied = Self {
            allocated: self.allocated,
            ..Self::default()
        };
        for (key, owner) in &self.records {
            let observed = owner.borrow().clone();
            let method = observed.duplicate_native_payload()?;
            let original_name = owner
                .original_name()
                .expect("named table record owns its original name");
            let original_key = self.keys[key].as_ptr();
            let record = Rc::new(NativeMethodRecord::named(original_name, method));
            record.set_private(owner.is_private());
            copied.records.insert(key.clone(), record);
            copied
                .keys
                .insert(key.clone(), Rc::new(obj::Owned::retain(original_key)));
        }
        Ok(copied)
    }
    pub(super) fn get(&self, key: &[u8]) -> Option<Method> {
        self.records.get(key).map(|owner| owner.borrow().clone())
    }
    pub(super) fn owner(&self, key: &[u8]) -> Option<MethodOwner> {
        self.records.get(key).cloned()
    }
    pub(super) fn mark_allocated(&mut self) {
        self.allocated = true;
    }
    pub(super) fn allocated(&self) -> bool {
        self.allocated
    }
    pub(super) fn insert_generated(&mut self, key: Vec<u8>, method: Method) -> Option<Method> {
        let original = obj::Owned::fresh(obj::new_string_bytes(&key));
        self.insert_original(key, original.as_ptr(), method)
    }
    pub(super) fn insert_original(
        &mut self,
        key: Vec<u8>,
        original: *mut TclObj,
        method: Method,
    ) -> Option<Method> {
        self.allocated = true;
        if let Some(owner) = self.records.get(&key) {
            // Tcl_NewMethod keeps the original name/key but resets current flags.
            owner.set_private(false);
            Some(owner.replace(method))
        } else {
            self.keys
                .insert(key.clone(), Rc::new(obj::Owned::retain(original)));
            self.records
                .insert(key, Rc::new(NativeMethodRecord::named(original, method)));
            None
        }
    }
    pub(super) fn remove(&mut self, key: &[u8]) -> Option<RetiredMethodEntry> {
        let record = self.records.remove(key)?;
        let key = self
            .keys
            .remove(key)
            .expect("named Method entry owns its native table key");
        Some(RetiredMethodEntry {
            _record: record,
            _key: key,
        })
    }
    pub(super) fn is_original_entry(&self, key: &[u8], original: &MethodOwner) -> bool {
        self.records
            .get(key)
            .is_some_and(|owner| Rc::ptr_eq(owner, original))
    }
    pub(super) fn rename_original(
        &mut self,
        from: &[u8],
        to: Vec<u8>,
        original: &MethodOwner,
        destination: *mut TclObj,
    ) -> Option<RetiredMethodName> {
        if !self.is_original_entry(from, original) || self.records.contains_key(&to) {
            return None;
        }
        let new_key = Rc::new(obj::Owned::retain(destination));
        let new_name = obj::Owned::retain(destination);
        let old_name = original
            .name
            .replace(Some(new_name))
            .expect("named source retains its native Method name");
        let owner = self
            .records
            .remove(from)
            .expect("exact original source entry");
        let old_key = self
            .keys
            .remove(from)
            .expect("exact source native table key");
        self.records.insert(to.clone(), owner);
        self.keys.insert(to, new_key);
        Some(RetiredMethodName {
            _record_name: old_name,
            _table_key: old_key,
        })
    }
}

pub(super) fn retain_owners(
    state: &OoState,
    mut chain: Vec<CallStep>,
    role: NativeTclooMethodReportRole,
) -> Vec<CallStep> {
    for step in &mut chain {
        if !step.is_object && step.declaring_facet.is_none() {
            step.declaring_facet = state
                .classes
                .get(&step.provider)
                .map(|class| Rc::downgrade(&class.facet_birth));
        }
        if step.method_owner.is_some() || step.synthetic_type.is_some() {
            continue;
        }
        step.method_owner = if role == NativeTclooMethodReportRole::Constructor {
            state
                .classes
                .get(&step.provider)
                .and_then(|class| class.constructor.owner())
        } else if role == NativeTclooMethodReportRole::Destructor {
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
    steps: Vec<CachedCallStep>,
}
// Idle caches observe the exact record, without retaining its clientData.
struct CachedCallStep {
    method_owner: Weak<NativeMethodRecord>,
    provider: OoId,
    method: Vec<u8>,
    is_object: bool,
    is_filter: bool,
    declaring_facet: Option<Weak<()>>,
}
impl CachedCallStep {
    fn capture(step: &CallStep) -> Option<Self> {
        Some(Self {
            method_owner: Rc::downgrade(step.method_owner.as_ref()?),
            provider: step.provider,
            method: step.method.clone(),
            is_object: step.is_object,
            is_filter: step.is_filter,
            declaring_facet: step.declaring_facet.clone(),
        })
    }
    fn enter(&self) -> Option<CallStep> {
        Some(CallStep {
            method_owner: Some(self.method_owner.upgrade()?),
            provider: self.provider,
            method: self.method.clone(),
            is_object: self.is_object,
            is_filter: self.is_filter,
            synthetic_type: None,
            declaring_facet: self.declaring_facet.clone(),
        })
    }
}
// One native call context owns one chain reference; next frames share that context.
struct NativeMethodContext {
    _chain: Rc<NativeMethodChain>,
}
#[derive(Clone)]
pub(super) struct ReachedMethodChain {
    steps: Rc<Vec<CallStep>>,
    _context: Option<Rc<NativeMethodContext>>,
    report_role: NativeTclooMethodReportRole,
    unknown: bool,
    terminal_report: Option<CallStep>,
}
impl ReachedMethodChain {
    pub(super) fn observed_steps(&self) -> std::rc::Weak<Vec<CallStep>> {
        Rc::downgrade(&self.steps)
    }
    pub(super) fn with_terminal_report(mut self, terminal: CallStep) -> Self {
        self.terminal_report = Some(terminal);
        self
    }
    pub(super) fn terminal_report(&self) -> Option<&CallStep> {
        self.terminal_report.as_ref()
    }
    pub(super) fn with_unknown(mut self, unknown: bool) -> Self {
        self.unknown = unknown;
        self
    }
    pub(super) fn is_unknown(&self) -> bool {
        self.unknown
    }
    pub(super) fn with_report_role(mut self, role: NativeTclooMethodReportRole) -> Self {
        self.report_role = role;
        self
    }
    pub(super) fn report_role(&self) -> NativeTclooMethodReportRole {
        self.report_role
    }
    fn from_cached(chain: Rc<NativeMethodChain>) -> Option<Self> {
        // TclOOInvokeContext retains EVERY original Method in an entered chain.
        // A missing original weak record cannot be replaced by its display name.
        let steps = chain
            .steps
            .iter()
            .map(CachedCallStep::enter)
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            steps: Rc::new(steps),
            _context: Some(Rc::new(NativeMethodContext { _chain: chain })),
            report_role: NativeTclooMethodReportRole::Named,
            unknown: false,
            terminal_report: None,
        })
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
            report_role: NativeTclooMethodReportRole::Named,
            unknown: false,
            terminal_report: None,
        }
    }
}
impl From<Rc<Vec<CallStep>>> for ReachedMethodChain {
    fn from(steps: Rc<Vec<CallStep>>) -> Self {
        Self {
            steps,
            _context: None,
            report_role: NativeTclooMethodReportRole::Named,
            unknown: false,
            terminal_report: None,
        }
    }
}

pub(super) struct CacheEntry {
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
    /// Existing mutation facts only; this observation issues no method-cache purpose.
    pub(super) fn mutation_stamp(&self, target: OoId) -> (Rc<()>, u64, u64) {
        (
            Rc::clone(&self.origin),
            self.foundation,
            self.epochs.get(&target).copied().unwrap_or(0),
        )
    }
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
    ) -> Option<BTreeMap<Vec<u8>, CacheEntry>> {
        if !class {
            self.bump_object(target);
        } else if dependents {
            self.foundation = self
                .foundation
                .checked_add(1)
                .expect("native OO Foundation epoch exhausted");
        } else if representative_mixins {
            self.bump_object(target);
            return self.tables.remove(&target);
        }
        None
    }
    fn bump_object(&mut self, target: OoId) {
        let epoch = self.epochs.entry(target).or_default();
        *epoch = epoch
            .checked_add(1)
            .expect("native OO object epoch exhausted");
    }
    pub(super) fn retire(&mut self, target: OoId) -> Option<BTreeMap<Vec<u8>, CacheEntry>> {
        let retired = self.tables.remove(&target);
        self.epochs.remove(&target);
        self.class_flags.remove(&target);
        retired
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

struct OriginalCacheWorld {
    target: native_context::OriginalOoTarget,
    origin: Rc<()>,
    table_owner: OoId,
    stamp: NativeTclOoMethodCacheStamp,
}
impl OriginalCacheWorld {
    fn capture(
        interpreter: &Interp,
        target: OoId,
        external: bool,
        protocol: NativeTclOoMethodCacheProtocol,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        let target_owner = native_context::OriginalOoTarget::capture(interpreter, target, false)?;
        let state = interpreter.oo.borrow();
        let (table_owner, stamp) =
            selected_stamp(&state, target, external, protocol).ok_or_else(Self::unavailable)?;
        Ok(Self {
            target: target_owner,
            origin: Rc::clone(&state.native_methods.origin),
            table_owner,
            stamp,
        })
    }
    fn unavailable() -> tcl_cmd_core::CmdError {
        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original TclOO method-cache world",
        )
        .into()
    }
    fn check(
        &self,
        interpreter: &Interp,
        originals: &crate::interp::native_arguments::OriginalArguments,
        target: OoId,
        external: bool,
        protocol: NativeTclOoMethodCacheProtocol,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        originals.check()?;
        self.target.check(interpreter)?;
        let state = interpreter.oo.borrow();
        if !Rc::ptr_eq(&self.origin, &state.native_methods.origin)
            || selected_stamp(&state, target, external, protocol)
                != Some((self.table_owner, self.stamp))
        {
            return Err(Self::unavailable());
        }
        Ok(())
    }
}
impl Interp {
    pub(super) fn lookup_native_method_chain(
        &self,
        target: OoId,
        method: &[u8],
        original: *mut TclObj,
        external: bool,
    ) -> Result<Option<ReachedMethodChain>, tcl_cmd_core::CmdError> {
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
        else {
            return Ok(None);
        };
        let originals =
            crate::interp::native_arguments::OriginalArguments::enter(self, &[original])?;
        let world = OriginalCacheWorld::capture(self, target, external, protocol)?;
        let check = || world.check(self, &originals, target, external, protocol);
        check()?;
        if let Some(chain) = obj::native_method_name::chain(original) {
            let reusable = valid(&self.oo.borrow(), &chain, world.stamp, protocol);
            if reusable {
                let entered = ReachedMethodChain::from_cached(chain)
                    .ok_or_else(OriginalCacheWorld::unavailable)?;
                check()?;
                return Ok(Some(entered));
            }
            // No OO Ref guard crosses the original primary/cache retirement.
            obj::native_method_name::clear(original);
            check()?;
        }
        let chain = self
            .oo
            .borrow()
            .native_methods
            .tables
            .get(&world.table_owner)
            .and_then(|table| table.get(method))
            .and_then(|entry| entry.chain.clone());
        let Some(chain) = chain else { return Ok(None) };
        if valid(&self.oo.borrow(), &chain, world.stamp, protocol) {
            let entered = ReachedMethodChain::from_cached(chain)
                .ok_or_else(OriginalCacheWorld::unavailable)?;
            check()?;
            return Ok(Some(entered));
        }
        let retired = self
            .oo
            .borrow_mut()
            .native_methods
            .tables
            .get_mut(&world.table_owner)
            .and_then(|table| table.get_mut(method))
            .and_then(|entry| entry.chain.take());
        drop(retired);
        check()?;
        Ok(None)
    }
    pub(super) fn remember_native_method_chain(
        &self,
        target: OoId,
        method: &[u8],
        original: *mut TclObj,
        external: bool,
        steps: Rc<Vec<CallStep>>,
    ) -> Result<ReachedMethodChain, tcl_cmd_core::CmdError> {
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
        else {
            return Ok(steps.into());
        };
        let originals =
            crate::interp::native_arguments::OriginalArguments::enter(self, &[original])?;
        let world = OriginalCacheWorld::capture(self, target, external, protocol)?;
        let chain = Rc::new(NativeMethodChain {
            origin: Rc::downgrade(&world.origin),
            stamp: world.stamp,
            steps: steps
                .iter()
                .map(CachedCallStep::capture)
                .collect::<Option<Vec<_>>>()
                .ok_or_else(OriginalCacheWorld::unavailable)?,
        });
        // StashCallChain reads/frees the method-name rep before InvokeContext
        // acquires the ALL-record entry role in the original C TclOO source.
        drop(steps);
        let check = || world.check(self, &originals, target, external, protocol);
        obj::native_method_name::install(original, Rc::clone(&chain), protocol.strings(), &check)?;
        check()?;
        let entered = ReachedMethodChain::from_cached(Rc::clone(&chain))
            .ok_or_else(OriginalCacheWorld::unavailable)?;
        let retired = {
            let mut state = self.oo.borrow_mut();
            let table = state
                .native_methods
                .tables
                .entry(world.table_owner)
                .or_default();
            if let Some(entry) = table.get_mut(method) {
                entry.chain.replace(chain)
            } else {
                table.insert(
                    method.to_vec(),
                    CacheEntry {
                        _original: obj::Owned::retain(original),
                        chain: Some(chain),
                    },
                );
                None
            }
        };
        drop(retired);
        check()?;
        Ok(entered)
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
        let retired = state.native_methods.structure_changed(
            target,
            class,
            dependents,
            representative_mixins,
        );
        drop(state);
        drop(retired);
        // The existing entered receipt latches first cause before any caller's
        // later property/cache publication. No policy change refreshes it.
        let _ = self.check_entered_native_operation();
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
                .unwrap()
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
                Method::Body { procedure, .. } => procedure,
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
                .insert_generated(
                    b"observe".to_vec(),
                    Method::builtin(
                        observe_replaced_procedure,
                        NativeTclooMethodPayloadType::Procedure,
                    ),
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
                Method::Body { procedure, .. } => procedure.declaration(),
                _ => panic!("actual authored procedure method"),
            };
            assert!(crate::interp::native_body_artifact::cache_snapshot(
                declaration.body.checked_ptr().unwrap()
            )
            .is_some());
        }
    }
}

#[cfg(test)]
mod original_ownership_tests {
    use super::*;
    use std::cell::Cell;
    use tcl_runtime_api::native_procedure_roles::NativeProcedureRoleOwner;

    fn native(version: &str, source: &[u8]) -> (Interp, OoId, obj::Owned) {
        let mut interpreter = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interpreter.eval_str(source), Code::Ok);
        let target = interpreter.oo_resolve_object(b"CacheObject");
        let selector = obj::Owned::fresh(obj::new_string_bytes(b"m"));
        (interpreter, target, selector)
    }
    fn invoke(interpreter: &mut Interp, target: OoId, selector: &obj::Owned) -> Code {
        let head = obj::Owned::fresh(obj::new_string_bytes(b"CacheObject"));
        interpreter.oo_invoke_with_original(
            target,
            b"m",
            &[],
            true,
            Some(b"CacheObject"),
            Some(&[head.as_ptr(), selector.as_ptr()]),
        )
    }
    fn original_record_was_retired(version: &str, case: u8, stage: &str) -> bool {
        let stream = match version {
            "tcl8.6" => include_str!("../../tests/data/native_method_cache_active_call_lifetime_original/506b/providers/8.6.18/execute.stdout"),
            "tcl9.0" => include_str!("../../tests/data/native_method_cache_active_call_lifetime_original/506b/providers/9.0.4/execute.stdout"),
            "tcl9.1" => include_str!("../../tests/data/native_method_cache_active_call_lifetime_original/506b/providers/9.1.0/execute.stdout"),
            _ => panic!("fixed original supported provider"),
        };
        let case_key = case.to_string();
        let row = stream
            .lines()
            .filter(|row| row.starts_with("ROW\t"))
            .map(|row| {
                row.split('\t')
                    .skip(1)
                    .filter_map(|field| field.split_once('='))
                    .collect::<BTreeMap<_, _>>()
            })
            .find(|row| {
                row.get("case") == Some(&case_key.as_str()) && row.get("stage") == Some(&stage)
            })
            .expect("matched original raw record phase");
        assert_eq!(row["status"], "0");
        row["d0_deletes"] == "1"
    }
    fn method(interpreter: &Interp, class: &[u8]) -> MethodOwner {
        let class = interpreter.oo_resolve_object(class);
        interpreter.oo.borrow().classes[&class]
            .methods
            .owner(b"m")
            .unwrap()
    }
    fn procedure(owner: &MethodOwner) -> Rc<crate::interp::ProcDef> {
        match &*owner.borrow() {
            Method::Body { procedure, .. } => procedure.declaration(),
            _ => panic!("actual original procedure method"),
        }
    }

    #[test]
    fn original_idle_method_cache_observes_retirement_without_retaining_payload() {
        // naming.tcloo.original-method-cache-and-active-call-lifetime
        // docs/design/analysis/name-resolution-proofs/tcloo-original-method-cache-and-active-call-lifetime.md
        // Runtime method-record ownership; procedure clientData has its own roles.
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let (mut interpreter, target, selector) = native(version,
                b"oo::class create CacheClass {method m {} {return FIRST}}; CacheClass create CacheObject");
            let original = method(&interpreter, b"CacheClass");
            let declaration = procedure(&original);
            let weak = Rc::downgrade(&original);
            drop(original);
            assert_eq!(invoke(&mut interpreter, target, &selector), Code::Ok);
            let duplicate = obj::Owned::fresh(obj::duplicate(selector.as_ptr()));
            assert!(obj::native_method_name::is_cached(selector.as_ptr()));
            assert!(obj::native_method_name::is_cached(duplicate.as_ptr()));
            assert_eq!(weak.strong_count(), 1, "idle caches are observations");
            assert_eq!(
                interpreter.eval_str(b"oo::define CacheClass deletemethod m"),
                Code::Ok
            );
            assert_eq!(
                weak.upgrade().is_none(),
                original_record_was_retired(version, 0, "after-idle-delete")
            );
            assert!(declaration.native_procedure_role_ledger().is_retired());
            assert!(declaration.body.checked_ptr().is_err());
            assert!(obj::native_method_name::is_cached(selector.as_ptr()));
            assert!(obj::native_method_name::is_cached(duplicate.as_ptr()));
            drop(duplicate);
            drop(selector);
            assert!(declaration.native_procedure_role_ledger().is_retired());
        }
    }

    #[test]
    fn original_entered_chain_retains_every_inherited_record_until_last_next_owner() {
        // naming.tcloo.original-method-cache-and-active-call-lifetime
        // docs/design/analysis/name-resolution-proofs/tcloo-original-method-cache-and-active-call-lifetime.md
        // This tests the validated entered-record role, not an entered proc body.
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let (mut interpreter,target,selector) = native(version,
                b"oo::class create CacheBase {method m {} {return BASE}}; oo::class create CacheClass {superclass CacheBase; method m {} {next}}; CacheClass create CacheObject");
            let base = method(&interpreter, b"CacheBase");
            let derived = method(&interpreter, b"CacheClass");
            let original_base = Rc::downgrade(&base);
            let original_derived = Rc::downgrade(&derived);
            drop(base);
            drop(derived);
            assert_eq!(invoke(&mut interpreter, target, &selector), Code::Ok);
            assert_eq!(interpreter.result_bytes(), b"BASE");
            let entered = interpreter
                .lookup_native_method_chain(target, b"m", selector.as_ptr(), true)
                .unwrap()
                .unwrap();
            assert_eq!(entered.len(), 2);
            let next_owner = entered.clone();
            assert_eq!(
                interpreter.eval_str(
                    b"oo::define CacheBase deletemethod m; oo::define CacheClass deletemethod m"
                ),
                Code::Ok
            );
            assert_eq!(
                original_base.upgrade().is_none(),
                original_record_was_retired(version, 2, "derived-after-delete")
            );
            assert!(original_derived.upgrade().is_some());
            assert!(Rc::ptr_eq(
                entered[0].method_owner.as_ref().unwrap(),
                &original_derived.upgrade().unwrap()
            ));
            assert!(Rc::ptr_eq(
                entered[1].method_owner.as_ref().unwrap(),
                &original_base.upgrade().unwrap()
            ));
            drop(entered);
            assert!(original_base.upgrade().is_some());
            drop(next_owner);
            assert_eq!(
                original_base.upgrade().is_none(),
                original_record_was_retired(version, 2, "after-active-next-invocation")
            );
            assert!(original_derived.upgrade().is_none());
        }
    }

    #[test]
    fn original_same_record_replacement_retires_unentered_old_payload_immediately() {
        // naming.tcloo.original-method-cache-and-active-call-lifetime
        // docs/design/analysis/name-resolution-proofs/tcloo-original-method-cache-and-active-call-lifetime.md
        // Strong record ownership is distinct from a proc's real entered-body role.
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let (mut interpreter,target,selector) = native(version,
                b"oo::class create CacheClass {method m {} {return FIRST}}; CacheClass create CacheObject");
            assert_eq!(invoke(&mut interpreter, target, &selector), Code::Ok);
            let entered = interpreter
                .lookup_native_method_chain(target, b"m", selector.as_ptr(), true)
                .unwrap()
                .unwrap();
            let original = Rc::clone(entered[0].method_owner.as_ref().unwrap());
            let old_payload = procedure(&original);
            assert_eq!(
                interpreter.eval_str(b"oo::define CacheClass method m {} {return SECOND}"),
                Code::Ok
            );
            let current = method(&interpreter, b"CacheClass");
            assert!(Rc::ptr_eq(&original, &current));
            assert_eq!(
                old_payload.native_procedure_role_ledger().is_retired(),
                original_record_was_retired(version, 3, "after-idle-replacement")
            );
            assert!(old_payload.body.checked_ptr().is_err());
            assert!(!Rc::ptr_eq(&old_payload, &procedure(&current)));
            assert_eq!(invoke(&mut interpreter, target, &selector), Code::Ok);
            assert_eq!(interpreter.result_bytes(), b"SECOND");
            drop(entered);
        }
    }

    #[test]
    fn original_missing_cached_record_cannot_adopt_a_recreated_name() {
        // naming.tcloo.original-method-cache-and-active-call-lifetime
        // docs/design/analysis/name-resolution-proofs/tcloo-original-method-cache-and-active-call-lifetime.md
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let (mut interpreter,target,selector) = native(version,
                b"oo::class create CacheClass {method m {} {return FIRST}}; CacheClass create CacheObject");
            assert_eq!(invoke(&mut interpreter, target, &selector), Code::Ok);
            let chain = obj::native_method_name::chain(selector.as_ptr()).unwrap();
            let original = Rc::downgrade(&method(&interpreter, b"CacheClass"));
            assert_eq!(interpreter.eval_str(b"oo::define CacheClass deletemethod m; oo::define CacheClass method m {} {return RECREATED}"),Code::Ok);
            assert!(original.upgrade().is_none());
            assert!(ReachedMethodChain::from_cached(chain).is_none());
            assert!(interpreter
                .lookup_native_method_chain(target, b"m", selector.as_ptr(), true)
                .unwrap()
                .is_none());
            assert!(!obj::native_method_name::is_cached(selector.as_ptr()));
            assert_eq!(invoke(&mut interpreter, target, &selector), Code::Ok);
            assert_eq!(interpreter.result_bytes(), b"RECREATED");
        }
    }

    thread_local! {
        static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const {RefCell::new(None)};
        static MODE: Cell<u8> = const {Cell::new(0)};
        static FREES: Cell<usize> = const {Cell::new(0)};
        static OUTSIDE_BORROW: Cell<bool> = const {Cell::new(false)};
        static ORIGINAL_RECORD: RefCell<Option<Weak<NativeMethodRecord>>> = const {RefCell::new(None)};
        static DELETED_BEFORE_ENTRY: Cell<bool> = const {Cell::new(false)};
    }
    extern "C" fn update_selector(original: *mut TclObj) {
        unsafe { obj::set_native_updater_string_rep(original, b"m", false) };
    }
    extern "C" fn retire_selector(_: *mut TclObj) {
        FREES.with(|count| count.set(count.get() + 1));
        let mut interpreter =
            OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
        OUTSIDE_BORROW.with(|outside| outside.set(interpreter.oo.try_borrow_mut().is_ok()));
        match MODE.with(Cell::get) {
            1 => assert_eq!(
                interpreter.eval_str(b"oo::define CacheClass method m {} {return CHANGED}"),
                Code::Ok
            ),
            2 => {
                interpreter.refuse_host_command("first original method cache retirement");
            }
            3 => {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed
                    .packages
                    .push(("original-method-cache".to_owned(), "1.0".to_owned()));
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
            4 => {
                let original =
                    ORIGINAL_RECORD.with(|record| record.borrow().as_ref().unwrap().clone());
                assert_eq!(
                    original.strong_count(),
                    1,
                    "only the declaration precedes entry"
                );
                let declaration = procedure(&original.upgrade().unwrap());
                assert_eq!(
                    interpreter.eval_str(b"oo::define CacheClass deletemethod m"),
                    Code::Ok
                );
                assert!(
                    original.upgrade().is_none(),
                    "cache construction has no entered record role"
                );
                assert!(declaration.native_procedure_role_ledger().is_retired());
                assert!(declaration.body.checked_ptr().is_err());
                DELETED_BEFORE_ENTRY.with(|deleted| deleted.set(true));
            }
            _ => {}
        }
    }
    static SELECTOR: obj::TclObjType = obj::TclObjType {
        name: c"originalMethodCacheSelector".as_ptr(),
        free_int_rep_proc: Some(retire_selector),
        dup_int_rep_proc: None,
        update_string_proc: Some(update_selector),
        set_from_any_proc: None,
    };
    #[test]
    fn original_cache_retirement_callback_cannot_publish_a_fresh_world_stamp() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Genuine software callbacks/actual context ABA, not original C methodType equivalence.
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for mode in [0, 1, 2, 3, 4] {
                let (mut interpreter,target,resident) = native(version,
                    b"oo::class create CacheClass {method m {} {return FIRST}}; CacheClass create CacheObject");
                drop(resident);
                OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
                MODE.with(|selected| selected.set(mode));
                FREES.with(|count| count.set(0));
                OUTSIDE_BORROW.with(|outside| outside.set(false));
                ORIGINAL_RECORD.with(|record| {
                    *record.borrow_mut() = Some(Rc::downgrade(&method(&interpreter, b"CacheClass")))
                });
                DELETED_BEFORE_ENTRY.with(|deleted| deleted.set(false));
                let selector = obj::Owned::fresh(obj::alloc_typed(&SELECTOR, 0));
                assert_eq!(
                    invoke(&mut interpreter, target, &selector),
                    if mode == 0 { Code::Ok } else { Code::Error }
                );
                assert_eq!(FREES.with(Cell::get), 1);
                assert!(OUTSIDE_BORROW.with(Cell::get));
                assert!(
                    obj::has_string_rep(selector.as_ptr()),
                    "reached original String survives"
                );
                assert_eq!(
                    obj::native_method_name::is_cached(selector.as_ptr()),
                    mode == 0
                );
                if mode != 0 {
                    assert_eq!(unsafe { (*selector.as_ptr()).ref_count }, 1);
                    let state = interpreter.oo.borrow();
                    assert!(!state.native_methods.tables.values().any(|table| table
                        .get(b"m".as_slice())
                        .is_some_and(|entry| entry.chain.is_some())));
                }
                if mode == 2 {
                    assert!(interpreter.native_execution_refusal().is_some());
                }
                assert_eq!(DELETED_BEFORE_ENTRY.with(Cell::get), mode == 4);
                drop(ORIGINAL_RECORD.with(|record| record.borrow_mut().take()));
                drop(OWNER.with(|owner| owner.borrow_mut().take()));
            }
        }
    }
}
