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

#[test]
fn original_jim_command_key_constructors_keep_independent_capture_purposes() {
    // naming.alias.original-command-key-replacement-child-capture-and-rename
    // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
    // Constructor selection is a pure recipe, not object ownership or a
    // grant to install a command, evaluate a prefix or recover a context.
    let path = ByteNamespacePath::root();
    let nested = NativeNameContext::with_jim_namespace(&path, b"actual_namespace");
    let jim = NativeNameProtocol::Jim084;
    assert_eq!(
        jim.jim_command_key_construction(nested, b"name", NativeNamePurpose::AliasPublication),
        Ok(NativeJimNamespaceConstruction::RetainOriginal)
    );
    for purpose in [
        NativeNamePurpose::ChildAliasPublication,
        NativeNamePurpose::CommandCApiPublication,
    ] {
        assert_eq!(
            jim.jim_command_key_construction(nested, b"name", purpose),
            Ok(NativeJimNamespaceConstruction::FreshString)
        );
    }
    for purpose in [
        NativeNamePurpose::CommandPublication,
        NativeNamePurpose::RenameSource,
        NativeNamePurpose::RenameDestination,
    ] {
        assert_eq!(
            jim.jim_command_key_construction(nested, b"name", purpose),
            Ok(NativeJimNamespaceConstruction::DuplicateNamespaceAndAppend)
        );
        assert_eq!(
            jim.jim_command_key_construction(nested, b"::name", purpose),
            Ok(NativeJimNamespaceConstruction::RetainOriginal)
        );
        assert_eq!(
            jim.jim_command_key_construction(NativeNameContext::root(), b"name", purpose),
            Ok(NativeJimNamespaceConstruction::RetainOriginal)
        );
        assert_eq!(
            jim.jim_command_key_construction(NativeNameContext::new(&path), b"name", purpose),
            Err(NameProjectionUnavailable::MissingJimNamespaceObject)
        );
        assert_eq!(
            NativeNameProtocol::C(tcl_dialect::TclVersion::V9_0)
                .jim_command_key_construction(nested, b"name", purpose),
            Err(NameProjectionUnavailable::PurposeNotModelled)
        );
    }
    assert_eq!(
        jim.jim_command_key_construction(nested, b"name", NativeNamePurpose::PackageName),
        Err(NameProjectionUnavailable::PurposeNotModelled)
    );
}
