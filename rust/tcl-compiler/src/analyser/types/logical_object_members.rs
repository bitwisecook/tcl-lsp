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

//! Selected Logical per-object member source, independent of Native dispatch.

use super::{AnalysisResult, MemberSide, MethodDef, ObjectMemberState, OriginalSourceMemberLedger};
use crate::analyser::ResolvedAnalysisInput;
use std::collections::{HashMap, HashSet};
use tcl_lexer::{NativeWord, SourceImage};
use tcl_registry::definer::MemberVisibility;

/// Whole current source metadata with independently selected name and mask operands.
/// This records source suggestions, not an entered or live object method table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceObjectMemberState {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    methods: HashMap<String, MethodDef>,
    exports: HashSet<String>,
    unexports: HashSet<String>,
    anchor: u32,
    conditional: bool,
    originals: OriginalSourceMemberLedger,
}

impl LogicalSourceObjectMemberState {
    pub(crate) fn capture(
        image: SourceImage,
        input: ResolvedAnalysisInput,
        state: &ObjectMemberState,
        originals: &OriginalSourceMemberLedger,
    ) -> Option<Self> {
        if !input.has_logical_source_name_context()
            || state.conditional
            || usize::try_from(state.anchor_offset).ok()? >= image.bytes().len()
            || state.methods.values().any(|method| {
                originals
                    .logical_name_operand(MemberSide::Instance, method, &input, &image)
                    .is_none()
            })
            || state.exports.iter().any(|name| {
                name.as_bytes().contains(&0)
                    || originals.logical_visibility_name(name, &input, &image)
                        != Some(MemberVisibility::Exported)
            })
            || state.unexports.iter().any(|name| {
                name.as_bytes().contains(&0)
                    || originals.logical_visibility_name(name, &input, &image)
                        != Some(MemberVisibility::Unexported)
            })
        {
            return None;
        }
        Some(Self {
            image,
            input,
            methods: state.methods.clone(),
            exports: state.exports.clone(),
            unexports: state.unexports.clone(),
            anchor: state.anchor_offset,
            conditional: state.conditional,
            originals: originals.clone(),
        })
    }

    pub(crate) fn matches_state(
        &self,
        state: &ObjectMemberState,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        self.image == *image && self.input == *input && self.matches_metadata(state)
    }

    fn matches_metadata(&self, state: &ObjectMemberState) -> bool {
        self.methods == state.methods
            && self.exports == state.exports
            && self.unexports == state.unexports
            && self.anchor == state.anchor_offset
            && self.conditional == state.conditional
    }

    pub(crate) fn original_members(&self) -> &OriginalSourceMemberLedger {
        &self.originals
    }
}

/// A current source state reborrowed through its actual analysis membership.
pub struct LogicalSourceObjectMembers<'a> {
    owner: &'a LogicalSourceObjectMemberState,
    state: &'a ObjectMemberState,
}

impl LogicalSourceObjectMembers<'_> {
    /// Original name word of an actual member in the selected source state.
    #[must_use]
    pub fn name_operand(&self, method: &MethodDef) -> Option<&NativeWord> {
        self.state
            .methods
            .values()
            .any(|current| std::ptr::eq(current, method))
            .then_some(())?;
        self.owner.originals.logical_name_operand(
            MemberSide::Instance,
            method,
            &self.owner.input,
            &self.owner.image,
        )
    }

    /// Literal mask names selected by their own original visibility statements.
    pub fn export_names(&self) -> impl Iterator<Item = &str> {
        self.owner.exports.iter().map(String::as_str)
    }

    /// Literal mask names selected by their own original visibility statements.
    pub fn unexport_names(&self) -> impl Iterator<Item = &str> {
        self.owner.unexports.iter().map(String::as_str)
    }

    /// Preserve the original source grammar when suggesting a literal name.
    #[must_use]
    pub fn source_word(&self, method: &MethodDef) -> Option<String> {
        let value = tcl_syntax::word_rules::original_static_word_ascii_presentation(
            self.name_operand(method)?,
        )?;
        if value.contains(&0) {
            return None;
        }
        let name = std::str::from_utf8(&value).ok()?;
        tcl_syntax::word_rules::lexical_ascii_source_word(name, self.owner.input.lexer_config())
    }
}

