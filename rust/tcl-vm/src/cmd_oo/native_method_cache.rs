// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original `TclOO` method records retained by table and reached call-chain owners.
use super::{
    BTreeMap, Code, Completion, Method, OoId, OoState, Rc, RefCell, Step, Value, Vm, err,
    method_report_bytes, native_method_key, ok,
};
use std::ops::Deref;

pub(super) type MethodOwner = Rc<RefCell<Method>>;
#[derive(Default)]
pub(super) struct MethodSlot(Option<MethodOwner>);
impl Clone for MethodSlot {
    fn clone(&self) -> Self {
        Self(
            self.0
                .as_ref()
                .map(|owner| Rc::new(RefCell::new(owner.borrow().duplicate_native_payload()))),
        )
    }
}
impl MethodSlot {
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
    pub(super) fn install(&mut self, method: Method) {
        // Constructors and destructors have no name-table entry to replace.
        self.0 = Some(Rc::new(RefCell::new(method)));
    }
}

#[derive(Default)]
pub(super) struct MethodTable(BTreeMap<String, MethodOwner>, bool);
impl Clone for MethodTable {
    fn clone(&self) -> Self {
        Self(
            self.0
                .iter()
                .map(|(key, owner)| {
                    (
                        key.clone(),
                        Rc::new(RefCell::new(owner.borrow().duplicate_native_payload())),
                    )
                })
                .collect(),
            self.1,
        )
    }
}
impl Deref for MethodTable {
    type Target = BTreeMap<String, MethodOwner>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl MethodTable {
    pub(super) fn get(&self, key: &str) -> Option<Method> {
        self.0.get(key).map(|owner| owner.borrow().clone())
    }
    pub(super) fn owner(&self, key: &str) -> Option<MethodOwner> {
        self.0.get(key).cloned()
    }
    pub(super) fn mark_allocated(&mut self) {
        self.1 = true;
    }
    pub(super) fn allocated(&self) -> bool {
        self.1
    }
    pub(super) fn insert(&mut self, key: String, method: Method) {
        self.1 = true;
        if let Some(owner) = self.0.get(&key) {
            *owner.borrow_mut() = method;
        } else {
            self.0.insert(key, Rc::new(RefCell::new(method)));
        }
    }
    pub(super) fn remove(&mut self, key: &str) -> Option<MethodOwner> {
        self.0.remove(key)
    }
    pub(super) fn insert_owner(&mut self, key: String, owner: MethodOwner) {
        self.1 = true;
        self.0.insert(key, owner);
    }
}

pub(super) fn retain_owners(state: &OoState, mut chain: Vec<Step>, method: &str) -> Vec<Step> {
    for step in &mut chain {
        step.method_owner = if method.is_empty() {
            state
                .classes
                .get(&step.provider)
                .and_then(|class| class.constructor.owner())
        } else if method == "<destructor>" {
            state
                .classes
                .get(&step.provider)
                .and_then(|class| class.destructor.owner())
        } else if step.is_object {
            state
                .objects
                .get(&step.provider)
                .and_then(|object| object.methods.owner(method))
        } else {
            state
                .classes
                .get(&step.provider)
                .and_then(|class| class.methods.owner(method))
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
    pub(super) steps: Rc<Vec<Step>>,
}
// One native call context owns one chain reference; next frames share that context.
struct NativeMethodContext {
    _chain: Rc<NativeMethodChain>,
}
#[derive(Clone)]
pub(super) struct ReachedMethodChain {
    steps: Rc<Vec<Step>>,
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
    type Target = Vec<Step>;
    fn deref(&self) -> &Self::Target {
        &self.steps
    }
}
impl From<Vec<Step>> for ReachedMethodChain {
    fn from(steps: Vec<Step>) -> Self {
        Self {
            steps: Rc::new(steps),
            _context: None,
        }
    }
}
impl From<Rc<Vec<Step>>> for ReachedMethodChain {
    fn from(steps: Rc<Vec<Step>>) -> Self {
        Self {
            steps,
            _context: None,
        }
    }
}

struct CacheEntry {
    _original: Value,
    chain: Option<Rc<NativeMethodChain>>,
}
#[derive(Default)]
pub(super) struct NativeMethodWorld {
    origin: Rc<()>,
    foundation: u64,
    epochs: BTreeMap<OoId, u64>,
    class_flags: BTreeMap<OoId, bool>,
    tables: BTreeMap<OoId, BTreeMap<String, CacheEntry>>,
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
        .unwrap_or_else(|| !object.methods.allocated() && object.mixins.is_empty());
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
                object.methods.is_empty() && object.mixins.is_empty(),
            );
        }
    }
}

