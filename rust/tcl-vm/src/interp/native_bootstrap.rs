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

//! Constructor inventory controls over the original retained root table.

use super::*;

#[test]
fn native_jim_core_keeps_only_constructor_empty_roles() {
    let profile = tcl_registry::model::resolve_environment("jim").unit_profile();
    let vm = Vm::with_native_core(
        Box::new(Vec::<u8>::new()),
        Rc::new(DefaultHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let context = vm.native_jim_object_context().unwrap();
    let empty = context.empty_object();
    context.with_result_object(|result| {
        assert_eq!(
            result.native_object_identity(),
            empty.native_object_identity()
        );
    });
    assert_eq!(empty.native_object_reference_count(), 4);
    assert!(vm.lookup_command("binary").is_none());
    for command in
        tcl_registry::dictionary_scope::stock_scripted_wrappers(vm.native_invocation_dialect())
    {
        assert!(
            vm.lookup_command(command.command).is_none(),
            "core constructor loaded {}",
            command.command
        );
    }
    assert!(vm.lookup_command("set").is_some());
}

#[test]
fn native_core_root_cells_match_six_original_constructor_inventories() {
    let fixture = include_str!("../../../tcl-registry/tests/data/native_bootstrap/core-roots.tsv");
    for (profile_name, version) in [
        ("tcl8.4", "8.4"),
        ("tcl8.5", "8.5"),
        ("tcl8.6", "8.6"),
        ("tcl9.0", "9.0"),
        ("tcl9.1", "9.1"),
        ("jim", "jim"),
    ] {
        let profile = tcl_registry::model::resolve_environment(profile_name).unit_profile();
        let vm = Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            Rc::new(DefaultHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: Some(b"/native/build/library".to_vec()),
            },
        )
        .unwrap();
        assert_eq!(
            vm.lookup_command("binary").is_some(),
            version != "jim",
            "{profile_name}"
        );
        assert_eq!(
            vm.lookup_command("oo::class").is_some(),
            matches!(version, "8.6" | "9.0" | "9.1"),
            "{profile_name}"
        );
        assert_eq!(
            vm.lookup_command("try").is_some(),
            matches!(version, "8.6" | "9.0" | "9.1" | "jim"),
            "{profile_name}"
        );
        assert_eq!(
            vm.lookup_command("throw").is_some(),
            matches!(version, "8.6" | "9.0" | "9.1"),
            "{profile_name}"
        );
        assert!(vm.lookup_command("auto_load").is_none(), "{profile_name}");
        assert!(vm.lookup_command("auto_import").is_none(), "{profile_name}");
        let expected: Vec<_> = fixture
            .lines()
            .filter_map(|line| {
                let fields: Vec<_> = line.split('\t').collect();
                (fields[0] == version).then_some((fields[1].as_bytes().to_vec(), fields[2] == "1"))
            })
            .collect();
        assert_ne!(expected, [] as [(std::vec::Vec<u8>, bool); 0]);
        let actual: Vec<_> = vm.frames[0]
            .locals
            .keys()
            .map(|name| {
                let id = vm.frames[0].locals.get(name).unwrap();
                let defined = !matches!(vm.var_arena.get(*id).unwrap().state(), Local::Undefined);
                (name.as_bytes().to_vec(), defined)
            })
            .collect();
        assert_eq!(actual, expected, "{profile_name}");
        for name in ["argv", "argc", "argv0", "tcl_library"] {
            assert!(
                !vm.frames[0].locals.contains_key(name),
                "{profile_name}: {name}"
            );
        }
    }
}
