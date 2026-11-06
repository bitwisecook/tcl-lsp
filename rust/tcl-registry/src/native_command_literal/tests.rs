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

use tcl_dialect::TclVersion;
use tcl_runtime_api::native_command_name::{
    NativeCommandNameCache, NativeCommandNameLookupState, NativeCommandNameReference,
    NativeCommandNameTarget,
};
use tcl_runtime_api::native_compilation::NativeInterpreterIdentity;

fn records(version: TclVersion) -> (NativeCommandNameCache, NativeCommandNameLookupState) {
    let interpreter = NativeInterpreterIdentity {
        owner: 4,
        interpreter: 2,
    };
    let reference = NativeCommandNameReference {
        namespace_token: 5,
        command_reference_epoch: 3,
    };
    (
        NativeCommandNameCache {
            interpreter,
            version,
            slot: tcl_core_types::NativeByteCommandSlot::new(
                tcl_core_types::ByteNamespacePath::root(),
                b"head".as_slice().into(),
            ),
            namespace_token: 7,
            token: 19,
            implementation_generation: 19,
            command_epoch: 8,
            reference: Some(reference),
        },
        NativeCommandNameLookupState {
            interpreter,
            reference,
            target: Some(NativeCommandNameTarget {
                token: 19,
                implementation_generation: 19,
                command_epoch: 8,
                namespace_token: 7,
                namespace_dying: false,
            }),
        },
    )
}

#[test]
fn live_cache_validity_requires_native_nodes_and_reference_incarnations() {
    for version in TclVersion::ALL {
        let protocol = crate::InvocationDialect::for_version(version)
            .native_command_name_protocol()
            .unwrap();
        let (cache, state) = records(version);
        assert!(protocol.cache_is_current(&cache, &state));
        let mut renamed = state.clone();
        renamed.target.as_mut().unwrap().namespace_token = 99;
        assert!(
            protocol.cache_is_current(&cache, &renamed),
            "rename before node epoch changes"
        );
        renamed.target.as_mut().unwrap().command_epoch += 1;
        assert!(!protocol.cache_is_current(&cache, &renamed));
        let mut foreign = state.clone();
        foreign.interpreter.interpreter += 1;
        assert!(!protocol.cache_is_current(&cache, &foreign));
        let mut recreated = state.clone();
        recreated.reference.namespace_token += 1;
        assert!(!protocol.cache_is_current(&cache, &recreated));
        let mut changed = state.clone();
        changed.reference.command_reference_epoch += 1;
        assert!(!protocol.cache_is_current(&cache, &changed));
        let mut dead = state.clone();
        dead.target = None;
        assert!(!protocol.cache_is_current(&cache, &dead));
        let mut dying = state;
        dying.target.as_mut().unwrap().namespace_dying = true;
        assert_eq!(
            protocol.cache_is_current(&cache, &dying),
            version == TclVersion::V8_4
        );
    }
}

#[test]
fn compiler_priming_is_distinct_from_runtime_getter_reference_selection() {
    let current = NativeCommandNameReference {
        namespace_token: 9,
        command_reference_epoch: 2,
    };
    let global = NativeCommandNameReference {
        namespace_token: 0,
        command_reference_epoch: 7,
    };
    for version in TclVersion::ALL {
        let protocol = crate::InvocationDialect::for_version(version)
            .native_command_name_protocol()
            .unwrap();
        let (cache, _) = records(version);
        let mut incoming = cache.clone();
        incoming.token += 1;
        assert_eq!(
            protocol.preserves_primed_cache(Some(&cache), false, &incoming),
            version <= TclVersion::V8_5
        );
        assert_eq!(
            protocol.preserves_primed_cache(None, true, &incoming),
            version <= TclVersion::V8_5
        );
        assert!(protocol.preserves_primed_cache(Some(&cache), false, &cache));
        assert_eq!(
            protocol.priming_reference(true, current),
            (version == TclVersion::V8_4).then_some(current)
        );
        assert_eq!(
            protocol.lookup_reference(true, current, global),
            (version == TclVersion::V8_4).then_some(global)
        );
        assert_eq!(
            protocol.lookup_reference(false, current, global),
            Some(current)
        );
        assert_eq!(
            protocol.installs_unresolved_on_miss(),
            version < TclVersion::V9_0
        );
        assert_eq!(
            protocol.invalidates_path_before_object_create(),
            version == TclVersion::V8_5
        );
    }
}
