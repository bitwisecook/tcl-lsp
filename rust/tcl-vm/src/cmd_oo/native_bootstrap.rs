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

//! Actual bootstrap allocations and passive same-entry `TclOO` observations.
use super::{
    BTreeSet, Class, Command, CommandSidecarKey, Method, MethodParameters, NameBytes, NsId, Object,
    OoId, Rc, RefCell, Value, Vm, native_context, native_method_cache,
};
use tcl_runtime_api::{
    NativeCompilationEntry,
    native_oo::{
        NativeOoBootstrapRole, NativeOoClassInventory, NativeOoClassObservation,
        NativeOoIntrinsicMethod, NativeOoLifecycleObservation, NativeOoMethodObservation,
    },
};

fn intrinsic(role: NativeOoIntrinsicMethod, public: bool) -> Method {
    Method {
        parameters: MethodParameters {
            values: Vec::new(),
            has_args: true,
        },
        compiled_body: None,
        body_src: Value::empty(),
        forward: None,
        property: None,
        intrinsic: Some(role),
        visibility_only: false,
        exported: public,
        private: false,
    }
}

pub(super) fn install(vm: &mut Vm) {
    let object_root = vm.oo.object_root.expect("allocated OO object root");
    let class_root = vm.oo.class_root.expect("allocated OO class root");
    let configurable_root = vm
        .oo
        .configurable_root
        .expect("allocated configurable root");
    let destroy = vm
        .oo
        .intern_method_key(NameBytes::from(b"destroy".as_slice()));
    vm.oo
        .classes
        .get_mut(&object_root)
        .unwrap()
        .methods
        .insert(destroy, intrinsic(NativeOoIntrinsicMethod::Destroy, true));
    let class_ns = vm.definition_namespace_token("::oo::define");
    let object_ns = vm.definition_namespace_token("::oo::objdefine");
    vm.oo
        .classes
        .get_mut(&object_root)
        .unwrap()
        .object_definition_namespace = Some(object_ns);
    let class = vm.oo.classes.get_mut(&class_root).unwrap();
    class.definition_namespace = Some(class_ns);
    class
        .constructor
        .install(intrinsic(NativeOoIntrinsicMethod::ClassConstructor, false));
    for (name, role) in [
        (b"create".as_slice(), NativeOoIntrinsicMethod::Create),
        (b"new".as_slice(), NativeOoIntrinsicMethod::New),
    ] {
        let name = vm.oo.intern_method_key(NameBytes::from(name));
        vm.oo
            .classes
            .get_mut(&class_root)
            .unwrap()
            .methods
            .insert(name, intrinsic(role, true));
    }
    let new = vm.oo.intern_method_key(NameBytes::from(b"new".as_slice()));
    let mut visibility = intrinsic(NativeOoIntrinsicMethod::New, false);
    visibility.intrinsic = None;
    visibility.visibility_only = true;
    let class_object = vm.oo.objects.get_mut(&class_root).unwrap();
    class_object.methods.insert(new.clone(), visibility);
    class_object.unexported.insert(new);
    install_configurable_support(
        vm,
        object_root,
        class_root,
        configurable_root,
        class_ns,
        object_ns,
    );
}

fn install_configurable_support(
    vm: &mut Vm,
    object_root: OoId,
    class_root: OoId,
    configurable_root: OoId,
    class_ns: NsId,
    object_ns: NsId,
) {
    let cfg_class_ns = vm.definition_namespace_token("::oo::configuresupport::configurableclass");
    let cfg_object_ns = vm.definition_namespace_token("::oo::configuresupport::configurableobject");
    for (namespace, parent) in [(cfg_class_ns, class_ns), (cfg_object_ns, object_ns)] {
        vm.push_ns_token(vm.namespace_object_bytes(namespace), namespace);
        vm.ns_path_set(vec![parent]);
        vm.pop_ns();
    }
    let support = vm.oo.allocate("oo::configuresupport::configurable");
    vm.oo.configurable_support = Some(support);
    vm.oo.classes.insert(
        support,
        Class {
            supers: vec![object_root],
            definition_namespace: Some(cfg_class_ns),
            object_definition_namespace: Some(cfg_object_ns),
            ..Class::default()
        },
    );
    let namespace = super::fresh_obj_name(vm);
    let namespace_token = native_context::install_object_helpers(vm, &namespace);
    vm.oo.objects.insert(
        support,
        Object {
            cached_name: Rc::new(RefCell::new(None)),
            class: class_root,
            ns: namespace,
            namespace_token: Some(namespace_token),
            creation_id: support.0,
            methods: native_method_cache::MethodTable::default(),
            variables: Vec::new(),
            private_variables: Vec::new(),
            mixins: Vec::new(),
            filters: Vec::new(),
            exported: BTreeSet::new(),
            unexported: BTreeSet::new(),
            readable_properties: BTreeSet::new(),
            writable_properties: BTreeSet::new(),
            destroyed: false,
        },
    );
    let command = vm.register_command(
        "oo::configuresupport::configurable",
        Command::Object(support),
    );
    vm.oo
        .command_keys
        .insert(support, CommandSidecarKey::visible(command));
    let configure = vm
        .oo
        .intern_method_key(NameBytes::from(b"configure".as_slice()));
    vm.oo.classes.get_mut(&support).unwrap().methods.insert(
        configure,
        intrinsic(NativeOoIntrinsicMethod::Configure, true),
    );
    let configurable = vm.oo.classes.get_mut(&configurable_root).unwrap();
    configurable.definition_namespace = Some(cfg_class_ns);
    configurable.constructor.install(intrinsic(
        NativeOoIntrinsicMethod::ConfigurableConstructor,
        false,
    ));
}

