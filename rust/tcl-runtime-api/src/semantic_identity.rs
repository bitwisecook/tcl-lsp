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

//! Common token vocabulary for the existing engine snapshot adapters.

use crate::native_compilation::{
    NativeCompilationBinding, NativeInterpreterIdentity, NativeNamespaceContext,
};
pub use tcl_core_types::{
    BodyUnitRef, CommandRef, FrameRef, IdentityDomain, IdentityKey, IdentityProvenance,
    InstanceRef, InterpreterRef, NamespaceRef, PackageRef, SemanticRef, VariablePlaceRef,
};

impl NativeInterpreterIdentity {
    /// Retain this interpreter's actual token, never its selector or display name.
    #[must_use]
    pub fn interpreter_ref(self) -> InterpreterRef {
        InterpreterRef::from_key(self.semantic_key(self.interpreter, 0))
    }

    /// Retain an already selected non-reused command token in this interpreter.
    #[must_use]
    pub fn command_ref(self, token: u64) -> CommandRef {
        CommandRef::from_key(self.semantic_key(token, 0))
    }

    fn semantic_key(self, token: u64, generation: u64) -> IdentityKey {
        IdentityKey {
            domain: IdentityDomain::Runtime {
                owner: self.owner,
                interpreter: self.interpreter,
            },
            token,
            generation,
            issuer: None,
        }
    }
}

/// Read identity from an already resolved engine handle. This performs no
/// name lookup and grants no invocation or current-lifetime authority.
pub trait CommandIdentityTokens {
    /// Retain this engine's actual interpreter identity.
    fn interpreter_ref(&self) -> InterpreterRef;
    /// Project an issued handle to its original command allocation token.
    fn command_ref(&self, command: crate::CommandId) -> tcl_core_types::Answer<CommandRef>;
}

impl NativeCompilationBinding {
    /// Command allocation exposed by either engine's original snapshot owner.
    /// Moving its slot leaves this reference unchanged; a recreated command has
    /// a different non-reused token. No lookup or invocation is performed here.
    #[must_use]
    pub fn command_ref(&self, interpreter: NativeInterpreterIdentity) -> CommandRef {
        CommandRef::from_key(interpreter.semantic_key(self.token, 0))
    }

    /// The separately versioned implementation of the retained command token.
    #[must_use]
    pub fn command_implementation_ref(&self, interpreter: NativeInterpreterIdentity) -> CommandRef {
        CommandRef::from_key(interpreter.semantic_key(self.token, self.implementation_generation))
    }
}

impl NativeNamespaceContext {
    /// Retain the namespace incarnation independently of its rendered path.
    #[must_use]
    pub fn namespace_ref(&self) -> NamespaceRef {
        NamespaceRef::from_key(self.interpreter.semantic_key(self.token, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{NativeCommandImplementation, NativeCompilerHookPresence};
    use tcl_core_types::{ByteCommandSlot, ByteNamespacePath};

    fn binding(token: u64) -> NativeCompilationBinding {
        NativeCompilationBinding {
            slot: ByteCommandSlot::new(ByteNamespacePath::root(), "p".into()),
            namespace_token: 1,
            token,
            implementation_generation: 1,
            implementation: NativeCommandImplementation::Opaque,
            compiler_hook: NativeCompilerHookPresence::Absent,
            compiler: None,
            procedure_header: None,
            has_execution_trace: false,
        }
    }

    #[test]
    fn rename_keeps_the_token_but_recreation_and_owners_split() {
        let interpreter = NativeInterpreterIdentity {
            owner: 12,
            interpreter: 3,
        };
        let original = binding(7);
        let mut moved = original.clone();
        moved.slot.simple = "moved".into();
        assert_eq!(
            original.command_ref(interpreter),
            moved.command_ref(interpreter)
        );
        assert_ne!(
            original.command_ref(interpreter),
            binding(8).command_ref(interpreter)
        );
        assert_ne!(
            original.command_ref(interpreter),
            original.command_ref(NativeInterpreterIdentity {
                owner: 13,
                ..interpreter
            })
        );
        moved.implementation_generation += 1;
        assert_eq!(
            original.command_ref(interpreter),
            moved.command_ref(interpreter)
        );
        assert_ne!(
            original.command_implementation_ref(interpreter),
            moved.command_implementation_ref(interpreter)
        );
    }
}
