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

//! Actual-engine package command and package-loader source inventory.

use crate::interp::{Code, Interp};
use crate::obj::{self, TclObj};
use std::collections::BTreeMap;
use tcl_dialect::{
    PackagePrefer, TclVersion, compare_versions_bytes_for, select_package_version_bytes_for,
    select_package_version_exact_bytes_for, validate_requirement_bytes_for,
    validate_version_bytes_for, version_matches_exact_bytes_for, version_satisfies_bytes_for,
};
use tcl_registry::native_package::{NativePackageProtocol, PackageDispatch};
use tcl_syntax::value::ValueOps;

struct Loader {
    script: Vec<u8>,
    origin: Option<Vec<u8>>,
}

/// Native package database and the original C9 file-list headers.
#[derive(Default)]
pub struct PackageState {
    provided: BTreeMap<Vec<u8>, Vec<u8>>,
    version_objects: BTreeMap<Vec<u8>, obj::Owned>,
    ifneeded: BTreeMap<(Vec<u8>, Vec<u8>), Loader>,
    unknown: Vec<u8>,
    loading: Vec<(Vec<u8>, Vec<u8>)>,
    prefer: PackagePrefer,
    files: BTreeMap<Vec<u8>, obj::Owned>,
    file_names: Vec<Vec<u8>>,
    file_inventory_active: bool,
    entry_order: tcl_core_types::NativeEntryLedger,
    version_order: BTreeMap<Vec<u8>, Vec<Vec<u8>>>,
}

impl PackageState {
    #[must_use]
    pub fn with_core(version: TclVersion) -> Self {
        let mut state = Self::default();
        state.provide_core(version);
        state
    }

    pub(crate) fn provide_core(&mut self, version: TclVersion) {
        self.entry_order.select_recipe(
            tcl_runtime_api::native_hash_abi::supported_backend_hash_abi(Some(0))
                .and_then(|abi| NativePackageProtocol::C(version).hash_recipe(abi)),
        );
        self.clear_core();
        for core in version.core_provided_packages() {
            let name = core.name.as_bytes().to_vec();
            let version = core.version.as_bytes().to_vec();
            self.entry_order.insert(&name);
            self.provided.insert(name.clone(), version.clone());
            if core.ifneeded_stub {
                self.version_order
                    .entry(name.clone())
                    .or_default()
                    .push(version.clone());
                self.ifneeded.insert(
                    (name, version),
                    Loader {
                        script: b"# Already present, OK?".to_vec(),
                        origin: None,
                    },
                );
            }
        }
    }

    pub(crate) fn clear_core(&mut self) {
        for release in TclVersion::ALL {
            for core in release.core_provided_packages() {
                self.provided.remove(core.name.as_bytes());
                self.version_objects.remove(core.name.as_bytes());
                self.entry_order.remove(core.name.as_bytes());
                self.version_order.remove(core.name.as_bytes());
                self.ifneeded
                    .retain(|(name, _), _| name != core.name.as_bytes());
            }
        }
    }
}

pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"package", package_cmd);
}

fn operand(
    interp: &mut Interp,
    protocol: NativePackageProtocol,
    object: *mut TclObj,
    counted: bool,
) -> Result<Vec<u8>, Code> {
    let bytes = interp
        .native_string_bytes(&object)
        .map_err(|error| interp.report_cmd_error(error.into()))?;
    Ok(if counted {
        bytes.to_vec()
    } else {
        protocol.c_word(&bytes).to_vec()
    })
}

macro_rules! word {
    ($interp:expr, $protocol:expr, $object:expr) => {
        match operand($interp, $protocol, $object, false) {
            Ok(bytes) => bytes,
            Err(code) => return code,
        }
    };
}
macro_rules! counted {
    ($interp:expr, $protocol:expr, $object:expr) => {
        match operand($interp, $protocol, $object, true) {
            Ok(bytes) => bytes,
            Err(code) => return code,
        }
    };
}

fn package_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(protocol) = interp.native_invocation_dialect().native_package_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("package command").into(),
        );
    };
    interp.packages.borrow_mut().entry_order.select_recipe(
        tcl_runtime_api::native_hash_abi::supported_backend_hash_abi(Some(0))
            .and_then(|abi| protocol.hash_recipe(abi)),
    );
    if argv.len() < 2 {
        let suffix = match protocol {
            NativePackageProtocol::C(_) => &b"option ?arg ...?"[..],
            NativePackageProtocol::Jim084 => b"subcommand ?arg ...?",
        };
        return interp.wrong_args_for_invocation(argv, suffix);
    }
    if protocol == NativePackageProtocol::Jim084 {
        return jim_package(interp, argv, protocol);
    }
    let options = protocol.c_members().expect("selected C package table");
    let table =
        tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(options);
    let sub = match interp.native_index_from_original(argv[1], &table, false, "option") {
        Ok(Ok(index)) => options[index],
        Ok(Err(message)) => {
            let word = word!(interp, protocol, argv[1]);
            return interp.error_with_code(
                &message,
                &protocol
                    .index_error_code(b"option", &word)
                    .expect("C package index metadata"),
            );
        }
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let NativePackageProtocol::C(release) = protocol else {
        unreachable!()
    };
    match sub {
        "provide" => provide(interp, argv, protocol, release),
        "require" => package_lookup(interp, argv, protocol, release, true),
        "present" => package_lookup(interp, argv, protocol, release, false),
        "ifneeded" => ifneeded(interp, argv, protocol, release),
        "unknown" => unknown(interp, argv, protocol),
        "names" => names(interp, argv),
        "versions" => versions(interp, argv, protocol),
        "vsatisfies" => vsatisfies_cmd(interp, argv, protocol, release),
        "vcompare" => vcompare_cmd(interp, argv, protocol, release),
        "forget" => forget(interp, argv, protocol),
        "prefer" => prefer(interp, argv, protocol),
        "files" => files(interp, argv, protocol),
        _ => unreachable!("shared package roster"),
    }
}

