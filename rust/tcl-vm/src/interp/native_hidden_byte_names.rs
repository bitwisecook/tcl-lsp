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

use super::*;

fn echo_original(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
    ok(vm
        .invoked_name_value()
        .expect("original hidden invocation value"))
}

#[test]
fn command_visibility_completions_match_original_c_release_results() {
    fn from_hex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let rows = include_str!("../../tests/fixtures/original_visibility_errors.tsv");
    let mut compared = 0;
    for row in rows.lines() {
        let fields = row.split_whitespace().collect::<Vec<_>>();
        assert_eq!(fields.len(), 5);
        let mut vm =
            crate::native_fixture::core(crate::environment::profile_for_dialect(fields[0]));
        let policy = vm.command_visibility_policy().unwrap();
        let completion = match fields[1] {
            "invalid-token" => CommandVisibilityError::InvalidHiddenToken(policy).into_completion(),
            "hide-non-global" => CommandVisibilityError::HideNonGlobal(policy).into_completion(),
            "expose-non-global" => {
                CommandVisibilityError::ExposeNonGlobal(policy).into_completion()
            }
            "missing-command" => {
                CommandVisibilityError::MissingCommand(NameBytes::from(b"missing"), policy)
                    .into_completion()
            }
            "missing-hidden" => {
                CommandVisibilityError::MissingHiddenToken(NameBytes::from(b"N::missing"), policy)
                    .into_completion()
            }
            "already-hidden" => {
                CommandVisibilityError::AlreadyHidden(NameBytes::from(b"held"), policy)
                    .into_completion()
            }
            "command-exists" => {
                CommandVisibilityError::CommandExists(NameBytes::from(b"other"), policy)
                    .into_completion()
            }
            "invoke-hidden-missing" => native_visibility_completion(
                policy,
                NativeCommandVisibilityFailure::InvokeHiddenMissing,
                b"N::missing",
            ),
            _ => panic!("unexpected original failure kind"),
        };
        assert_eq!(fields[2], "1");
        assert_eq!(completion.code, Code::Error);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            from_hex(fields[3])
        );
        let expected_code = from_hex(fields[4]);
        let actual_code = crate::command::opt_get(&completion.options, "-errorcode");
        if expected_code == b"NONE" {
            assert!(actual_code.is_none(), "{row}");
        } else {
            assert_eq!(
                actual_code.unwrap().string_bytes().as_ref(),
                expected_code,
                "{row}"
            );
        }
        compared += 1;
    }
    assert_eq!(compared, 40);
}

#[test]
fn exposed_slots_preserve_empty_and_distinct_hidden_source_roles() {
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = Vm::with_output(Box::new(std::io::sink()));
        vm.set_dialect_profile(crate::environment::profile_for_dialect(
            version.dialect_profile_name(),
        ));
        vm.register_command_in_slot(
            CommandSlot {
                namespace: ROOT_NS,
                simple: NameBytes::from(b"kept"),
            },
            Command::Builtin(echo_original),
        );
        vm.hide_command_bytes(b"kept", b"held").unwrap();
        for destination in [b"::root".as_slice(), b"::", b"N::q"] {
            assert!(matches!(
                vm.expose_own_command_bytes(b"held", destination),
                Err(CommandVisibilityError::ExposeNonGlobal(_))
            ));
            assert_eq!(vm.own_hidden_names_bytes(), [NameBytes::from(b"held")]);
        }
        assert!(
            matches!(vm.expose_own_command_bytes(b"N::missing", b"allowed"),
                Err(CommandVisibilityError::MissingHiddenToken(name, _)) if name.as_bytes() == b"N::missing")
        );
        vm.expose_own_command_bytes(b"held", b"").unwrap();
        assert!(
            vm.lookup_command_bytes_checked(ROOT_NS, b"")
                .unwrap()
                .is_some()
        );
        assert_eq!(
            vm.own_hidden_names_bytes(),
            [] as [tcl_core_types::NameBytes; 0]
        );
    }
}

#[test]
fn hidden_tokens_index_exact_native_bytes_and_keep_original_invocation() {
    for version in tcl_dialect::TclVersion::ALL {
        let profile = crate::environment::profile_for_dialect(version.dialect_profile_name());
        let mut vm = Vm::with_output(Box::new(std::io::sink()));
        vm.set_dialect_profile(profile);
        let slot = CommandSlot {
            namespace: ROOT_NS,
            simple: NameBytes::from(b"c\xff"),
        };
        vm.register_command_in_slot(slot, Command::Builtin(echo_original));
        vm.hide_command_bytes(b"c\xff", b"h\xff").unwrap();
        assert_eq!(vm.own_hidden_names_bytes(), [NameBytes::from(b"h\xff")]);
        let original = Value::from_native_string_bytes(b"h\xff".as_slice());
        let hidden = vm.invoke_own_hidden_value(&original, &[]);
        assert_eq!(hidden.code, Code::Ok);
        assert_eq!(hidden.result.string_bytes().as_ref(), b"h\xff");
        vm.expose_own_command_bytes(b"h\xff", b"q\xff").unwrap();
        assert_eq!(
            vm.own_hidden_names_bytes(),
            [] as [tcl_core_types::NameBytes; 0]
        );
        let selected = vm
            .lookup_command_bytes_checked(ROOT_NS, b"q\xff")
            .unwrap()
            .unwrap();
        assert_eq!(
            vm.command_slot(&selected.0).unwrap().simple.as_bytes(),
            b"q\xff"
        );
        vm.hide_command_bytes(b"q\xff", b"t\0z").unwrap();
        assert_eq!(vm.own_hidden_names_bytes(), [NameBytes::from(b"t")]);
        let original = Value::from_native_string_bytes(b"t\0z".as_slice());
        let hidden = vm.invoke_own_hidden_value(&original, &[]);
        assert_eq!(hidden.code, Code::Ok);
        assert_eq!(hidden.result.string_bytes().as_ref(), b"t\0z");
        vm.expose_own_command_bytes(b"t\0z", b"m\xc0\x80z").unwrap();
        assert!(
            vm.lookup_command_bytes_checked(ROOT_NS, b"m\xc0\x80z")
                .unwrap()
                .is_some()
        );
        assert!(
            vm.lookup_command_bytes_checked(ROOT_NS, b"m")
                .unwrap()
                .is_none()
        );
    }
}
