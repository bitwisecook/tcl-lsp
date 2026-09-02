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

//! The target-neutral **native lowering** descriptor.
//!
//! A sibling of [`crate::CommandSpec::lowering_hook`],
//! [`crate::CommandSpec::codegen_hook`], and
//! [`crate::CommandSpec::inline_codegen_hook`]: it says which *shape* of
//! native code a registry-resolved invocation lowers to once the common
//! executable IR has been built (`docs/design/compiler/wasm-native-lowering-plan.md`
//! §3.3). The compiler keeps the implementations; the registry keeps the
//! catalogue, so no emitter ever selects a native shape by command name.
//!
//! Every shape is a *candidate*, never a proof: the dispatch-stability,
//! trace, and representation proofs still decide whether the native shape is
//! taken or the generic argv invocation is kept.

use crate::completion::CompletionCode;
use crate::hooks::LoweringHookId;
use crate::intrinsic::IntrinsicId;

/// The native shape a registry-described command invocation lowers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeLowering {
    /// A pure or read-only value operation: the arguments are values, the
    /// result is a value, and the runtime intrinsic performs the operation.
    Intrinsic {
        /// The target-neutral intrinsic identity.
        id: IntrinsicId,
        /// The argument count (after the command head) the native shape
        /// accepts; any other count keeps the generic invocation.
        arity: ArityRule,
    },
    /// A read-modify-write of one variable cell: the first argument is a
    /// place, the rest are values, and the runtime operates on the cell's
    /// object in place with copy-on-write.
    CellReadModifyWrite(CellUpdate),
    /// A structural operation already described by a common lowering hook,
    /// projected into executable edges by the executable IR.
    Structured(LoweringHookId),
    /// A command whose whole effect is to complete with a fixed code
    /// (`break`, `continue`): the native shape is the completion itself.
    Completion(CompletionCode),
    /// A scope declaration that links a local cell to another frame's cell.
    Scope(ScopeKind),
    /// A definition-time command (`proc`) whose body the compiler may compile
    /// separately while the runtime binds the source form.
    Definition,
    /// Everything else: a generic argv invocation through runtime dispatch.
    Generic,
}

impl NativeLowering {
    /// Stable compiler and Explorer spelling of the shape family.
    #[must_use]
    pub const fn kind_str(self) -> &'static str {
        match self {
            Self::Intrinsic { .. } => "intrinsic",
            Self::CellReadModifyWrite(_) => "cell-read-modify-write",
            Self::Structured(_) => "structured",
            Self::Completion(_) => "completion",
            Self::Scope(_) => "scope",
            Self::Definition => "definition",
            Self::Generic => "generic",
        }
    }
}

/// The post-head argument count a native intrinsic shape accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArityRule {
    /// Exactly this many arguments.
    Exact(u8),
    /// An inclusive range of argument counts.
    Range {
        /// Fewest accepted arguments.
        min: u8,
        /// Most accepted arguments.
        max: u8,
    },
    /// At least this many arguments.
    AtLeast(u8),
}

impl ArityRule {
    /// Whether `count` post-head arguments satisfy the rule.
    #[must_use]
    pub const fn accepts(self, count: usize) -> bool {
        match self {
            Self::Exact(n) => count == n as usize,
            Self::Range { min, max } => count >= min as usize && count <= max as usize,
            Self::AtLeast(n) => count >= n as usize,
        }
    }
}

/// Which in-place cell update a [`NativeLowering::CellReadModifyWrite`]
/// performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellUpdate {
    /// `incr`: add an integer to the cell's numeric value.
    Increment,
    /// `append`: append string values to the cell's string.
    Append,
    /// `lappend`: append list elements to the cell's list.
    ListAppend,
}

impl CellUpdate {
    /// Stable compiler and Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Increment => "increment",
            Self::Append => "append",
            Self::ListAppend => "list-append",
        }
    }
}

/// Which frame the linked cell of a [`NativeLowering::Scope`] lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    /// `global`: the global namespace's cell.
    Global,
    /// `variable`: the current namespace's cell.
    NamespaceVariable,
    /// `upvar`: a cell in a caller's frame.
    Upvar,
    /// `namespace upvar`: a cell in a named namespace.
    NamespaceUpvar,
}

impl ScopeKind {
    /// Stable compiler and Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::NamespaceVariable => "namespace-variable",
            Self::Upvar => "upvar",
            Self::NamespaceUpvar => "namespace-upvar",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommandRegistry;

    #[test]
    fn arity_rules_accept_their_documented_counts() {
        assert!(ArityRule::Exact(0).accepts(0));
        assert!(!ArityRule::Exact(0).accepts(1));
        assert!(ArityRule::Range { min: 1, max: 3 }.accepts(1));
        assert!(ArityRule::Range { min: 1, max: 3 }.accepts(3));
        assert!(!ArityRule::Range { min: 1, max: 3 }.accepts(4));
        assert!(ArityRule::AtLeast(2).accepts(7));
        assert!(!ArityRule::AtLeast(2).accepts(1));
    }