fn jim_package(interp: &mut Interp, argv: &[*mut TclObj], protocol: NativePackageProtocol) -> Code {
    let member = counted!(interp, protocol, argv[1]);
    let target = if member == b"-help" && argv.len() > 2 {
        Some(counted!(interp, protocol, argv[2]))
    } else {
        None
    };
    let sub = match protocol
        .jim_dispatch(&member, target.as_deref())
        .expect("Jim dispatcher")
    {
        PackageDispatch::Result(bytes) => {
            interp.set_result_bytes(protocol.c_word(&bytes));
            return Code::Ok;
        }
        PackageDispatch::Error(bytes) => return interp.set_error(&bytes),
        PackageDispatch::Invoke(sub) => sub,
    };
    match sub {
        "provide" if (3..=4).contains(&argv.len()) => {
            let name = word!(interp, protocol, argv[2]);
            if interp
                .packages
                .borrow()
                .provided
                .get(&name)
                .is_some_and(|v| !v.is_empty())
            {
                return interp.set_error(
                    &[b"package \"".as_slice(), &name, b"\" was already provided"].concat(),
                );
            }
            {
                let mut state = interp.packages.borrow_mut();
                state.entry_order.insert(&name);
                state.provided.insert(name, b"1.0".to_vec());
            }
            interp.set_result_bytes(b"");
            Code::Ok
        }
        "require" if (3..=4).contains(&argv.len()) => {
            let name = word!(interp, protocol, argv[2]);
            jim_require(interp, &name)
        }
        "forget" if argv.len() >= 3 => forget(interp, argv, protocol),
        "names" | "list" if argv.len() == 2 => names(interp, argv),
        "provide" | "require" => interp.wrong_args_for_prefix(
            argv,
            2,
            protocol
                .member_usage("provide")
                .expect("package provide usage")
                .as_bytes(),
        ),
        "forget" => interp.wrong_args_for_prefix(argv, 2, b"package ..."),
        "names" | "list" => interp.wrong_args_for_prefix(argv, 2, b""),
        _ => unreachable!("selected Jim member"),
    }
}