pub(super) fn lookup(
    vm: &mut Vm,
    target: OoId,
    method: &str,
    original: &Value,
    external: bool,
) -> Option<ReachedMethodChain> {
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_tcloo_method_cache_protocol()?;
    let (owner, stamp) = selected_stamp(&vm.oo, target, external, protocol)?;
    if let Some(chain) = original.native_method_name_chain() {
        if valid(&vm.oo, &chain, stamp, protocol) {
            return Some(ReachedMethodChain::from_cached(chain));
        }
        original.clear_native_method_name();
    }
    let chain = vm
        .oo
        .native_methods
        .tables
        .get(&owner)?
        .get(method)?
        .chain
        .clone()?;
    if valid(&vm.oo, &chain, stamp, protocol) {
        return Some(ReachedMethodChain::from_cached(chain));
    }
    let retired = vm
        .oo
        .native_methods
        .tables
        .get_mut(&owner)?
        .get_mut(method)?
        .chain
        .take();
    drop(retired);
    None
}
pub(super) fn remember(
    vm: &mut Vm,
    target: OoId,
    method: &str,
    original: &Value,
    external: bool,
    steps: Rc<Vec<Step>>,
) -> Result<ReachedMethodChain, tcl_syntax::value::ValueError> {
    let Some(protocol) = vm
        .actual_native_invocation_dialect()
        .native_tcloo_method_cache_protocol()
    else {
        return Ok(steps.into());
    };
    let Some((owner, stamp)) = selected_stamp(&vm.oo, target, external, protocol) else {
        return Ok(steps.into());
    };
    let chain = Rc::new(NativeMethodChain {
        origin: Rc::downgrade(&vm.oo.native_methods.origin),
        stamp,
        steps: Rc::clone(&steps),
    });
    original.retain_native_method_name(Rc::clone(&chain), protocol.strings())?;
    let table = vm.oo.native_methods.tables.entry(owner).or_default();
    if let Some(entry) = table.get_mut(method) {
        entry.chain = Some(Rc::clone(&chain));
    } else {
        table.insert(
            method.to_owned(),
            CacheEntry {
                _original: original.clone(),
                chain: Some(Rc::clone(&chain)),
            },
        );
    }
    Ok(ReachedMethodChain::from_cached(chain))
}
impl Vm {
    pub(super) fn native_method_created(&mut self, target: OoId, class: bool) {
        if self
            .actual_native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
            .is_some()
        {
            self.oo.native_methods.method_created(target, class);
        }
    }
    pub(super) fn native_method_structure_changed(&mut self, target: OoId, class: bool) {
        if self
            .actual_native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
            .is_none()
        {
            return;
        }
        let dependents = self.oo.classes.iter().any(|(id, c)| {
            *id != target && (c.supers.contains(&target) || c.mixins.contains(&target))
        }) || self
            .oo
            .objects
            .values()
            .any(|o| o.class == target || o.mixins.contains(&target));
        let representative_mixins = self
            .oo
            .objects
            .get(&target)
            .is_some_and(|o| !o.mixins.is_empty());
        self.oo
            .native_methods
            .structure_changed(target, class, dependents, representative_mixins);
    }
}

