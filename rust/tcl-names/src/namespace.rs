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

//! Namespace keys: the source-analysis namespace identity and the authored
//! namespace geometry of signature assistance.
//!
//! Native context equality uses original interpreter and namespace incarnation,
//! with retained component boundaries. Presentation strings never select a
//! native namespace. Source allocations remain distinct from native tokens.

use crate::command::{AllocationIncarnation, CommandAllocationSite};
use tcl_core_types::{ByteNamespacePath, NameBytes};
use tcl_runtime_api::native_compilation::NativeNamespaceContext;
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameContext, NativeNameProtocol};

/// Exact source-analysis namespace identity. An authored symbolic context
/// supplies no actual namespace token or globally callable source spelling.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceNamespaceKey {
    /// Symbolic compatibility context from a Unicode document.
    Authored(String),
    /// Actual retained namespace incarnation and component geometry.
    Native(NativeNamespaceContext),
    /// A reached source allocation, distinct from every actual native token.
    Allocated {
        /// Original allocation instruction and its source instance.
        site: CommandAllocationSite,
        /// Distinct bounded executions of the allocation instruction.
        incarnation: AllocationIncarnation,
        /// Constructed child path retained before rendering.
        path: ByteNamespacePath,
    },
}

impl SourceNamespaceKey {
    /// Retain an authored symbolic context; this does not parse a native path.
    #[must_use]
    pub fn authored(namespace: impl Into<String>) -> Self {
        Self::Authored(namespace.into())
    }

    /// Authored context for retained C namespace components. `None` when a
    /// component is not UTF-8 or would not survive the authored splitter; the
    /// key comes from the components, never from a rendered presentation.
    #[must_use]
    pub fn authored_path(path: &ByteNamespacePath) -> Option<Self> {
        tcl_syntax::naming::checked_namespace_path_key(path).map(Self::Authored)
    }

    /// Authored context for Jim's counted flat namespace value. `None` when
    /// the value is not UTF-8.
    #[must_use]
    pub fn authored_jim_namespace(value: &NameBytes) -> Option<Self> {
        Some(Self::Authored(tcl_syntax::naming::root_unrooted_key(
            value.try_utf8().ok()?,
        )))
    }

    /// Authored context for a structured signature scope, built from its
    /// retained components or counted bytes rather than its display text.
    #[must_use]
    pub fn authored_scope(scope: &SignatureNamespaceScope) -> Option<Self> {
        match scope {
            SignatureNamespaceScope::Symbolic(key) => Some(Self::authored(key.clone())),
            SignatureNamespaceScope::C(path) => Self::authored_path(path),
            SignatureNamespaceScope::Jim(value) => Self::authored_jim_namespace(value),
        }
    }