fn jim_require(interp: &mut Interp, name: &[u8]) -> Code {
    let protocol = NativePackageProtocol::Jim084;
    let provided = interp.packages.borrow().provided.get(name).cloned();
    if let Some(version) = provided {
        return publish_version(interp, protocol, name, &version);
    }
    let Some(paths) = interp.var_get(b"::auto_path") else {
        return jim_load_error(interp, name, b"");
    };
    let paths = obj::Owned::retain(paths);
    let directories = match interp.list_elements(&paths.as_ptr()) {
        Ok(directories) => {
            let mut bytes = Vec::new();
            for directory in directories {
                bytes.push(word!(interp, NativePackageProtocol::Jim084, directory));
            }
            bytes
        }
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    for directory in directories {
        for candidate in
            tcl_dialect::PackageProtocol::Jim.direct_files_bytes(&directory, name, true)
        {
            let host = interp.host();
            let Some(filesystem) = host.filesystem() else {
                return jim_load_error(interp, name, b"");
            };
            let Ok(bytes) = filesystem.read_bytes(&candidate.path) else {
                continue;
            };
            {
                let mut state = interp.packages.borrow_mut();
                state.entry_order.insert(name);
                state.provided.insert(name.to_vec(), Vec::new());
            }
            let code = match candidate.kind {
                tcl_dialect::DirectPackageFileKind::Script => {
                    interp.eval_sourced_global(&bytes, &candidate.path)
                }
                tcl_dialect::DirectPackageFileKind::Native => {
                    let mut script = b"load ".to_vec();
                    crate::list::append_list_element(&mut script, &candidate.path, false);
                    interp.eval_uplevel(0, &script)
                }
            };
            if code != Code::Ok {
                {
                    let mut state = interp.packages.borrow_mut();
                    state.provided.remove(name);
                    state.version_objects.remove(name);
                    state.entry_order.remove(name);
                }
                let prior = interp.result_bytes();
                let message = [
                    prior.as_slice(),
                    if prior.is_empty() { b"" } else { b"\n" },
                    b"Can't load package ",
                    name,
                ]
                .concat();
                interp.set_result_bytes(&message);
                return code;
            }
            let version = {
                let mut state = interp.packages.borrow_mut();
                let version = state.provided.entry(name.to_vec()).or_default();
                if version.is_empty() {
                    *version = b"1.0".to_vec();
                }
                version.clone()
            };
            return publish_version(interp, protocol, name, &version);
        }
    }
    jim_load_error(interp, name, b"")
}

/// Host package provision uses the same selected C package protocol and keeps
/// the interpreter result unchanged on success.
pub(crate) fn provide_package(interp: &mut Interp, name: &[u8], version: &[u8]) -> Code {
    let _scope = match crate::interp::native_operation_currency::NativeOperationScope::enter(interp)
    {
        Ok(scope) => scope,
        Err(cause) => return interp.refuse_native_execution(cause),
    };
    let code = provide_package_selected(interp, name, version);
    if _scope.currency().ensure_current_or_refuse().is_err() {
        return Code::Error;
    }
    code
}

fn provide_package_selected(interp: &mut Interp, name: &[u8], version: &[u8]) -> Code {
    let Some(protocol) = interp.native_invocation_dialect().native_package_protocol() else {
        return interp.refuse_host_command("package provision protocol");
    };
    let NativePackageProtocol::C(release) = protocol else {
        return interp.refuse_host_command("C API package provision protocol");
    };
    if !validate_version_bytes_for(version, release) {
        return invalid_version(interp, protocol, version);
    }
    let existing = interp.packages.borrow().provided.get(name).cloned();
    if let Some(existing) = existing {
        if compare_versions_bytes_for(&existing, version, release) != core::cmp::Ordering::Equal {
            let message = [
                b"conflicting versions provided for package \"".as_slice(),
                name,
                b"\": ",
                &existing,
                b", then ",
                version,
            ]
            .concat();
            return interp.error_with_code(&message, b"TCL PACKAGE VERSIONCONFLICT");
        }
        return Code::Ok;
    }
    let mut state = interp.packages.borrow_mut();
    state.entry_order.insert(name);
    state.provided.insert(name.to_vec(), version.to_vec());
    Code::Ok
}

fn jim_load_error(interp: &mut Interp, name: &[u8], prior: &[u8]) -> Code {
    interp.set_error(
        &[
            prior,
            if prior.is_empty() { b"" } else { b"\n" },
            b"Can't load package ",
            name,
        ]
        .concat(),
    )
}

fn publish_version(
    interp: &mut Interp,
    protocol: NativePackageProtocol,
    name: &[u8],
    version: &[u8],
) -> Code {
    if protocol.retains_version_object() && !version.is_empty() {
        let pointer = {
            let mut state = interp.packages.borrow_mut();
            state
                .version_objects
                .entry(name.to_vec())
                .or_insert_with(|| obj::Owned::fresh(crate::interp::new_string(version)))
                .as_ptr()
        };
        // SAFETY: the package record owns this header, and result publication
        // acquires its independent native result role before any callback.
        unsafe { interp.set_obj_result(pointer) };
    } else {
        interp.set_result_bytes(version);
    }
    Code::Ok
}

fn provide(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    protocol: NativePackageProtocol,
    release: TclVersion,
) -> Code {
    if !(3..=4).contains(&argv.len()) {
        return interp.wrong_args_for_prefix(
            argv,
            2,
            protocol
                .member_usage("provide")
                .expect("package provide usage")
                .as_bytes(),
        );
    }
    let name = word!(interp, protocol, argv[2]);
    if argv.len() == 3 {
        let version = interp
            .packages
            .borrow()
            .provided
            .get(&name)
            .cloned()
            .unwrap_or_default();
        return publish_version(interp, protocol, &name, &version);
    }
    let version = word!(interp, protocol, argv[3]);
    if !validate_version_bytes_for(&version, release) {
        return invalid_version(interp, protocol, &version);
    }
    let existing = interp.packages.borrow().provided.get(&name).cloned();
    if let Some(existing) = existing {
        if compare_versions_bytes_for(&existing, &version, release) != std::cmp::Ordering::Equal {
            return interp.error_with_code(
                &[
                    b"conflicting versions provided for package \"".as_slice(),
                    &name,
                    b"\": ",
                    &existing,
                    b", then ",
                    &version,
                ]
                .concat(),
                protocol.conflict_error_code(),
            );
        }
    } else {
        let mut state = interp.packages.borrow_mut();
        state.entry_order.insert(&name);
        state.provided.insert(name, version);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn lookup_usage(interp: &mut Interp, argv: &[*mut TclObj], release: TclVersion) -> Code {
    let protocol = NativePackageProtocol::C(release);
    interp.wrong_args_for_prefix(
        argv,
        2,
        protocol
            .member_usage("require")
            .expect("C package lookup usage")
            .as_bytes(),
    )
}

fn package_lookup(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    protocol: NativePackageProtocol,
    release: TclVersion,
    load: bool,
) -> Code {
    let mut index = 2;
    let exact = if let Some(&arg) = argv.get(index) {
        word!(interp, protocol, arg) == b"-exact"
    } else {
        false
    };
    if exact {
        index += 1;
    }
    let Some(&name_obj) = argv.get(index) else {
        return lookup_usage(interp, argv, release);
    };
    let mut requirements = Vec::new();
    for &arg in &argv[index + 1..] {
        requirements.push(word!(interp, protocol, arg));
    }
    if (exact && requirements.len() != 1)
        || (!release.has_package_requirements() && requirements.len() > 1)
    {
        return lookup_usage(interp, argv, release);
    }
    for requirement in &requirements {
        let error = if exact {
            (!validate_version_bytes_for(requirement, release))
                .then(|| tcl_dialect::RequirementBytesValidationError::InvalidVersion(requirement))
        } else {
            validate_requirement_bytes_for(requirement, release).err()
        };
        if let Some(error) = error {
            return invalid_requirement(interp, protocol, error);
        }
    }
    let name = word!(interp, protocol, name_obj);
    match check_provided(interp, &name, exact, &requirements, protocol, release, load) {
        Some(Ok(version)) => {
            return publish_version(interp, protocol, &name, &version);
        }
        Some(Err(code)) => return code,
        None if !load => {
            return interp.error_with_code(
                &[b"package ".as_slice(), &name, b" is not present"].concat(),
                &protocol.failure_error_code(
                    tcl_registry::native_package::NativePackageFailure::PresentMissing,
                    &name,
                ),
            );
        }
        None => {}
    }
    let loading = interp
        .packages
        .borrow()
        .loading
        .iter()
        .rev()
        .find(|(key, _)| key == &name)
        .map(|(_, version)| version.clone());
    if let Some(version) = loading {
        let mut message = [
            b"circular package dependency: attempt to provide ".as_slice(),
            &name,
            b" ",
            &version,
            b" requires ",
            &name,
        ]
        .concat();
        for req in &requirements {
            message.push(b' ');
            message.extend_from_slice(req);
        }
        return interp.error_with_code(
            &message,
            &protocol.failure_error_code(
                tcl_registry::native_package::NativePackageFailure::Circularity,
                &name,
            ),
        );
    }
    interp.packages.borrow_mut().entry_order.insert(&name);
    for attempt in 0..2 {
        if let Some(version) = best_ifneeded(interp, &name, exact, &requirements, release) {
            let (script, origin) = {
                let state = interp.packages.borrow();
                let loader = &state.ifneeded[&(name.clone(), version.clone())];
                (loader.script.clone(), loader.origin.clone())
            };
            {
                let mut state = interp.packages.borrow_mut();
                state.loading.push((name.clone(), version.clone()));
                if protocol.tracks_files() {
                    state.file_inventory_active = true;
                    state.file_names.push(name.clone());
                }
            }
            if let Some(origin) = origin {
                if let Err(error) = interp.record_package_source_file(&origin) {
                    interp.end_package_loader_files(protocol);
                    return interp.report_cmd_error(error.into());
                }
            }
            let code = interp.eval_uplevel(0, protocol.c_word(&script));
            interp.end_package_loader_files(protocol);
            if code != Code::Ok {
                {
                    let mut state = interp.packages.borrow_mut();
                    state.provided.remove(&name);
                    state.version_objects.remove(&name);
                }
                if let Some(message) = tcl_dialect::PackageProtocol::Tcl
                    .ifneeded_completion_error_bytes(&name, &version, code.as_int())
                {
                    return interp.error_with_code(
                        &message,
                        &protocol.failure_error_code(
                            tcl_registry::native_package::NativePackageFailure::LoaderCompletion,
                            &name,
                        ),
                    );
                }
                return code;
            }
            let provided = interp.packages.borrow().provided.get(&name).cloned();
            match provided {
                Some(provided)
                    if compare_versions_bytes_for(&provided, &version, release)
                        == std::cmp::Ordering::Equal =>
                {
                    return publish_version(interp, protocol, &name, &provided);
                }
                Some(provided) => {
                    {
                        let mut state = interp.packages.borrow_mut();
                        state.provided.remove(&name);
                        state.version_objects.remove(&name);
                    }
                    return interp.error_with_code(
                        &[
                            b"attempt to provide package ".as_slice(),
                            &name,
                            b" ",
                            &version,
                            b" failed: package ",
                            &name,
                            b" ",
                            &provided,
                            b" provided instead",
                        ]
                        .concat(),
                        &protocol.failure_error_code(
                            tcl_registry::native_package::NativePackageFailure::LoaderWrongVersion,
                            &name,
                        ),
                    );
                }
                None => {
                    return interp.error_with_code(
                        &[
                            b"attempt to provide package ".as_slice(),
                            &name,
                            b" ",
                            &version,
                            b" failed: no version of package ",
                            &name,
                            b" provided",
                        ]
                        .concat(),
                        &protocol.failure_error_code(
                            tcl_registry::native_package::NativePackageFailure::LoaderMissing,
                            &name,
                        ),
                    );
                }
            }
        }
        if attempt == 0 {
            let handler = interp.packages.borrow().unknown.clone();
            if handler.is_empty() {
                break;
            }
            let mut script = protocol.c_word(&handler).to_vec();
            script.push(b' ');
            tcl_syntax::list::append_list_element(&mut script, &name, false);
            if requirements.is_empty() {
                script.extend_from_slice(if release.has_package_requirements() {
                    b" 0-"
                } else {
                    b" {}"
                });
            }
            for requirement in &requirements {
                script.push(b' ');
                let exact_requirement;
                let req = if exact && release.has_package_requirements() {
                    exact_requirement =
                        [requirement.as_slice(), b"-", requirement.as_slice()].concat();
                    &exact_requirement
                } else {
                    requirement
                };
                tcl_syntax::list::append_list_element(&mut script, req, false);
            }
            if exact && !release.has_package_requirements() {
                script.extend_from_slice(b" -exact");
            }
            let code = interp.eval_uplevel(0, &script);
            if code != Code::Ok {
                if code != Code::Error {
                    return interp.error_with_code(
                        format!("bad return code: {}", code.as_int()).as_bytes(),
                        &protocol.failure_error_code(
                            tcl_registry::native_package::NativePackageFailure::LoaderCompletion,
                            &name,
                        ),
                    );
                }
                return code;
            }
        }
    }
    interp.error_with_code(
        &[b"can't find package ".as_slice(), &name].concat(),
        &protocol.failure_error_code(
            tcl_registry::native_package::NativePackageFailure::RequireMissing,
            &name,
        ),
    )
}

fn best_ifneeded(
    interp: &Interp,
    name: &[u8],
    exact: bool,
    requirements: &[Vec<u8>],
    release: TclVersion,
) -> Option<Vec<u8>> {
    let state = interp.packages.borrow();
    let candidates = state
        .ifneeded
        .keys()
        .filter(|(key, _)| key == name)
        .map(|(_, version)| version.clone())
        .collect::<Vec<_>>();
    let selected = if exact {
        select_package_version_exact_bytes_for(&candidates, &requirements[0], release)
    } else {
        select_package_version_bytes_for(
            &candidates,
            &requirements.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            state.prefer,
            release,
        )
    }?;
    Some(candidates[selected].clone())
}

fn check_provided(
    interp: &mut Interp,
    name: &[u8],
    exact: bool,
    requirements: &[Vec<u8>],
    protocol: NativePackageProtocol,
    release: TclVersion,
    load: bool,
) -> Option<Result<Vec<u8>, Code>> {
    let version = interp.packages.borrow().provided.get(name)?.clone();
    let satisfied = requirements.is_empty()
        || if exact {
            version_matches_exact_bytes_for(&version, &requirements[0], release)
        } else {
            requirements
                .iter()
                .any(|req| version_satisfies_bytes_for(&version, req, release))
        };
    if satisfied {
        return Some(Ok(version));
    }
    let mut message = [
        b"version conflict for package \"".as_slice(),
        name,
        b"\": have ",
        &version,
        b", need ",
        if exact { b"exactly " } else { b"" },
    ]
    .concat();
    for (index, req) in requirements.iter().enumerate() {
        if index > 0 {
            message.push(b' ');
        }
        message.extend_from_slice(req);
    }
    Some(Err(interp.error_with_code(
        &message,
        &if load {
            protocol.conflict_error_code().to_vec()
        } else {
            protocol.failure_error_code(
                tcl_registry::native_package::NativePackageFailure::PresentMissing,
                name,
            )
        },
    )))
}

fn ifneeded(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    protocol: NativePackageProtocol,
    release: TclVersion,
) -> Code {
    if !(4..=5).contains(&argv.len()) {
        return interp.wrong_args_for_prefix(argv, 2, b"package version ?script?");
    }
    let name = word!(interp, protocol, argv[2]);
    let version = word!(interp, protocol, argv[3]);
    if !validate_version_bytes_for(&version, release) {
        return invalid_version(interp, protocol, &version);
    }
    let key = interp
        .packages
        .borrow()
        .ifneeded
        .keys()
        .find(|(key, known)| {
            key == &name
                && compare_versions_bytes_for(known, &version, release) == std::cmp::Ordering::Equal
        })
        .cloned()
        .unwrap_or((name, version));
    if argv.len() == 5 {
        let script = counted!(interp, protocol, argv[4]);
        {
            let mut state = interp.packages.borrow_mut();
            state.entry_order.insert(&key.0);
            let versions = state.version_order.entry(key.0.clone()).or_default();
            if !versions.contains(&key.1) {
                versions.push(key.1.clone());
            }
        }
        let origin = interp.package_source_origin();
        interp
            .packages
            .borrow_mut()
            .ifneeded
            .insert(key, Loader { script, origin });
        interp.set_result_bytes(b"");
    } else {
        let script = interp
            .packages
            .borrow()
            .ifneeded
            .get(&key)
            .map(|loader| loader.script.clone())
            .unwrap_or_default();
        interp.set_result_bytes(protocol.c_word(&script));
    }
    Code::Ok
}

fn unknown(interp: &mut Interp, argv: &[*mut TclObj], protocol: NativePackageProtocol) -> Code {
    match argv.len() {
        2 => {
            let bytes = interp.packages.borrow().unknown.clone();
            interp.set_result_bytes(protocol.c_word(&bytes));
            Code::Ok
        }
        3 => {
            interp.packages.borrow_mut().unknown = counted!(interp, protocol, argv[2]);
            interp.set_result_bytes(b"");
            Code::Ok
        }
        _ => interp.wrong_args_for_prefix(argv, 2, b"?command?"),
    }
}

fn names(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args_for_prefix(argv, 2, b"");
    }
    let values = {
        let state = interp.packages.borrow();
        let Some(order) = state.entry_order.keys() else {
            drop(state);
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable("package table ABI")
                    .into(),
            );
        };
        order
            .into_iter()
            .filter(|key| {
                state.provided.contains_key(*key)
                    || state.ifneeded.keys().any(|(name, _)| name == key)
            })
            .map(crate::interp::new_string)
            .collect::<Vec<_>>()
    };
    interp.set_result(interp.new_list_object(&values));
    Code::Ok
}

fn versions(interp: &mut Interp, argv: &[*mut TclObj], protocol: NativePackageProtocol) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"package");
    }
    let name = word!(interp, protocol, argv[2]);
    let values = interp
        .packages
        .borrow()
        .version_order
        .get(&name)
        .into_iter()
        .flatten()
        .map(|version| crate::interp::new_string(version))
        .collect::<Vec<_>>();
    interp.set_result(interp.new_list_object(&values));
    Code::Ok
}

