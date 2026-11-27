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

//! Source-advice custody from a completed modeled stock class publication.

use super::{Arc, OriginalJimStockClass, SourceCommandBindings, SourceJimClassHelper};
use crate::command_binding::source_command_world::OriginalCommandPublication;
use crate::command_binding::{BindingBaseline, OriginalSourceAnalysisOwner};

#[derive(Clone)]
pub(in crate::command_binding) struct OriginalJimStockSourceEffects {
    class: Arc<OriginalJimStockClass>,
    owner: Arc<OriginalSourceAnalysisOwner>,
    baseline: Arc<BindingBaseline>,
    publications: Vec<Arc<OriginalCommandPublication>>,
}
impl std::fmt::Debug for OriginalJimStockSourceEffects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalJimStockSourceEffects")
            .field("publications", &self.publications.len())
            .finish_non_exhaustive()
    }
}
impl PartialEq for OriginalJimStockSourceEffects {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.class, &other.class)
            && Arc::ptr_eq(&self.owner, &other.owner)
            && Arc::ptr_eq(&self.baseline, &other.baseline)
            && self.publications == other.publications
    }
}
impl Eq for OriginalJimStockSourceEffects {}

impl super::OriginalJimStockName {
    pub(in crate::command_binding) fn is_source_class_dispatcher(&self) -> bool {
        self.helper == SourceJimClassHelper::Dispatcher
            && self.method.is_none()
            && self.owner.base.is_none()
    }
    pub(in crate::command_binding) fn same_source_class_helper(
        &self,
        other: &Self,
        helper: SourceJimClassHelper,
    ) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && other.method.is_none() && other.helper == helper
    }
    pub(in crate::command_binding) fn matches_class_continuation(
        &self,
        point: &crate::command_binding::SourceBindingPoint,
    ) -> bool {
        self.is_source_class_dispatcher()
            && !point.dispatch
            && !point.declaration_preview
            && point.realm == tcl_dialect::model::InvocationRealm::RuleLoader
            && point.continuation_of.as_ref().is_some_and(|site| {
                site.offset == self.owner.site.offset
                    && Arc::ptr_eq(&site.source, &self.owner.site.source)
            })
    }
    pub(in crate::command_binding) fn source_class_effects(
        &self,
        owner: &Arc<OriginalSourceAnalysisOwner>,
        bindings: &SourceCommandBindings,
        point_index: usize,
        publications: Vec<Arc<OriginalCommandPublication>>,
    ) -> Option<OriginalJimStockSourceEffects> {
        let point = bindings.points.get(point_index)?;
        let state = &point.state;
        let input = state.baseline.metadata_context.source_analysis_input()?;
        (Arc::ptr_eq(owner, bindings.original_analysis_owner()?) && owner.matches(bindings, input))
            .then_some(())?;
        self.matches_class_continuation(point).then_some(())?;
        self.matches_publication_state(state).then_some(())?;
        for (helper, publication) in self.owner.recipe.publications().iter().zip(&publications) {
            let crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(value) =
                publication.name_input()
            else {
                return None;
            };
            self.same_source_class_helper(value.jim_stock_name()?, *helper)
                .then_some(())?;
            let key =
                state.original_command_key_for_slot(publication.slot(), publication.policy())?;
            let bindings = state.original_bindings_for_key(&key)?;
            let super::MayBinding::Target(target) = bindings.first()? else {
                return None;
            };
            (bindings.len() == 1
                && publication
                    .has_implementation_allocation(target.implementation_allocation.as_ref()?))
            .then_some(())?;
        }
        (self.owner.recipe.publications().len() == publications.len()).then_some(())?;
        Some(OriginalJimStockSourceEffects {
            class: Arc::clone(&self.owner),
            owner: Arc::clone(owner),
            baseline: Arc::clone(&state.baseline),
            publications,
        })
    }
}
impl OriginalJimStockSourceEffects {
    pub(in crate::command_binding) fn matches_seed(
        &self,
        owner: Option<&Arc<OriginalSourceAnalysisOwner>>,
        origin: &Arc<crate::command_binding::SourceOriginId>,
        baseline: &Arc<BindingBaseline>,
    ) -> bool {
        owner.is_some_and(|owner| Arc::ptr_eq(owner, &self.owner))
            && Arc::ptr_eq(baseline, &self.baseline)
            && Arc::ptr_eq(origin, &self.class.site.source)
    }
    pub(in crate::command_binding) fn original(&self) -> &[tcl_lexer::NativeWord] {
        &self.class.original
    }
    pub(in crate::command_binding) fn publications(
        &self,
        original: &[tcl_lexer::NativeWord],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<&[Arc<OriginalCommandPublication>]> {
        (self.class.original == original
            && self.class.policy == policy
            && self.class.base.is_none())
        .then_some(self.publications.as_slice())
    }
}
/// Whether any point of one undetached point inventory continues a Jim stock
/// class under one origin and baseline. The answer reads the points and the
/// two identities only. A weak handle reserves each allocation's address, and
/// a point inventory with a weak handle is never written in place, so equal
/// addresses are the same inputs. A clone starts empty.
#[derive(Default)]
pub(in crate::command_binding) struct JimStockContinuationMemo(
    std::sync::Mutex<Option<JimStockContinuationSlot>>,
);

struct JimStockContinuationSlot {
    points: std::sync::Weak<Vec<super::super::SourceBindingPoint>>,
    origin: std::sync::Weak<super::super::SourceOriginId>,
    baseline: std::sync::Weak<super::super::BindingBaseline>,
    any: bool,
}

impl Clone for JimStockContinuationMemo {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for JimStockContinuationMemo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JimStockContinuationMemo")
            .finish_non_exhaustive()
    }
}