    /// Retain the actual current namespace from an immutable runtime entry.
    ///
    /// # Errors
    /// Returns unavailable provenance for a missing or conflicting namespace row.
    pub fn from_native_entry(
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self, tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable> {
        entry
            .retained_namespace_context(entry.current_namespace)
            .map(Self::Native)
    }

    /// Exact native or allocated component geometry. Authored symbolic labels
    /// cannot donate boundaries by reparsing their presentation.
    #[must_use]
    pub fn exact_native_path(&self) -> Option<&ByteNamespacePath> {
        match self {
            Self::Native(context) => Some(&context.path),
            Self::Allocated { path, .. } => Some(path),
            Self::Authored(_) => None,
        }
    }

    /// Optional compatibility key whose analytical splitter agrees with the
    /// retained geometry. Even this view carries no callable-name authority.
    #[doc(hidden)]
    #[must_use]
    pub fn advisory_key(&self) -> Option<String> {
        let display = self.display()?;
        if let Some(path) = self.exact_native_path() {
            let components = tcl_syntax::naming::checked_namespace_path_utf8(path).ok()?;
            if tcl_syntax::naming::key_segments(&display) != components {
                return None;
            }
        }
        Some(display)
    }

    /// Retained compiled namespace geometry without a displayed-name lookup.
    #[must_use]
    pub fn to_compiled_context(&self) -> Option<tcl_runtime_api::CompiledNamespaceContext> {
        match self {
            Self::Authored(_) => None,
            Self::Native(context) => Some(tcl_runtime_api::CompiledNamespaceContext::Native(
                context.clone(),
            )),
            Self::Allocated { path, .. } => Some(
                tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(path.clone()),
            ),
        }
    }

    /// Namespace geometry for a source artifact. Authored rooted coordinates
    /// remain distinct from an actual native namespace incarnation.
    #[must_use]
    pub fn source_artifact_path(&self) -> Option<ByteNamespacePath> {
        match self {
            Self::Authored(coordinate) => {
                let unrooted = tcl_syntax::naming::unroot_rooted_key(coordinate)?;
                Some(ByteNamespacePath::from_segments(
                    tcl_syntax::naming::key_segments(unrooted),
                ))
            }
            _ => self.exact_native_path().cloned(),
        }
    }

    /// Actual retained native context, excluding source-created allocations.
    #[must_use]
    pub fn native_context(&self) -> Option<&NativeNamespaceContext> {
        if let Self::Native(context) = self {
            Some(context)
        } else {
            None
        }
    }

    /// Exact root geometry from the same retained native namespace incarnation.
    /// Source labels and constructed paths cannot supply this native context.
    #[must_use]
    pub fn is_native_root_in_entry(&self, entry: &tcl_runtime_api::NativeCompilationEntry) -> bool {
        self.native_context().is_some_and(|context| {
            context.path.is_root()
                && entry.retained_namespace_context(context.token).as_ref() == Ok(context)
        })
    }

    #[doc(hidden)]
    #[must_use]
    pub fn is_root(&self) -> bool {
        match self {
            Self::Authored(text) => text == "::",
            _ => self
                .exact_native_path()
                .is_some_and(ByteNamespacePath::is_root),
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn is_descendant_of(&self, parent: &Self) -> bool {
        match (self, parent) {
            (Self::Authored(child), Self::Authored(parent)) => {
                child.starts_with(&format!("{parent}::"))
            }
            _ => match (self.exact_native_path(), parent.exact_native_path()) {
                (Some(child), Some(parent)) => {
                    child.as_segments().len() > parent.as_segments().len()
                        && child.as_segments().starts_with(parent.as_segments())
                }
                _ => false,
            },
        }
    }

    /// Optional Unicode presentation, with no lookup or callable-name authority.
    #[must_use]
    pub fn display(&self) -> Option<String> {
        match self {
            Self::Authored(text) => Some(text.clone()),
            Self::Native(context) => display_path(&context.path),
            Self::Allocated { path, .. } => display_path(path),
        }
    }
}

fn display_path(path: &ByteNamespacePath) -> Option<String> {
    String::from_utf8(tcl_syntax::naming::native_namespace_full_name_bytes(path)).ok()
}

impl From<String> for SourceNamespaceKey {
    fn from(value: String) -> Self {
        Self::authored(value)
    }
}
impl From<&str> for SourceNamespaceKey {
    fn from(value: &str) -> Self {
        Self::authored(value)
    }
}

impl Default for SourceNamespaceKey {
    fn default() -> Self {
        Self::authored("::")
    }
}

/// Source namespace geometry. These values supply no namespace existence,
/// entered frame, command identity or native compilation permission.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignatureNamespaceScope {
    /// Explicit symbolic compatibility context without an audited name policy.
    Symbolic(String),
    /// C namespace components retained before any display rendering.
    C(ByteNamespacePath),
    /// Jim's counted flat namespace value, independent of C components.
    Jim(NameBytes),
}

impl SignatureNamespaceScope {
    /// Whether the retained source geometry is the selected provider's root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        match self {
            Self::C(path) => path.is_root(),
            Self::Jim(value) => value.as_bytes().is_empty(),
            Self::Symbolic(key) => key == "::",
        }
    }