fn forget(interp: &mut Interp, argv: &[*mut TclObj], protocol: NativePackageProtocol) -> Code {
    for &arg in &argv[2..] {
        let name = word!(interp, protocol, arg);
        let mut state = interp.packages.borrow_mut();
        state.provided.remove(&name);
        state.version_objects.remove(&name);
        state.ifneeded.retain(|(key, _), _| key != &name);
        state.files.remove(&name);
        state.entry_order.remove(&name);
        state.version_order.remove(&name);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn prefer(interp: &mut Interp, argv: &[*mut TclObj], protocol: NativePackageProtocol) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"?latest|stable?");
    }
    if argv.len() == 3 {
        let table = tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(
            protocol.preference_members().expect("C preference table"),
        );
        match interp.native_index_from_original(argv[2], &table, false, "preference") {
            Ok(Ok(0)) => interp.packages.borrow_mut().prefer = PackagePrefer::Latest,
            Ok(Ok(_)) => {}
            Ok(Err(message)) => {
                let word = word!(interp, protocol, argv[2]);
                return interp.error_with_code(
                    &message,
                    &protocol
                        .index_error_code(b"preference", &word)
                        .expect("C preference index"),
                );
            }
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    }
    let prefer = interp.packages.borrow().prefer;
    interp.set_result_bytes(match prefer {
        PackagePrefer::Latest => b"latest",
        PackagePrefer::Stable => b"stable",
    });
    Code::Ok
}