    /// The drift gate: every command the native tier lowers by descriptor
    /// carries one, and the descriptor agrees with the common lowering hook
    /// or intrinsic the command already declares. A command that gains one of
    /// these hooks without a native shape falls back to generic invocation
    /// silently, which is exactly the drift this test exists to catch.
    #[test]
    fn native_tier_commands_carry_a_native_lowering_descriptor() {
        let registry = CommandRegistry::build_default();
        let structured = [
            ("set", LoweringHookId::Set),
            ("expr", LoweringHookId::Expr),
            ("if", LoweringHookId::If),
            ("while", LoweringHookId::While),
            ("for", LoweringHookId::For),
            ("return", LoweringHookId::Return),
        ];
        for (name, hook) in structured {
            let spec = registry.get(name).expect(name);
            assert_eq!(spec.lowering_hook, Some(hook), "{name}");
            assert_eq!(
                spec.native_lowering,
                Some(NativeLowering::Structured(hook)),
                "{name} must lower natively through its structural hook"
            );
        }
        let cells = [
            ("incr", LoweringHookId::Incr, CellUpdate::Increment),
            (
                "append",
                LoweringHookId::AppendOrLappend,
                CellUpdate::Append,
            ),
            (
                "lappend",
                LoweringHookId::AppendOrLappend,
                CellUpdate::ListAppend,
            ),
        ];
        for (name, hook, update) in cells {
            let spec = registry.get(name).expect(name);
            assert_eq!(spec.lowering_hook, Some(hook), "{name}");
            assert_eq!(
                spec.native_lowering,
                Some(NativeLowering::CellReadModifyWrite(update)),
                "{name} must lower natively as a cell read-modify-write"
            );
        }
        for (name, code) in [
            ("break", CompletionCode::Break),
            ("continue", CompletionCode::Continue),
        ] {
            let spec = registry.get(name).expect(name);
            assert_eq!(
                spec.native_lowering,
                Some(NativeLowering::Completion(code)),
                "{name} must lower natively to its completion code"
            );
        }
        let puts = registry.get("puts").expect("puts");
        assert_eq!(
            puts.semantic_operation,
            Some(crate::SemanticOperationId::Intrinsic(
                IntrinsicId::ChannelWrite
            ))
        );
        assert!(
            matches!(
                puts.native_lowering,
                Some(NativeLowering::Intrinsic {
                    id: IntrinsicId::ChannelWrite,
                    ..
                })
            ),
            "puts must lower natively as the channel-write intrinsic"
        );
        assert_eq!(
            registry.get("proc").expect("proc").native_lowering,
            Some(NativeLowering::Definition)
        );
        for (name, kind) in [
            ("global", ScopeKind::Global),
            ("variable", ScopeKind::NamespaceVariable),
            ("upvar", ScopeKind::Upvar),
        ] {
            assert_eq!(
                registry.get(name).expect(name).native_lowering,
                Some(NativeLowering::Scope(kind)),
                "{name}"
            );
        }
    }

    /// A fixed-completion command touches nothing but its completion code:
    /// its resolved effect footprint is closed and barrier-free, so a loop
    /// body containing `break`/`continue` keeps its dispatch proofs.
    #[test]
    fn completion_commands_have_a_closed_effect_footprint() {
        let registry = CommandRegistry::build_default();
        for name in ["break", "continue"] {
            let facts = registry
                .resolve_structured_invocation(crate::InvocationWords::literals(name, &[]), None)
                .resolved()
                .unwrap_or_else(|| panic!("{name} resolves"))
                .facts();
            assert!(!facts.effects.requires_world_barrier(), "{name}");
            assert!(
                facts.effects.accesses().iter().all(|access| access.domain
                    != crate::world_effect::WorldStateDomain::InterpreterPolicy),
                "{name}: {:?}",
                facts.effects.accesses()
            );
        }
    }

    /// A descriptor is stamped only where a native shape is implemented;
    /// everything else is the generic invocation, stated once here rather
    /// than defaulted silently somewhere else.
    #[test]
    fn undeclared_commands_lower_generically() {
        let registry = CommandRegistry::build_default();
        for name in ["string", "list", "catch", "foreach", "switch"] {
            let spec = registry.get(name).expect(name);
            assert_eq!(spec.native_lowering, None, "{name}");
            assert_eq!(spec.native_lowering(), NativeLowering::Generic, "{name}");
        }
        assert_eq!(NativeLowering::Generic.kind_str(), "generic");
    }
}