impl AnalysisResult {
    /// Original source and actual state membership remain separate from receiver dispatch.
    #[must_use]
    pub fn logical_source_object_member_state<'a>(
        &'a self,
        state: &'a ObjectMemberState,
    ) -> Option<LogicalSourceObjectMembers<'a>> {
        self.object_member_state
            .values()
            .flatten()
            .any(|current| std::ptr::eq(current, state))
            .then_some(())?;
        if !self.allows_retained_logical_declaration_advice() {
            return None;
        }
        let input = self.resolved_input.as_ref()?;
        let realm = self.retained_command_realm()?;
        realm.matches_resolved_analysis_input(input).then_some(())?;
        let image = realm.original_source_image()?;
        self.matches_original_source_image(image, input.lexer_config())
            .then_some(())?;
        let owner = state.logical_source_members.as_ref()?;
        owner
            .matches_state(state, image, input)
            .then_some(LogicalSourceObjectMembers { owner, state })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn logical_object_names_and_masks_require_the_current_selected_state() {
        let source = "oo::class create C {method masked {} {}}; set o [C new]; oo::objdefine $o {method {$own} {} {}; export {$own}; unexport masked}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let state = analysis.object_member_state["o"].last().unwrap();
        let view = analysis
            .logical_source_object_member_state(state)
            .expect("current original per-object source");
        let method = &state.methods["$own"];
        assert_eq!(view.source_word(method).as_deref(), Some("{$own}"));
        assert_eq!(view.export_names().collect::<Vec<_>>(), vec!["$own"]);
        assert_eq!(view.unexport_names().collect::<Vec<_>>(), vec!["masked"]);
        assert!(view.name_operand(&method.clone()).is_none());
        assert!(
            analysis
                .logical_source_object_member_state(&state.clone())
                .is_none()
        );
        let mut changed = analysis.clone();
        changed
            .object_member_state
            .get_mut("o")
            .unwrap()
            .last_mut()
            .unwrap()
            .unexports
            .clear();
        let state = changed.object_member_state["o"].last().unwrap();
        assert!(changed.logical_source_object_member_state(state).is_none());
        let mut changed = analysis.clone();
        changed.resolved_input = None;
        let state = changed.object_member_state["o"].last().unwrap();
        assert!(changed.logical_source_object_member_state(state).is_none());
    }

    #[test]
    fn logical_object_source_preserves_prior_operands_and_refuses_open_mutations() {
        let prefix = "oo::class create C {}; set o [C new]; oo::objdefine $o {method first {} {}};";
        let source = format!("{prefix} oo::objdefine $o {{method second {{}} {{}}}}");
        let analysis = Analyser::new().analyse(&source, "tcl");
        let state = analysis.object_member_state["o"].last().unwrap();
        let view = analysis.logical_source_object_member_state(state).unwrap();
        for name in ["first", "second"] {
            assert_eq!(
                view.source_word(&state.methods[name]).as_deref(),
                Some(name)
            );
        }
        for suffix in [
            "oo::objdefine $o {renamemethod first moved}",
            "oo::objdefine $o {unexport $computed}",
            "oo::objdefine [pick] {unexport first}",
        ] {
            let source = format!("{prefix} {suffix}");
            let analysis = Analyser::new().analyse(&source, "tcl");
            let state = analysis.object_member_state["o"].last().unwrap();
            assert!(analysis.logical_source_object_member_state(state).is_none());
        }
    }

    #[test]
    fn logical_object_source_refuses_nul_masks_and_receiver_names() {
        for command in ["export", "unexport"] {
            let source = format!(
                "oo::class create C {{}}; set o [C new]; oo::objdefine $o {{{command} {{nul\0mask}}}}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl");
            let state = analysis.object_member_state["o"].last().unwrap();
            let masks = if command == "export" {
                &state.exports
            } else {
                &state.unexports
            };
            assert!(masks.contains("nul\0mask"));
            assert!(state.logical_source_members.is_none());
            assert!(analysis.logical_source_object_member_state(state).is_none());
        }
        for receiver in ["{nul\0object}", "${nul\0variable}"] {
            let source = format!(
                "oo::class create C {{}}; set o [C new]; oo::objdefine $o {{method prior {{}} {{}}}}; oo::objdefine {receiver} {{method unsupported {{}} {{}}}}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl");
            let prior = analysis.object_member_state["o"].last().unwrap();
            assert!(analysis.logical_source_object_member_state(prior).is_none());
            assert!(
                analysis
                    .object_member_state
                    .keys()
                    .all(|name| !name.as_bytes().contains(&0))
            );
        }
    }

    #[test]
    fn objdefine_receiver_keeps_literal_sigils_separate_from_scalar_references() {
        let source = "oo::class create C {}; C create {$literal}; C create literal; oo::objdefine {$literal} {method own {} {}}; oo::objdefine literal {method other {} {}}; set o [C new]; oo::objdefine \"$o\" {method quoted {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let literal = analysis.object_member_state["$literal"].last().unwrap();
        assert!(literal.methods.contains_key("own"));
        assert!(!literal.methods.contains_key("other"));
        let bare = analysis.object_member_state["literal"].last().unwrap();
        assert!(bare.methods.contains_key("other"));
        assert!(!bare.methods.contains_key("own"));
        let variable = analysis.object_member_state["o"].last().unwrap();
        assert!(variable.methods.contains_key("quoted"));
    }
}