fn files(interp: &mut Interp, argv: &[*mut TclObj], protocol: NativePackageProtocol) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"package");
    }
    if !interp.packages.borrow().file_inventory_active {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    let name = word!(interp, protocol, argv[2]);
    let header = interp
        .packages
        .borrow()
        .files
        .get(&name)
        .map(obj::Owned::as_ptr);
    if let Some(header) = header {
        interp.set_result(header);
    } else {
        interp.set_result_bytes(b"");
    }
    Code::Ok
}

fn vsatisfies_cmd(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    protocol: NativePackageProtocol,
    release: TclVersion,
) -> Code {
    if argv.len() < 4 || (!release.has_package_requirements() && argv.len() != 4) {
        return interp.wrong_args_for_prefix(
            argv,
            2,
            protocol
                .member_usage("vsatisfies")
                .expect("C vsatisfies usage")
                .as_bytes(),
        );
    }
    let version = word!(interp, protocol, argv[2]);
    if !validate_version_bytes_for(&version, release) {
        return invalid_version(interp, protocol, &version);
    }
    let mut requirements = Vec::new();
    for &arg in &argv[3..] {
        let req = word!(interp, protocol, arg);
        if let Err(error) = validate_requirement_bytes_for(&req, release) {
            return invalid_requirement(interp, protocol, error);
        }
        requirements.push(req);
    }
    interp.set_result_bytes(
        if requirements
            .iter()
            .any(|req| version_satisfies_bytes_for(&version, req, release))
        {
            b"1"
        } else {
            b"0"
        },
    );
    Code::Ok
}

fn vcompare_cmd(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    protocol: NativePackageProtocol,
    release: TclVersion,
) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"version1 version2");
    }
    let left = word!(interp, protocol, argv[2]);
    let right = word!(interp, protocol, argv[3]);
    for version in [&left, &right] {
        if !validate_version_bytes_for(version, release) {
            return invalid_version(interp, protocol, version);
        }
    }
    interp.set_result_bytes(match compare_versions_bytes_for(&left, &right, release) {
        std::cmp::Ordering::Less => b"-1",
        std::cmp::Ordering::Equal => b"0",
        std::cmp::Ordering::Greater => b"1",
    });
    Code::Ok
}

fn invalid_version(interp: &mut Interp, protocol: NativePackageProtocol, version: &[u8]) -> Code {
    interp.error_with_code(
        &[
            b"expected version number but got \"".as_slice(),
            version,
            b"\"",
        ]
        .concat(),
        protocol
            .version_error_code(false)
            .expect("C version metadata"),
    )
}

fn invalid_requirement(
    interp: &mut Interp,
    protocol: NativePackageProtocol,
    error: tcl_dialect::RequirementBytesValidationError<'_>,
) -> Code {
    match error {
        tcl_dialect::RequirementBytesValidationError::InvalidVersion(version) => {
            invalid_version(interp, protocol, version)
        }
        tcl_dialect::RequirementBytesValidationError::InvalidRange(range) => interp
            .error_with_code(
                &[
                    b"expected versionMin-versionMax but got \"".as_slice(),
                    range,
                    b"\"",
                ]
                .concat(),
                protocol.version_error_code(true).expect("C range metadata"),
            ),
    }
}

impl Interp {
    fn end_package_loader_files(&mut self, protocol: NativePackageProtocol) {
        let mut state = self.packages.borrow_mut();
        state.loading.pop();
        if protocol.tracks_files() {
            state.file_names.pop();
        }
    }

    pub(crate) fn begin_package_initialization(&mut self) -> bool {
        let Some(name) = self
            .native_invocation_dialect()
            .native_package_protocol()
            .and_then(NativePackageProtocol::initialization_package)
        else {
            return false;
        };
        let mut state = self.packages.borrow_mut();
        state.file_inventory_active = true;
        state.file_names.push(name.to_vec());
        true
    }

    pub(crate) fn end_package_initialization(&mut self, active: bool) {
        if active {
            self.packages.borrow_mut().file_names.pop();
        }
    }

