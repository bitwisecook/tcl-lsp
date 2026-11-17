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

//! Static layout projections of one immutable lookup point.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{SourceInvocationBinding, SourceNamespaceKey};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct LayoutProjectionKey {
    pub(super) namespace: SourceNamespaceKey,
    pub(super) head: tcl_lexer::NativeWord,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) dialect: tcl_registry::InvocationDialect,
    pub(super) logical_source: bool,
}

/// A projection reads only its point's retained state and the key, so one
/// answer serves every declaration row that shares the point. A detached
/// clone or a joined point starts empty; the memo is not semantic state.
#[derive(Debug, Default)]
pub(super) struct LayoutProjectionMemo {
    projections: Mutex<HashMap<LayoutProjectionKey, Option<Arc<SourceInvocationBinding>>>>,
}

impl Clone for LayoutProjectionMemo {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl LayoutProjectionMemo {
    pub(super) fn get_or_project(
        &self,
        key: LayoutProjectionKey,
        project: impl FnOnce(&LayoutProjectionKey) -> Option<SourceInvocationBinding>,
    ) -> Option<Arc<SourceInvocationBinding>> {
        if let Some(known) = self.projections.lock().unwrap().get(&key) {
            return known.clone();
        }
        let projected = project(&key).map(Arc::new);
        self.projections
            .lock()
            .unwrap()
            .entry(key)
            .or_insert(projected)
            .clone()
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.projections.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_memoised_projection_is_the_projection_and_is_shared_by_the_point() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit =
            crate::compilation_unit::CompilationUnit::build_for("set x value", registry, false);
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .iter()
            .filter_map(|statement| {
                unit.ir_module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
            })
            .find(|tokens| tokens.argv_texts.first().is_some_and(|head| head == "set"))
            .unwrap()
            .clone();
        let binding = tokens.source_binding.as_ref().unwrap();
        let snapshot = binding.lookup_state.as_ref().unwrap();
        let site = binding.invocation_site().unwrap();
        let config = binding.original_lexer_config_for_tokens(&tokens).unwrap();
        let head = site
            .source
            .original_lexical_command(tokens.words(), site.offset, config)
            .unwrap()
            .native_words()
            .first()
            .unwrap()
            .clone();
        let key = LayoutProjectionKey {
            namespace: binding.lookup_namespace_key.clone(),
            head,
            config,
            dialect: snapshot.state.source_variables.invocation_dialect.unwrap(),
            logical_source: false,
        };
        let direct = |key: &LayoutProjectionKey| {
            super::super::original_static_layout_projection(
                snapshot,
                &key.namespace,
                &key.head,
                key.config,
                key.dialect,
                key.logical_source,
            )
        };
        let memo = LayoutProjectionMemo::default();
        let first = memo.get_or_project(key.clone(), direct).unwrap();
        assert_eq!(Some(first.as_ref().clone()), direct(&key));
        let second = memo
            .get_or_project(key, |_| unreachable!("a second projection"))
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(memo.len(), 1);
        assert_eq!(memo.clone().len(), 0);
    }
}
