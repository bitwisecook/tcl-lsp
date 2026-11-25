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

//! The object methods a family provides without a declaration and who can reach them.

use super::DefinitionBodyGrammar;
use super::member_effect::MemberVisibility;
use tcl_dialect::TclVersion;

/// One method every instance of a class system's objects has without the
/// class body declaring it — see
/// [`DefinitionBodyGrammar::builtin_object_methods`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinObjectMethod {
    /// The method name as dispatched (`variable`, `destroy`, `cget`).
    pub name: &'static str,
    /// Whether the object's own command exposes it.  [`MemberVisibility::
    /// Unexported`] means "reachable only through the family's self-dispatch
    /// keyword" — the same rule an `unexport`ed user method obeys.
    pub visibility: MemberVisibility,
    /// Which receiver actually carries the method.
    pub receiver: BuiltinMethodReceiver,
    /// Audited native operation of this builtin implementation. A name match
    /// does not establish that the current receiver still selects the builtin.
    pub operation: Option<BuiltinObjectMethodOperation>,
    /// One-line description, for hover / completion.
    pub detail: &'static str,
}

/// Purpose-limited successful operation of an actual native object builtin.
/// Source consumers must separately retain the receiver, dispatcher identity,
/// and absence of a current overriding method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinObjectMethodOperation {
    /// Link each supplied scalar name from the selected object's namespace
    /// into the actual calling frame. No operands is a successful no-op.
    ObjectVariableLinks,
}

impl BuiltinObjectMethodOperation {
    /// Whether one frozen operand is a valid scalar name for this operation.
    /// This supplies no object namespace or local-cell identity.
    #[must_use]
    pub fn accepts_variable_link_name(self, name: &str) -> bool {
        match self {
            Self::ObjectVariableLinks => {
                !tcl_syntax::naming::is_qualified(name.as_bytes())
                    && tcl_syntax::naming::split_array_name(name).1.is_none()
            }
        }
    }
}

/// Which kind of object a [`BuiltinObjectMethod`] lives on.
///
/// A class command is itself an object, so the two sets overlap but are not
/// the same: `Foo new` works on the class and never on one of its instances
/// (tclsh 9.0.4: `[Foo new] new` → `unknown method "new"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinMethodReceiver {
    /// Present on every object of the family, instances included.
    AnyObject,
    /// Present only on a **class** object — `TclOO`'s `new` / `create` /
    /// `createWithNamespace`, snit's `create`.
    ClassObject,
}

/// How a dispatch reaches its receiver — the axis that decides whether an
/// unexported [`BuiltinObjectMethod`] is visible at a call site.
///
/// Pinned against tclsh 9.0.4 and 8.6.16 from inside a method body of a
/// class declaring only `probe`:
///
/// ```text
/// my varname v        -> ::oo::Obj22::v      (unexported: reachable)
/// [self] varname v    -> unknown method "varname": must be destroy or probe
/// $obj varname v      -> unknown method "varname": must be destroy or probe
/// ```
///
/// `[self]` substitutes to the object's *command*, so it reaches exactly
/// what an outside caller reaches — it is [`Self::ObjectCommand`], not
/// [`Self::SelfDispatch`], despite naming the same object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodReach {
    /// Through the object's own command — `$obj m`, `[self] m`, or a
    /// bareword instance command bound by `CLASS create NAME`.  Exported
    /// members only.
    ObjectCommand,
    /// Through the family's self-dispatch keyword (`my m`), which bypasses
    /// export filtering.  Exported *and* unexported members.
    SelfDispatch,
}

impl DefinitionBodyGrammar {
    /// Whether `name` is a type-level member this family provides without
    /// the body declaring it (see [`Self::builtin_type_methods`]).
    #[must_use]
    pub fn is_builtin_type_method(&self, name: &str) -> bool {
        self.builtin_type_methods.contains(&name)
    }

    /// The built-in object method `name` names, if this family supplies one
    /// **that a `reach` dispatch can actually call** (see
    /// [`Self::builtin_object_methods`]).
    ///
    /// The whole point of routing through here rather than scanning the
    /// slice is the visibility rule: an unexported builtin answers only for
    /// [`MethodReach::SelfDispatch`], so a consumer cannot accidentally
    /// vouch for `$obj variable v`, which really is an error.
    ///
    /// [`BuiltinMethodReceiver`] is deliberately **not** filtered here —
    /// the caller decides whether it holds a class object or an instance,
    /// and a consumer that cannot tell (a `$var` whose value may be either a
    /// class command or one of its instances) must accept both rather than
    /// guess.  Read [`BuiltinObjectMethod::receiver`] off the answer when
    /// that distinction is available.
    #[must_use]
    pub fn builtin_object_method(
        &self,
        name: &str,
        reach: MethodReach,
    ) -> Option<&'static BuiltinObjectMethod> {
        let idx = self.builtin_object_methods.iter().position(|m| {
            m.name == name
                && (m.visibility == MemberVisibility::Exported
                    || reach == MethodReach::SelfDispatch)
        })?;
        Some(&self.builtin_object_methods[idx])
    }

    /// Native builtin operation available through an independently selected
    /// method reach and interpreter release. This does not select a current
    /// receiver implementation or grant command/compiler execution authority.
    #[must_use]
    pub fn builtin_method_operation(
        &self,
        name: &str,
        reach: MethodReach,
        dialect: crate::InvocationDialect,
    ) -> Option<BuiltinObjectMethodOperation> {
        if !self.profile.has_native_definition_workers()
            || reach != MethodReach::SelfDispatch
            || dialect
                .tcl_version
                .is_none_or(|version| version < TclVersion::V8_6)
        {
            return None;
        }
        self.builtin_object_method(name, reach)?.operation
    }

    /// Whether the family's built-in implementation always terminates with a
    /// non-normal completion.
    #[must_use]
    pub fn builtin_method_terminates(&self, name: &str) -> bool {
        self.builtin_terminating_methods.contains(&name)
    }
}