    pub(crate) fn take_package_file_scope(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.packages.borrow_mut().file_names)
    }

    pub(crate) fn restore_package_file_scope(&mut self, scope: Vec<Vec<u8>>) {
        self.packages.borrow_mut().file_names = scope;
    }

    pub(crate) fn package_source_origin(&self) -> Option<Vec<u8>> {
        self.native_invocation_dialect()
            .native_package_protocol()
            .filter(|protocol| protocol.tracks_files())?;
        {
            let name = self.current_script();
            (!name.is_empty()).then(|| tcl_core_types::c_string_extent(&name).to_vec())
        }
    }

    /// Record a reached native source boundary against the innermost loader.
    pub(crate) fn record_package_source_file(
        &mut self,
        filename: &[u8],
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let Some(protocol) = self.native_invocation_dialect().native_package_protocol() else {
            return Ok(());
        };
        if !protocol.tracks_files() {
            return Ok(());
        }
        let Some(name) = self.packages.borrow().file_names.last().cloned() else {
            return Ok(());
        };
        let strings = self
            .native_invocation_dialect()
            .native_string_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "package source string",
            ))?;
        let header = {
            let mut state = self.packages.borrow_mut();
            state
                .files
                .entry(name)
                .or_insert_with(|| {
                    obj::Owned::fresh(crate::list::new_list_obj_native(&[], strings))
                })
                .as_ptr()
        };
        if obj::is_shared(header) {
            return Err(tcl_syntax::value::ValueError::NativeFatalCondition(
                tcl_syntax::raw_string::NativeFatalCondition::SharedPackageFileListMutation,
            ));
        }
        let file = obj::Owned::fresh(crate::interp::new_string(protocol.c_word(filename)));
        crate::list::list_append(header, file.as_ptr()).map_err(|error| {
            tcl_syntax::value::ValueError::ListParse {
                error: error.shared(),
                source: Vec::new(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};
    use crate::obj;
    use tcl_dialect::TclVersion;
    use tcl_syntax::value::ValueOps;

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(counters::finalize(), 0, "leak");
        assert_eq!(counters::double_free_count(), 0);
    }

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    fn native_package_fixture(profile: &'static tcl_dialect::DialectProfile) -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("the original package fixture requires its selected native core")
    }

    #[test]
    fn jim_direct_packages_use_global_source_and_preserve_partial_effects() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let directory =
            std::env::temp_dir().join(format!("2286-jim-package-runtime-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("package directory");
        for (name, source) in [
            (
                "plain",
                "set ::loaderContext [list [namespace current] [info level]]; return done",
            ),
            (
                "recursive",
                "set ::recursiveVersion [package require recursive 999]; package provide recursive invalid",
            ),
            (
                "bad",
                "proc ::retainedSideEffect {} {return yes}; package provide bad 999; error stop",
            ),
        ] {
            std::fs::write(directory.join(format!("{name}.tcl")), source).expect("package source");
        }
        let mut interp = native_package_fixture(profile);
        let setup = format!(
            "set ::auto_path [list {}]",
            tcl_syntax::list::list_element(&directory.to_string_lossy())
        );
        assert_eq!(interp.eval_str(setup.as_bytes()), Code::Ok);
        assert_eq!(
            run(
                &mut interp,
                b"namespace eval Probe {proc p {} {package require plain 999}; p}"
            ),
            b"1.0"
        );
        assert_eq!(run(&mut interp, b"set ::loaderContext"), b":: 0");
        assert_eq!(run(&mut interp, b"package require recursive"), b"1.0");
        assert_eq!(run(&mut interp, b"set ::recursiveVersion"), b"");
        assert_eq!(interp.eval_str(b"package require bad"), Code::Error);
        assert!(
            !interp
                .packages
                .borrow()
                .provided
                .contains_key(b"bad".as_slice())
        );
        assert_eq!(run(&mut interp, b"retainedSideEffect"), b"yes");
        assert_eq!(interp.eval_str(b"package provide plain"), Code::Error);
        std::fs::remove_dir_all(directory).expect("remove fixtures");
    }

    #[test]
    fn package_option_word_resolves_like_tcl_get_index_from_obj() {
        const MUST: &str = "must be files, forget, ifneeded, names, prefer, present, provide, require, unknown, \
                            vcompare, versions, or vsatisfies";
        leak_free(|i| {
            let err = |i: &mut Interp, script: &[u8]| {
                assert_eq!(i.eval_str(script), Code::Error, "expected an error");
                String::from_utf8_lossy(&i.result_bytes()).into_owned()
            };
            assert_eq!(err(i, b"package x"), format!("bad option \"x\": {MUST}"));
            assert_eq!(
                err(i, b"package {}"),
                format!("ambiguous option \"\": {MUST}")
            );
            assert_eq!(
                err(i, b"package v"),
                format!("ambiguous option \"v\": {MUST}")
            );
            // A unique prefix resolves.
            assert_eq!(run(i, b"package provide foo 1.0"), b"");
            assert_eq!(
                run(i, b"llength [lsearch -all -exact [package n] foo]"),
                b"1"
            );
            // C's lookup error code travels with the message.
            assert_eq!(
                run(i, b"catch {package x} e opts; dict get $opts -errorcode"),
                b"TCL LOOKUP INDEX option x"
            );
        });
    }

    #[test]
    fn core_require_and_provide() {
        leak_free(|i| {
            assert_eq!(run(i, b"package require -exact tcl 9.0.4"), b"9.0.4");
            assert_eq!(run(i, b"package require Tcl 8.5-"), b"9.0.4");
            assert_eq!(run(i, b"package provide Tcl"), b"9.0.4");
            run(i, b"package provide mypkg 1.2");
            assert_eq!(run(i, b"package require mypkg"), b"1.2");
            assert_eq!(i.eval_str(b"package require nosuch"), Code::Error);
        });
    }

    #[test]
    fn vsatisfies_and_vcompare() {
        leak_free(|i| {
            assert_eq!(run(i, b"package vsatisfies 9.0.4 9.0-"), b"1");
            assert_eq!(run(i, b"package vsatisfies 9.0.4 8.5-9.0"), b"0");
            assert_eq!(run(i, b"package vsatisfies 8.6.1 8.5"), b"1");
            assert_eq!(run(i, b"package vsatisfies 9.0 8.5"), b"0");
            assert_eq!(run(i, b"package vcompare 8.5 9.0"), b"-1");
            assert_eq!(run(i, b"package vcompare 9.0.4 9.0.4"), b"0");
        });
    }

    #[test]
    fn package_versions_use_the_pinned_shared_release_policy() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            for command in [
                "package provide p 1.2+x",
                "package ifneeded p 1.2+x {}",
                "package vsatisfies 1.2 1.2+x",
                "package vcompare 1.2+x 1.2",
                "package require absent 1.2+x",
                "package present absent 1.2+x",
            ] {
                assert_eq!(i.eval_str(command.as_bytes()), Code::Error, "{command}");
                let caught =
                    format!("catch {{{command}}} message options; dict get $options -errorcode");
                assert_eq!(run(i, caught.as_bytes()), b"TCL VALUE VERSION", "{command}");
            }
            assert_eq!(run(i, b"package provide p 1.2"), b"");
            assert_eq!(
                run(i, b"catch {package provide p 1.3} message; set message"),
                b"conflicting versions provided for package \"p\": 1.2, then 1.3"
            );
            assert_eq!(
                run(
                    i,
                    b"catch {package provide p 1.3} message options; dict get $options -errorcode"
                ),
                b"TCL PACKAGE VERSIONCONFLICT"
            );

            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            assert_eq!(
                run(
                    i,
                    b"catch {package vsatisfies 1 1-bad} message; set message"
                ),
                b"expected version number but got \"bad\""
            );
            assert_eq!(
                run(i, b"catch {package vsatisfies 1 1-2-3} message options; dict get $options -errorcode"),
                b"TCL VALUE VERSIONRANGE"
            );
            assert_eq!(run(i, b"package provide q 1.2+x"), b"");
            assert_eq!(run(i, b"package provide q 1.2+y"), b"");
            assert_eq!(run(i, b"package vcompare 1.2+x 1.2"), b"0");
            assert_eq!(
                run(i, b"package ifneeded r 1.2+x {package provide r 1.2+x}"),
                b""
            );
            assert_eq!(run(i, b"package require -exact r 1.2+y"), b"1.2+x");
            run(
                i,
                b"proc package_callback args {set ::package_callback $args}",
            );
            run(i, b"package unknown package_callback");
            assert_eq!(
                run(
                    i,
                    b"catch {package require -exact absent {1.2+x space{brace}}}; set ::package_callback"
                ),
                b"absent {1.2+x space{brace}-1.2+x space{brace}}"
            );
        });
    }

    #[test]
    fn ifneeded_and_unknown() {
        leak_free(|i| {
            run(i, b"package ifneeded foo 1.0 {set loaded yes}");
            assert_eq!(run(i, b"package ifneeded foo 1.0"), b"set loaded yes");
            run(i, b"package unknown myhandler");
            assert_eq!(run(i, b"package unknown"), b"myhandler");
        });
    }
    #[test]
    fn provided_version_headers_match_all_native_30_windows() {
        use crate::obj;
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        use tcl_syntax::scalar_getter::NativeScalarGetterKind;
        const FIXTURES: [&str; 5] = [
            include_str!(
                "../../../rust/tcl-registry/tests/data/native_package_versions/8.4.20.tsv"
            ),
            include_str!(
                "../../../rust/tcl-registry/tests/data/native_package_versions/8.5.19.tsv"
            ),
            include_str!(
                "../../../rust/tcl-registry/tests/data/native_package_versions/8.6.18.tsv"
            ),
            include_str!("../../../rust/tcl-registry/tests/data/native_package_versions/9.0.4.tsv"),
            include_str!("../../../rust/tcl-registry/tests/data/native_package_versions/9.1.0.tsv"),
        ];
        fn class(value: *mut obj::TclObj) -> &'static str {
            match obj::native_object_snapshot(value).unwrap().cache {
                Cache::None => "none",
                Cache::Numeric(_) => "int",
                other => panic!("unexpected version primary {other:?}"),
            }
        }
        let mut compared = 0;
        for (version, fixture) in tcl_dialect::TclVersion::ALL.into_iter().zip(FIXTURES) {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                assert_eq!(
                    interp.eval_str(b"package provide probe 12; package provide probe"),
                    Code::Ok
                );
                let held = obj::Owned::retain(interp.result_obj());
                let rows: Vec<Vec<&str>> = fixture
                    .lines()
                    .map(|row| row.split('\t').collect())
                    .collect();
                assert_eq!(class(held.as_ptr()), rows[0][2]);
                assert_eq!(
                    unsafe { (*held.as_ptr()).ref_count }.to_string(),
                    rows[0][3]
                );
                compared += 1;
                crate::typed_value::native_scalar_getter(
                    held.as_ptr(),
                    interp.native_invocation_dialect(),
                    NativeScalarGetterKind::Int,
                )
                .unwrap();
                assert_eq!(class(held.as_ptr()), rows[1][3]);
                assert_eq!(rows[1][2], "12");
                compared += 1;
                for (member, row) in ["provide", "require", "present"]
                    .into_iter()
                    .zip(&rows[2..5])
                {
                    let script = format!("package {member} probe");
                    assert_eq!(interp.eval_str(script.as_bytes()), Code::Ok);
                    assert_eq!(row[1], "0");
                    let result = interp.result_obj();
                    assert_eq!(usize::from(result == held.as_ptr()).to_string(), row[2]);
                    assert_eq!(class(result), row[3]);
                    assert_eq!(unsafe { (*held.as_ptr()).ref_count }.to_string(), row[4]);
                    compared += 1;
                }
                assert_eq!(interp.eval_str(b"package forget probe"), Code::Ok);
                assert_eq!(
                    unsafe { (*held.as_ptr()).ref_count }.to_string(),
                    rows[5][2]
                );
                assert_eq!(class(held.as_ptr()), rows[5][3]);
                compared += 1;
            });
        }
        assert_eq!(compared, 30);
    }

    #[test]
    fn package_files_headers_match_all_native_10_windows_and_scope_lifetimes() {
        use super::NativePackageProtocol;
        use crate::obj;
        const FIXTURES: [&str; 2] = [
            include_str!("../../../rust/tcl-registry/tests/data/native_package_files/9.0.4.tsv"),
            include_str!("../../../rust/tcl-registry/tests/data/native_package_files/9.1.0.tsv"),
        ];
        let mut compared = 0;
        for (version, fixture) in [tcl_dialect::TclVersion::V9_0, tcl_dialect::TclVersion::V9_1]
            .into_iter()
            .zip(FIXTURES)
        {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                let protocol = NativePackageProtocol::C(version);
                {
                    let mut state = interp.packages.borrow_mut();
                    state.file_inventory_active = true;
                    state.loading.push((b"probe".to_vec(), b"1".to_vec()));
                    state.file_names.push(b"probe".to_vec());
                }
                interp.record_package_source_file(b"leaf.tcl").unwrap();
                interp.end_package_loader_files(protocol);
                let rows: Vec<Vec<&str>> = fixture
                    .lines()
                    .map(|row| row.split('\t').collect())
                    .collect();
                let pointer = interp.packages.borrow().files[b"probe".as_slice()].as_ptr();
                assert_eq!(unsafe { (*pointer).ref_count }.to_string(), rows[0][2]);
                assert_eq!(rows[0][3], "list");
                compared += 1;
                assert_eq!(interp.eval_str(b"package files probe"), Code::Ok);
                assert_eq!(interp.result_obj(), pointer);
                assert_eq!(unsafe { (*pointer).ref_count }.to_string(), rows[1][3]);
                compared += 1;
                let held = obj::Owned::retain(pointer);
                assert_eq!(unsafe { (*pointer).ref_count }.to_string(), rows[2][3]);
                compared += 1;
                assert_eq!(interp.eval_str(b"package forget probe"), Code::Ok);
                assert_eq!(unsafe { (*pointer).ref_count }.to_string(), rows[3][3]);
                compared += 1;
                unsafe { interp.set_obj_result(held.as_ptr()) };
                assert_eq!(unsafe { (*pointer).ref_count }.to_string(), rows[4][3]);
                compared += 1;
                interp.set_result_bytes(b"");
                drop(held);
                interp
                    .packages
                    .borrow_mut()
                    .file_names
                    .push(b"outer".to_vec());
                interp.record_package_source_file(b"first.tcl").unwrap();
                let detached = interp.take_package_file_scope();
                interp.record_package_source_file(b"excluded.tcl").unwrap();
                interp
                    .packages
                    .borrow_mut()
                    .file_names
                    .push(b"nested".to_vec());
                interp.record_package_source_file(b"nested.tcl").unwrap();
                interp.packages.borrow_mut().file_names.pop();
                interp.restore_package_file_scope(detached);
                assert_eq!(interp.eval_str(b"package forget outer"), Code::Ok);
                interp
                    .record_package_source_file(b"after-forget.tcl")
                    .unwrap();
                interp.packages.borrow_mut().file_names.pop();
                assert_eq!(run(interp, b"package files outer"), b"after-forget.tcl");
                assert_eq!(run(interp, b"package files nested"), b"nested.tcl");
            });
        }
        assert_eq!(compared, 10);
    }

    #[test]
    fn shared_package_files_append_is_an_outer_native_fatal_condition() {
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                interp
                    .packages
                    .borrow_mut()
                    .file_names
                    .push(b"probe".to_vec());
                interp.record_package_source_file(b"leaf.tcl").unwrap();
                let pointer = interp.packages.borrow().files[b"probe".as_slice()].as_ptr();
                let retained = obj::Owned::retain(pointer);
                let error = interp
                    .record_package_source_file(b"second.tcl")
                    .unwrap_err();
                assert!(matches!(
                    error,
                    tcl_syntax::value::ValueError::NativeFatalCondition(
                        tcl_syntax::raw_string::NativeFatalCondition::SharedPackageFileListMutation
                    )
                ));
                assert_eq!(
                    interp
                        .native_string_bytes(&retained.as_ptr())
                        .unwrap()
                        .as_ref(),
                    b"leaf.tcl"
                );
            });
        }
    }

    #[test]
    fn package_completion_matches_all_six_native_84_controls() {
        let jim: &'static tcl_dialect::DialectProfile =
            Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )));
        const FIXTURES: [&str; 6] = [
            include_str!(
                "../../../rust/tcl-registry/tests/data/native_package_ordinary/8.4.20.tsv"
            ),
            include_str!(
                "../../../rust/tcl-registry/tests/data/native_package_ordinary/8.5.19.tsv"
            ),
            include_str!(
                "../../../rust/tcl-registry/tests/data/native_package_ordinary/8.6.18.tsv"
            ),
            include_str!("../../../rust/tcl-registry/tests/data/native_package_ordinary/9.0.4.tsv"),
            include_str!("../../../rust/tcl-registry/tests/data/native_package_ordinary/9.1.0.tsv"),
            include_str!("../../../rust/tcl-registry/tests/data/native_package_ordinary/Jim.tsv"),
        ];
        const FILES: &[(&str, &[u8])] = &[
            (
                "/workspace/.proofs/2286-packages-dialects/package-native-completion/pkgIndex.tcl",
                include_bytes!(
                    "../../../rust/tcl-registry/tests/data/native_package_ordinary/pkgIndex.tcl"
                )
                .as_slice(),
            ),
            (
                "/workspace/.proofs/2286-packages-dialects/package-native-completion/leaf.tcl",
                include_bytes!(
                    "../../../rust/tcl-registry/tests/data/native_package_ordinary/leaf.tcl"
                )
                .as_slice(),
            ),
            (
                "/workspace/.proofs/2286-packages-dialects/package-native-completion/nested.tcl",
                include_bytes!(
                    "../../../rust/tcl-registry/tests/data/native_package_ordinary/nested.tcl"
                )
                .as_slice(),
            ),
            (
                "/workspace/.proofs/2286-packages-dialects/package-native-completion/skip.tcl",
                include_bytes!(
                    "../../../rust/tcl-registry/tests/data/native_package_ordinary/skip.tcl"
                )
                .as_slice(),
            ),
        ];
        fn unhex(text: &str) -> Vec<u8> {
            assert_eq!(text.len() % 2, 0);
            text.as_bytes()
                .chunks_exact(2)
                .map(|pair| {
                    let digits = std::str::from_utf8(pair).unwrap();
                    u8::from_str_radix(digits, 16).unwrap()
                })
                .collect()
        }
        let mut compared = 0;
        for (engine, fixture) in FIXTURES.into_iter().enumerate() {
            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields.len(), 4);
                let name = fields[0];
                let script = unhex(fields[1]);
                let expected = unhex(fields[3]);
                let profile = if engine == 5 {
                    jim
                } else {
                    tcl_registry::model::ingress::resolve_environment(
                        TclVersion::ALL[engine].dialect_name(),
                    )
                    .unit_profile()
                };
                let mut interp = native_package_fixture(profile);
                let host =
                    tcl_test_support::fixed_sources::FixedSourceHost::new(interp.host(), FILES);
                interp.set_host(std::rc::Rc::new(host));
                let code = interp.eval_str(&script);
                assert_eq!(code.as_int().to_string(), fields[2], "{engine} {name}");
                assert_eq!(interp.result_bytes(), expected, "{engine} {name}");
                compared += 1;
            }
        }
        assert_eq!(compared, 84);
    }
}