fn table_mut(state: &mut OoState, target: OoId, class: bool) -> Option<&mut MethodTable> {
    if class {
        state
            .classes
            .get_mut(&target)
            .map(|class| &mut class.methods)
    } else {
        state
            .objects
            .get_mut(&target)
            .map(|object| &mut object.methods)
    }
}
pub(super) fn delete_method(
    vm: &mut Vm,
    class: bool,
    target: OoId,
    args: &[Value],
) -> Completion<Value> {
    if args.is_empty() {
        return crate::command::native_wrong_args(vm, "deletemethod name ?name ...?");
    }
    for original in args {
        let key = match native_method_key(vm, original) {
            Ok(key) => key,
            Err(error) => return error,
        };
        let owner = table_mut(&mut vm.oo, target, class).and_then(|table| table.remove(&key));
        let Some(owner) = owner else {
            let mut message = b"method ".to_vec();
            message.extend_from_slice(&method_report_bytes(vm, &key));
            message.extend_from_slice(b" does not exist");
            return Completion::new(
                Code::Error,
                Value::from_native_string_bytes(message),
                Value::empty(),
            );
        };
        if class {
            if let Some(class) = vm.oo.classes.get_mut(&target) {
                class.exported.remove(&key);
                class.unexported.remove(&key);
            }
        } else if let Some(object) = vm.oo.objects.get_mut(&target) {
            object.exported.remove(&key);
            object.unexported.remove(&key);
        }
        vm.native_property_structure_changed(target, class);
        drop(owner);
    }
    ok(Value::empty())
}
pub(super) fn rename_method(
    vm: &mut Vm,
    class: bool,
    target: OoId,
    args: &[Value],
) -> Completion<Value> {
    let [from, to] = args else {
        return crate::command::native_wrong_args(vm, "renamemethod oldName newName");
    };
    let from = match native_method_key(vm, from) {
        Ok(key) => key,
        Err(error) => return error,
    };
    let to = match native_method_key(vm, to) {
        Ok(key) => key,
        Err(error) => return error,
    };
    if from == to {
        return err("cannot rename method to itself");
    }
    let Some(table) = table_mut(&mut vm.oo, target, class) else {
        return err("attempt to misuse API");
    };
    if table.contains_key(&to) {
        return err("method already exists");
    }
    let Some(owner) = table.remove(&from) else {
        return err("method does not exist");
    };
    table.insert_owner(to.clone(), owner);
    let (exported, unexported) = if class {
        let class = vm.oo.classes.get_mut(&target).expect("retained class");
        (&mut class.exported, &mut class.unexported)
    } else {
        let object = vm.oo.objects.get_mut(&target).expect("retained object");
        (&mut object.exported, &mut object.unexported)
    };
    if exported.remove(&from) {
        exported.insert(to.clone());
    }
    if unexported.remove(&from) {
        unexported.insert(to);
    }
    vm.native_property_structure_changed(target, class);
    ok(Value::empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd_oo::{oo_invoke_value, resolve_object_value};
    fn vm(version: tcl_dialect::TclVersion) -> Vm {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm
    }
    fn source(vm: &mut Vm, text: &str) {
        let completion = vm.eval_source(text).unwrap();
        assert_eq!(
            completion.code,
            Code::Ok,
            "{text}: {}",
            completion.result.to_str()
        );
    }
    fn call(vm: &mut Vm, object: OoId, original: &Value) -> Value {
        let invoked = Value::string("obj");
        let completion = oo_invoke_value(vm, object, "m", original, &[], true, &invoked);
        assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
        completion.result
    }
    #[test]
    fn original_method_primary_and_table_keys_share_the_actual_chain() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            source(&mut vm, "oo::class create C {method m {} {return FIRST}}");
            source(&mut vm, "C create obj");
            let object = resolve_object_value(&mut vm, &Value::string("obj"))
                .unwrap()
                .unwrap();
            let original = Value::string("m");
            assert_eq!(call(&mut vm, object, &original).to_str().as_ref(), "FIRST");
            assert_eq!(original.native_object_type_name(), "TclOO method name");
            assert_eq!(original.native_object_reference_count(), 2);
            let chain = original.native_method_name_chain().unwrap();
            let weak = Rc::downgrade(&chain);
            drop(chain);
            assert_eq!(weak.strong_count(), 2);
            let reached = lookup(&mut vm, object, "m", &original, true).unwrap();
            assert_eq!(weak.strong_count(), 3);
            let next_frame = reached.clone();
            assert_eq!(weak.strong_count(), 3);
            drop(reached);
            drop(next_frame);
            assert_eq!(weak.strong_count(), 2);
            let equal = Value::string("m");
            call(&mut vm, object, &equal);
            assert_eq!(equal.native_object_type_name(), "none");
            assert_eq!(equal.native_object_reference_count(), 1);
            let duplicate = original.duplicate_native_object();
            assert!(Rc::ptr_eq(
                &original.native_method_name_chain().unwrap(),
                &duplicate.native_method_name_chain().unwrap()
            ));
            assert_eq!(weak.strong_count(), 3);
            source(&mut vm, "rename obj renamed");
            call(&mut vm, object, &original);
            assert!(Rc::ptr_eq(
                &weak.upgrade().unwrap(),
                &original.native_method_name_chain().unwrap()
            ));
            source(&mut vm, "oo::define C method m {} {return SECOND}");
            assert_eq!(call(&mut vm, object, &original).to_str().as_ref(), "SECOND");
            assert!(!Rc::ptr_eq(
                &weak.upgrade().unwrap(),
                &original.native_method_name_chain().unwrap()
            ));
            source(&mut vm, "oo::objdefine renamed method m {} {return LOCAL}");
            assert_eq!(call(&mut vm, object, &original).to_str().as_ref(), "LOCAL");
            assert_eq!(original.native_method_name_chain().unwrap().stamp.flags, 1);
            source(&mut vm, "oo::objdefine renamed deletemethod m");
            assert_eq!(call(&mut vm, object, &original).to_str().as_ref(), "SECOND");
            assert_eq!(original.native_method_name_chain().unwrap().stamp.flags, 1);
            source(&mut vm, "oo::objdefine renamed mixin");
            call(&mut vm, object, &original);
            assert!(original.native_method_name_chain().is_none());
            let class = vm.oo.objects.get(&object).unwrap().class;
            let chain = vm.oo.native_methods.tables[&class]["m"]
                .chain
                .as_ref()
                .unwrap();
            assert_eq!(chain.stamp.flags, 0x4001);
        }
    }
    #[test]
    fn reached_method_records_distinguish_replacement_from_recreation() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            for (mutation, expected) in [
                ("oo::define B deletemethod m", "BASE"),
                ("oo::define B method m {} {return REPLACED}", "REPLACED"),
                (
                    "oo::define B deletemethod m; oo::define B method m {} {return NEW_RECORD}",
                    "BASE",
                ),
            ] {
                let mut vm = vm(version);
                source(&mut vm, "oo::class create B {method m {} {return BASE}}");
                source(
                    &mut vm,
                    &format!(
                        "oo::class create C {{superclass B; method m {{}} {{{mutation}; next}}}}"
                    ),
                );
                source(&mut vm, "C create obj");
                let object = resolve_object_value(&mut vm, &Value::string("obj"))
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    call(&mut vm, object, &Value::string("m")).to_str().as_ref(),
                    expected
                );
            }
        }
    }
    #[test]
    fn foreign_cache_and_missing_methods_cannot_supply_an_owner() {
        let mut first = vm(tcl_dialect::TclVersion::V8_6);
        let mut second = vm(tcl_dialect::TclVersion::V8_6);
        for vm in [&mut first, &mut second] {
            source(vm, "oo::class create C {method m {} {return RESULT}}");
            source(vm, "C create obj");
        }
        let object = resolve_object_value(&mut first, &Value::string("obj"))
            .unwrap()
            .unwrap();
        let original = Value::string("m");
        call(&mut first, object, &original);
        let before = original.native_method_name_chain().unwrap();
        let second_object = resolve_object_value(&mut second, &Value::string("obj"))
            .unwrap()
            .unwrap();
        call(&mut second, second_object, &original);
        assert!(!Rc::ptr_eq(
            &before,
            &original.native_method_name_chain().unwrap()
        ));
        let missing = Value::string("absent");
        let result = oo_invoke_value(
            &mut second,
            second_object,
            "absent",
            &missing,
            &[],
            true,
            &Value::string("obj"),
        );
        assert_eq!(result.code, Code::Error);
        assert!(missing.native_method_name_chain().is_none());
    }
}
