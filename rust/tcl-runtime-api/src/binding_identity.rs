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

//! The compiler's assumptions about one runtime command or procedure binding.

use super::native_compilation;
use tcl_core_types::ByteNamespacePath;

/// A compiler assumption about one runtime command binding.
///
/// `resolution_namespace` is the unrooted constructed namespace key at the
/// source binding site; `name` is the spelling whose live resolution must be
/// checked there (and may be an alias); `identity` is the registry command
/// implementation the specialised operation was compiled for. Keeping all
/// three parts makes aliases and inlined cross-namespace bodies first-class
/// without teaching a runtime which source names happen to compile specially.
/// Boundary at which a compiled native binding requirement is checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandBindingGuard {
    /// Native Tcl 8.4 compiled hooks capture the selected implementation when
    /// the chunk is admitted; later script mutations do not change that opcode.
    ChunkEntry,
    /// Modern Tcl and Jim check the selected implementation before this
    /// command's argument substitutions execute.
    BeforeArguments,
}

/// Exact lookup context retained by an executable compiler dependency.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompiledNamespaceContext {
    /// An original native namespace owner and incarnation.
    Native(native_compilation::NativeNamespaceContext),
    /// Exact geometry from a source allocation which has no actual token yet.
    ConstructedPath(ByteNamespacePath),
}