pub(super) fn definition_namespace(vm: &Vm, target: OoId, class: bool) -> Option<NsId> {
    let maker = vm.oo.objects.get(&target)?.class;
    vm.oo.class_linear_of(maker).iter().find_map(|step| {
        let provider = vm.oo.classes.get(&step.provider)?;
        if class {
            provider.definition_namespace
        } else {
            provider.object_definition_namespace
        }
    })
}

fn observed_method(
    vm: &Vm,
    name: Option<&str>,
    owner: &native_method_cache::MethodOwner,
) -> NativeOoMethodObservation {
    let method = owner.borrow();
    NativeOoMethodObservation {
        name: name.map(|name| NameBytes::from(vm.oo.method_name_bytes(name))),
        allocation: u64::try_from(Rc::as_ptr(owner).addr())
            .expect("method allocation identity fits u64"),
        visibility_only: method.visibility_only,
        intrinsic: method.intrinsic,
        public: method.exported,
        private: method.private,
    }
}
fn lifecycle(vm: &Vm, slot: &native_method_cache::MethodSlot) -> NativeOoLifecycleObservation {
    slot.owner()
        .map_or(NativeOoLifecycleObservation::Absent, |owner| {
            NativeOoLifecycleObservation::Present(observed_method(vm, None, &owner))
        })
}
fn methods(vm: &Vm, table: &native_method_cache::MethodTable) -> Vec<NativeOoMethodObservation> {
    table
        .iter()
        .map(|(name, owner)| observed_method(vm, Some(name), owner))
        .collect()
}
fn names(vm: &Vm, values: &BTreeSet<String>) -> Vec<NameBytes> {
    values
        .iter()
        .map(|name| NameBytes::from(vm.oo.method_name_bytes(name)))
        .collect()
}

pub(super) fn capture(vm: &Vm, entry: &NativeCompilationEntry) -> Option<NativeOoClassInventory> {
    let version = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()?
        .tcl_version()?;
    if version < tcl_dialect::TclVersion::V8_6 || !entry.closed {
        return None;
    }
    let mut classes = Vec::new();
    for (&id, class) in &vm.oo.classes {
        if version < tcl_dialect::TclVersion::V9_0
            && (Some(id) == vm.oo.configurable_root || Some(id) == vm.oo.configurable_support)
        {
            continue;
        }
        let object = vm.oo.objects.get(&id)?;
        let CommandSidecarKey::Visible(key) = vm.oo.command_keys.get(&id)? else {
            return None;
        };
        let binding =
            vm.native_compilation_binding(key, &Command::Object(id), entry.interpreter)?;
        let Some(current) = entry.commands.iter().find(|candidate| {
            candidate.token == binding.token
                && candidate.implementation_generation == binding.implementation_generation
        }) else {
            continue;
        };
        let namespace = entry
            .retained_namespace_context(u64::from(object.namespace_token?.0))
            .ok()?;
        let definition = |token: Option<NsId>| -> Option<
            Option<tcl_runtime_api::native_compilation::NativeNamespaceContext>,
        > {
            match token {
                None => Some(None),
                Some(token) => Some(Some(
                    entry.retained_namespace_context(u64::from(token.0)).ok()?,
                )),
            }
        };
        let bootstrap_role = if Some(id) == vm.oo.object_root {
            Some(NativeOoBootstrapRole::ObjectRoot)
        } else if Some(id) == vm.oo.class_root {
            Some(NativeOoBootstrapRole::ClassFactory)
        } else if Some(id) == vm.oo.configurable_root {
            Some(NativeOoBootstrapRole::ConfigurableFactory)
        } else if Some(id) == vm.oo.configurable_support {
            Some(NativeOoBootstrapRole::ConfigurableSupport)
        } else {
            None
        };
        let (foundation_epoch, dispatch_epoch) = vm.oo.native_methods.observation_epochs(id);
        classes.push(NativeOoClassObservation {
            object: id.0,
            creation: object.creation_id,
            bootstrap_role,
            command_token: current.token,
            implementation_generation: current.implementation_generation,
            foundation_epoch,
            dispatch_epoch,
            maker: object.class.0,
            superclasses: class.supers.iter().map(|id| id.0).collect(),
            class_mixins: class.mixins.iter().map(|id| id.0).collect(),
            object_mixins: object.mixins.iter().map(|id| id.0).collect(),
            class_filters: Some(class.filters.clone()),
            object_filters: Some(object.filters.clone()),
            namespace,
            class_definition_namespace: definition(class.definition_namespace)?,
            object_definition_namespace: definition(class.object_definition_namespace)?,
            constructor: lifecycle(vm, &class.constructor),
            destructor: lifecycle(vm, &class.destructor),
            class_exported: names(vm, &class.exported),
            class_unexported: names(vm, &class.unexported),
            object_exported: names(vm, &object.exported),
            object_unexported: names(vm, &object.unexported),
            class_methods: methods(vm, &class.methods),
            object_methods: methods(vm, &object.methods),
        });
    }
    Some(NativeOoClassInventory {
        interpreter: entry.interpreter,
        epoch: entry.epoch,
        classes,
    })
}
