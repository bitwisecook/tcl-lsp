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

//! Readonly Logical member declarations from the actual completed class walk.

use super::{AnalysisResult, ClassDef, MemberSide, MethodDef};
use crate::analyser::ResolvedAnalysisInput;
use std::sync::Arc;
use tcl_lexer::SourceImage;

/// A completed Logical class declaration, independent of Native name recipes.
/// The snapshot supplies source membership only, never dispatch or editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceClassDeclaration {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    class: Arc<ClassDef>,
}

/// One exact side and declaration in the retained Logical class snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceMemberDeclaration {
    class: LogicalSourceClassDeclaration,
    side: MemberSide,
    method: MethodDef,
}

impl LogicalSourceClassDeclaration {
    pub(crate) fn capture(
        image: SourceImage,
        input: ResolvedAnalysisInput,
        class: &ClassDef,
    ) -> Option<Self> {
        let len = u32::try_from(image.bytes().len()).ok()?;
        let bounded = |span: tcl_lexer::Span| span.start() < span.end() && span.end() <= len;
        (input.has_logical_source_name_context()
            && bounded(class.name_span)
            && bounded(class.body_span))
        .then_some(())?;
        Some(Self {
            image,
            input,
            class: Arc::new(class.clone()),
        })
    }

    fn matches(&self, analysis: &AnalysisResult, class: &ClassDef) -> bool {
        if !analysis.allows_retained_logical_declaration_advice()
            || analysis.resolved_input.as_ref() != Some(&self.input)
            || !analysis.matches_original_source_image(&self.image, self.input.lexer_config())
            || !analysis
                .retained_command_realm()
                .is_some_and(|realm| realm.matches_resolved_analysis_input(&self.input))
        {
            return false;
        }
        let mut current = class.clone();
        current
            .qualified_name
            .clone_from(&self.class.qualified_name);
        current == *self.class
    }

    /// Exact retained member metadata on the selected source side.
    #[must_use]
    pub fn member(
        &self,
        side: MemberSide,
        method: &MethodDef,
    ) -> Option<LogicalSourceMemberDeclaration> {
        let members = match side {
            MemberSide::Instance => &self.class.methods,
            MemberSide::ClassObject => &self.class.class_methods,
        };
        let mut matches = members.values().filter(|candidate| *candidate == method);
        let selected = matches.next()?;
        matches
            .next()
            .is_none()
            .then(|| LogicalSourceMemberDeclaration {
                class: self.clone(),
                side,
                method: selected.clone(),
            })
    }
}

impl AnalysisResult {
    /// A current actual class must still agree with the completed source walk.
    /// Reporting qualified labels are excluded; altered member metadata declines.
    #[must_use]
    pub fn logical_source_class_declaration(
        &self,
        class: &ClassDef,
    ) -> Option<&LogicalSourceClassDeclaration> {
        self.all_classes
            .values()
            .any(|current| std::ptr::eq(current, class))
            .then_some(())?;
        let mut declarations = self
            .logical_class_declarations
            .iter()
            .filter(|declaration| declaration.matches(self, class));
        let selected = declarations.next()?;
        declarations.next().is_none().then_some(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn logical_member_declarations_keep_actual_side_source_and_current_metadata() {
        let source = "oo::class create Base {method m {arg} {}; self method m {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let class = &analysis.all_classes["::Base"];
        let declaration = analysis
            .logical_source_class_declaration(class)
            .expect("actual Logical class walk");
        let instance = declaration
            .member(MemberSide::Instance, &class.methods["m"])
            .unwrap();
        let class_side = declaration
            .member(MemberSide::ClassObject, &class.class_methods["m"])
            .unwrap();
        assert_ne!(instance, class_side);
        assert!(
            declaration
                .member(MemberSide::ClassObject, &class.methods["m"])
                .is_none()
        );
        assert!(
            analysis
                .logical_source_class_declaration(&class.clone())
                .is_none()
        );
        let mut labels = analysis.clone();
        labels.all_classes.get_mut("::Base").unwrap().qualified_name = "display only".into();
        assert_eq!(
            labels.logical_source_class_declaration(&labels.all_classes["::Base"]),
            Some(declaration)
        );
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::Base")
            .unwrap()
            .methods
            .get_mut("m")
            .unwrap()
            .params
            .clear();
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::Base"])
                .is_none()
        );
        let mut changed = analysis.clone();
        changed.all_classes.get_mut("::Base").unwrap().body_span = tcl_lexer::Span::new(0, 1);
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::Base"])
                .is_none()
        );
        let mut changed = analysis.clone();
        changed.resolved_input = None;
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::Base"])
                .is_none()
        );
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            native
                .logical_source_class_declaration(&native.all_classes["::Base"])
                .is_none()
        );
    }
}