impl JimStockContinuationMemo {
    fn any(
        &self,
        points: &super::super::SharedSourceInventory<Vec<super::super::SourceBindingPoint>>,
        origin: &Arc<super::super::SourceOriginId>,
        baseline: &Arc<super::super::BindingBaseline>,
        scan: impl FnOnce(&[super::super::SourceBindingPoint]) -> bool,
    ) -> bool {
        let points = points.shared();
        let mut slot = self.0.lock().expect("jim stock continuation memo poisoned");
        if let Some(current) = slot.as_ref().filter(|current| {
            std::ptr::eq(current.points.as_ptr(), Arc::as_ptr(&points))
                && std::ptr::eq(current.origin.as_ptr(), Arc::as_ptr(origin))
                && std::ptr::eq(current.baseline.as_ptr(), Arc::as_ptr(baseline))
        }) {
            return current.any;
        }
        let any = scan(&points);
        *slot = Some(JimStockContinuationSlot {
            points: Arc::downgrade(&points),
            origin: Arc::downgrade(origin),
            baseline: Arc::downgrade(baseline),
            any,
        });
        any
    }
}

impl SourceCommandBindings {
    pub(in crate::command_binding) fn original_jim_stock_source_effects(
        &self,
    ) -> Vec<OriginalJimStockSourceEffects> {
        let Some(owner) = self.original_analysis_owner() else {
            return Vec::new();
        };
        let Some(origin) = self.source_origin() else {
            return Vec::new();
        };
        // A class continuation needs a rule-loader point whose world holds
        // a Jim stock publication; both are checked before any scan.
        let continues = |point: &super::super::SourceBindingPoint| {
            point
                .state
                .original_command_world
                .may_hold_jim_stock_publication()
                && !point.declaration_preview
                && !point.dispatch
                && point.realm == tcl_dialect::model::InvocationRealm::RuleLoader
                && point
                    .continuation_of
                    .as_ref()
                    .is_some_and(|site| Arc::ptr_eq(&site.source, origin))
                && Arc::ptr_eq(&point.state.baseline, &self.final_state.baseline)
        };
        if !self.derived_indexes.get().jim_stock_continuations.any(
            &self.points,
            origin,
            &self.final_state.baseline,
            |points| points.iter().any(&continues),
        ) {
            return Vec::new();
        }
        let mut effects = Vec::new();
        for (point_index, point) in self.points.iter().enumerate() {
            if !continues(point) {
                continue;
            }
            for effect in point.state.original_command_world.jim_stock_source_effects(
                self,
                point_index,
                owner,
            ) {
                if !effects.contains(&effect) {
                    effects.push(effect);
                }
            }
        }
        effects
    }
}
