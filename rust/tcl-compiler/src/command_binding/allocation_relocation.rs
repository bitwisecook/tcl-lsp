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

//! Moving the command allocation sites a lowered script carries into another
//! source instance that holds the same bytes at a shifted offset.

use std::sync::Arc;

use super::{
    CommandAllocationSite, SourceInvocationBinding, SourceOriginId, SourceVariableEvaluationOwner,
    compiled_invocation::SourceNativeCompilerAdmission,
};
use crate::ir::{CommandTokens, Script};

impl CommandTokens {
    /// Visit the allocation site of every invocation these tokens retain for a
    /// native consumer: the dispatch site, each compiled and named candidate,
    /// the admitted compiler selection, and the owners of the variable reads.
    pub fn for_each_allocation_site_mut(
        &mut self,
        visit: &mut impl FnMut(&mut CommandAllocationSite),
    ) {
        if let Some(binding) = &mut self.source_binding {
            binding.for_each_allocation_site_mut(visit);
        }
        if !self.nested_bindings.is_empty() {
            for (_, binding) in self.nested_bindings_mut() {
                binding.for_each_allocation_site_mut(visit);
            }
        }
        for access in &mut self.variable_accesses {
            for_each_owner_site_mut(&mut access.owner, visit);
        }
    }
}

impl SourceInvocationBinding {
    fn for_each_allocation_site_mut(&mut self, visit: &mut impl FnMut(&mut CommandAllocationSite)) {
        if let Some(site) = &mut self.dispatch_site {
            visit(site);
        }
        if let Some(reads) = &mut self.invocation_variable_reads {
            for access in &mut Arc::make_mut(reads).substitutions {
                for_each_owner_site_mut(&mut access.owner, visit);
            }
        }
        for compiled in &mut self.compiled_candidates {
            visit(&mut compiled.compilation_site);
        }
        for named in &mut self.compiled_named_candidates {
            visit(&mut named.compilation_site);
        }
        if let Some(admission) = &mut self.native_compiler_admission {
            match Arc::make_mut(admission) {
                SourceNativeCompilerAdmission::Inline(compiled) => {
                    visit(&mut compiled.compilation_site);
                }
                SourceNativeCompilerAdmission::Named(named) => visit(&mut named.compilation_site),
            }
        }
    }
}

fn for_each_owner_site_mut(
    owner: &mut SourceVariableEvaluationOwner,
    visit: &mut impl FnMut(&mut CommandAllocationSite),
) {
    match owner {
        SourceVariableEvaluationOwner::InvocationArguments { invocation, parent }
        | SourceVariableEvaluationOwner::NativeExpression { invocation, parent }
        | SourceVariableEvaluationOwner::InvocationBody { invocation, parent } => {
            visit(invocation);
            if let Some(parent) = parent {
                for_each_owner_site_mut(Arc::make_mut(parent), visit);
            }
        }
        SourceVariableEvaluationOwner::Alternatives(owners) => {
            for owner in owners {
                for_each_owner_site_mut(owner, visit);
            }
        }
        SourceVariableEvaluationOwner::Unspecified => {}
    }
}

/// Move `script` into `onto`, which holds the bytes of the instance `from` at
/// `delta` bytes further on: every span shifts by `delta`, and every allocation
/// site in `from` (that very instance, not one with equal bytes) becomes the
/// same instruction in `onto`. `onto` must be an instance no analysis issued,
/// so no memo keyed by its origin can answer for a relocated site.
///
/// Returns `false`, with the spans shifted and the sites in `from` moved, when
/// the script retains a site in a third instance, whose instruction `onto`
/// does not hold: a caller that needs every site in `onto` declines then.
pub fn relocate_script(
    script: &mut Script,
    delta: u32,
    from: &Arc<SourceOriginId>,
    onto: &Arc<SourceOriginId>,
) -> bool {
    crate::lattice_rebase::rebase_script(script, i64::from(delta));
    let mut closed = true;
    let mut relocate = |site: &mut CommandAllocationSite| {
        if Arc::ptr_eq(&site.source, from) {
            match site.offset.checked_add(delta) {
                Some(offset) => {
                    debug_assert!(
                        holds_from(from, site.offset, onto, offset),
                        "a relocated allocation site names other bytes"
                    );
                    site.source = Arc::clone(onto);
                    site.offset = offset;
                }
                None => closed = false,
            }
        } else if !Arc::ptr_eq(&site.source, onto) {
            closed = false;
        }
    };
    for_each_script_allocation_site_mut(script, &mut relocate);
    closed
}

/// Whether `onto` holds, from `to`, the bytes `from` holds from `at` onwards.
fn holds_from(from: &SourceOriginId, at: u32, onto: &SourceOriginId, to: u32) -> bool {
    let tail = from.source_image().bytes().get(at as usize..);
    tail.is_some_and(|tail| {
        onto.source_image()
            .bytes()
            .get(to as usize..)
            .is_some_and(|bytes| bytes.starts_with(tail))
    })
}

/// Visit the allocation site of every invocation `script` retains, in its
/// statements and in the binding sites of its typed operations, nested
/// scripts included.
pub fn for_each_script_allocation_site_mut(
    script: &mut Script,
    visit: &mut impl FnMut(&mut CommandAllocationSite),
) {
    crate::ir::for_each_script_mut(script, &mut |script| {
        for statement in &mut script.statements {
            if let Some(tokens) = statement.tokens_mut() {
                tokens.for_each_allocation_site_mut(visit);
            }
        }
        for site in script.command_binding_sites.iter_mut() {
            if let Some(tokens) = &mut site.source_tokens {
                tokens.for_each_allocation_site_mut(visit);
            }
        }
    });
}