impl CompiledNamespaceContext {
    /// Exact component boundaries, independent of all presentation strings.
    #[must_use]
    pub fn path(&self) -> &ByteNamespacePath {
        match self {
            Self::Native(context) => &context.path,
            Self::ConstructedPath(path) => path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandBindingIdentity {
    /// Actual namespace owner and incarnation for a native compilation site.
    /// When present, this is authoritative; the text field is presentation only.
    pub namespace_context: Option<CompiledNamespaceContext>,
    /// Constructed namespace key in which the source binding resolved.
    ///
    /// This uses the runtime ABI's unrooted representation: `""` is global,
    /// `"n"` is `::n`, and every byte of a literal-colon namespace segment is
    /// otherwise preserved.
    pub resolution_namespace: String,
    /// Source binding to resolve in [`Self::resolution_namespace`].
    pub name: String,
    /// Stable registry identity expected at that binding.
    pub identity: String,
    /// Validation boundary selected by the native compilation policy.
    pub guard: CommandBindingGuard,
}

impl CommandBindingIdentity {
    /// Construct a binding requirement in the global namespace.
    #[must_use]
    pub fn new(name: impl Into<String>, identity: impl Into<String>) -> Self {
        Self {
            namespace_context: None,
            resolution_namespace: String::new(),
            name: name.into(),
            identity: identity.into(),
            guard: CommandBindingGuard::BeforeArguments,
        }
    }

    /// Construct a binding requirement in an unrooted constructed namespace.
    #[must_use]
    pub fn in_namespace(
        resolution_namespace: impl Into<String>,
        name: impl Into<String>,
        identity: impl Into<String>,
    ) -> Self {
        Self {
            namespace_context: None,
            resolution_namespace: resolution_namespace.into(),
            name: name.into(),
            identity: identity.into(),
            guard: CommandBindingGuard::BeforeArguments,
        }
    }

    /// Retain the actual namespace incarnation used by native source lookup.
    #[must_use]
    pub fn with_native_namespace(
        mut self,
        context: Option<native_compilation::NativeNamespaceContext>,
    ) -> Self {
        self.namespace_context = context.map(CompiledNamespaceContext::Native);
        self
    }

    /// Retain an exact native or source-allocated lookup context.
    #[must_use]
    pub fn with_namespace_context(mut self, context: Option<CompiledNamespaceContext>) -> Self {
        self.namespace_context = context;
        self
    }

    /// Select the native admission/dispatch boundary for this requirement.
    #[must_use]
    pub fn with_guard(mut self, guard: CommandBindingGuard) -> Self {
        self.guard = guard;
        self
    }

    /// Construct a binding requirement from the compiler's rooted constructed
    /// namespace representation.
    ///
    /// Exactly one global-root marker is removed. This must not use written
    /// Tcl name canonicalisation: an unrooted key may itself begin with colons
    /// because they can be literal namespace-segment bytes.
    #[must_use]
    pub fn in_rooted_namespace(
        resolution_namespace: &str,
        name: impl Into<String>,
        identity: impl Into<String>,
    ) -> Self {
        Self::in_namespace(
            tcl_syntax::naming::unroot_rooted_key(resolution_namespace)
                .unwrap_or(resolution_namespace),
            name,
            identity,
        )
    }
}

/// A compiler assumption about one exact user-procedure binding.
///
/// Unlike [`CommandBindingIdentity`], this is not a registry implementation:
/// `resolution_namespace` and `invocation_name` identify the source binding
/// whose call the compiler erased, while `name` is the canonical rooted
/// constructed command key selected there. `parameters` and `body` identify
/// the source definition copied into the caller. A runtime may execute that
/// caller only while the source binding still resolves to that exact command
/// key and it still holds an equivalent user procedure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcedureBindingIdentity {
    /// Actual source lookup namespace, independent of its printed spelling.
    pub namespace_context: Option<CompiledNamespaceContext>,
    /// Constructed namespace key in which the source invocation resolved.
    ///
    /// This uses the runtime ABI's unrooted representation: `""` is global,
    /// `"n"` is `::n`, and literal namespace-segment bytes are preserved.
    pub resolution_namespace: String,
    /// Source invocation to resolve in [`Self::resolution_namespace`].
    pub invocation_name: String,
    /// Canonical rooted constructed procedure name (for example `::ns::p`).
    pub name: String,
    /// Raw formal-parameter list value, including defaults.
    pub parameters: String,
    /// Raw procedure body value copied by the inliner.
    pub body: String,
}

impl ProcedureBindingIdentity {
    /// Construct an exact user-procedure binding requirement in the global
    /// namespace.
    #[must_use]
    pub fn new(
        invocation_name: impl Into<String>,
        name: impl Into<String>,
        parameters: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::in_namespace("", invocation_name, name, parameters, body)
    }

    /// Construct an exact user-procedure binding requirement in an unrooted
    /// constructed namespace.
    #[must_use]
    pub fn in_namespace(
        resolution_namespace: impl Into<String>,
        invocation_name: impl Into<String>,
        name: impl Into<String>,
        parameters: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            namespace_context: None,
            resolution_namespace: resolution_namespace.into(),
            invocation_name: invocation_name.into(),
            name: name.into(),
            parameters: parameters.into(),
            body: body.into(),
        }
    }

    /// Construct a requirement from the compiler's rooted constructed
    /// namespace representation.
    ///
    /// Exactly one global-root marker is removed. Written Tcl name
    /// canonicalisation must not be applied to a constructed namespace key.
    #[must_use]
    pub fn in_rooted_namespace(
        resolution_namespace: &str,
        invocation_name: impl Into<String>,
        name: impl Into<String>,
        parameters: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::in_namespace(
            tcl_syntax::naming::unroot_rooted_key(resolution_namespace)
                .unwrap_or(resolution_namespace),
            invocation_name,
            name,
            parameters,
            body,
        )
    }

    /// Retain the actual namespace incarnation used by the source invocation.
    #[must_use]
    pub fn with_native_namespace(
        mut self,
        context: Option<native_compilation::NativeNamespaceContext>,
    ) -> Self {
        self.namespace_context = context.map(CompiledNamespaceContext::Native);
        self
    }

    /// Retain an exact native or source-allocated lookup context.
    #[must_use]
    pub fn with_namespace_context(mut self, context: Option<CompiledNamespaceContext>) -> Self {
        self.namespace_context = context;
        self
    }
}