    /// Readonly label from retained geometry, including opaque native bytes.
    /// The result grants no component reconstruction, lookup or source spelling.
    #[must_use]
    pub fn reporting_label(&self) -> String {
        match self {
            Self::Symbolic(label) => label.clone(),
            Self::C(path) => tcl_syntax::native_string::resident_name_label(
                &tcl_syntax::naming::native_namespace_full_name_bytes(path),
            ),
            Self::Jim(value) => tcl_syntax::naming::root_unrooted_key(
                &tcl_syntax::native_string::resident_name_label(value.as_bytes()),
            ),
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn root(policy: Option<NamePolicyProtocol>) -> Self {
        match policy.map(NamePolicyProtocol::recipe) {
            Some(NativeNameProtocol::C(_)) => Self::C(ByteNamespacePath::root()),
            Some(NativeNameProtocol::Jim084) => Self::Jim(NameBytes::from(b"".as_slice())),
            None => Self::Symbolic("::".to_owned()),
        }
    }

    /// Presentation only; its colon boundaries cannot recover this scope.
    #[must_use]
    pub fn display(&self) -> Option<String> {
        match self {
            Self::Symbolic(key) => Some(key.clone()),
            Self::C(path) => {
                String::from_utf8(tcl_syntax::naming::native_namespace_full_name_bytes(path)).ok()
            }
            Self::Jim(value) => Some(tcl_syntax::naming::root_unrooted_key(
                value.try_utf8().ok()?,
            )),
        }
    }

    /// Retained projection context only; it grants no namespace existence or token.
    #[must_use]
    pub fn context(&self) -> Option<NativeNameContext<'_>> {
        match self {
            Self::C(path) => Some(NativeNameContext::new(path)),
            Self::Jim(value) => Some(NativeNameContext::with_jim_namespace(
                &ROOT,
                value.as_bytes(),
            )),
            Self::Symbolic(_) => None,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn context_for_policy(&self, policy: NamePolicyProtocol) -> Option<NativeNameContext<'_>> {
        match (self, policy.recipe()) {
            (Self::C(_), NativeNameProtocol::C(_)) | (Self::Jim(_), NativeNameProtocol::Jim084) => {
                self.context()
            }
            _ => None,
        }
    }

    /// Strict namespace ancestry from retained components. Unsupported Jim
    /// parents remain unknown; rendered labels never supply boundaries.
    #[must_use]
    pub fn is_strict_ancestor_of(&self, child: &Self, policy: NamePolicyProtocol) -> Option<bool> {
        match (self, child, policy.recipe()) {
            (Self::C(parent), Self::C(child), NativeNameProtocol::C(_)) => Some(
                parent.as_segments().len() < child.as_segments().len()
                    && child.as_segments().starts_with(parent.as_segments()),
            ),
            (Self::Jim(parent), Self::Jim(child), recipe @ NativeNameProtocol::Jim084) => {
                let mut current = child.as_bytes();
                loop {
                    if current == parent.as_bytes() {
                        return Some(current != child.as_bytes());
                    }
                    if current.is_empty() {
                        return Some(false);
                    }
                    if current.contains(&0) {
                        return None;
                    }
                    let next = recipe.namespace_qualifier_bytes(current);
                    if next == current {
                        return None;
                    }
                    current = next;
                }
            }
            _ => None,
        }
    }

    /// Optional written namespace spelling checked against the retained geometry.
    #[must_use]
    pub fn source_spelling(&self, policy: Option<NamePolicyProtocol>) -> Option<String> {
        match (self, policy.map(NamePolicyProtocol::recipe)) {
            (Self::Symbolic(key), None) => Some(key.clone()),
            (Self::C(path), Some(recipe @ NativeNameProtocol::C(_))) => {
                tcl_syntax::naming::native_namespace_source_spelling(recipe, path)
            }
            (Self::Jim(value), Some(NativeNameProtocol::Jim084)) => {
                tcl_syntax::naming::native_jim_namespace_source_spelling(value.as_bytes())
            }
            _ => None,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn child(&self, written: &str, policy: Option<NamePolicyProtocol>) -> Option<Self> {
        let Some(policy) = policy else {
            let Self::Symbolic(namespace) = self else {
                return None;
            };
            return Some(Self::Symbolic(tcl_syntax::naming::qualify_namespace(
                namespace, written,
            )));
        };
        let recipe = policy.recipe();
        match self {
            Self::C(_) => Some(Self::C(
                recipe
                    .namespace_address_path(self.context()?, written.as_bytes())
                    .ok()?,
            )),
            Self::Jim(_) => Some(Self::Jim(
                recipe
                    .jim_namespace_canonical_input(self.context()?, written.as_bytes())
                    .ok()?
                    .selected()
                    .into(),
            )),
            Self::Symbolic(_) => None,
        }
    }
}

static ROOT: ByteNamespacePath = ByteNamespacePath::root();
