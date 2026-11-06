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

//! Source inventory for Tcllib 2.0 Clay's definition vocabulary.

use super::{
    DefinerFamily, DefinitionBodyGrammar, INSTANCE_METHOD, METHOD_ROLES, MemberEffect, MemberSpec,
    NO_ROLES, TCLOO_MEMBER_BODY_NAMESPACE_PATH, TCLOO_METHOD_VISIBILITY_ARGUMENT,
};

const MEMBERS: &[MemberSpec] = &[
    // DefineThunk passes every original method argument to oo::define.
    MemberSpec::flat("method", METHOD_ROLES, INSTANCE_METHOD)
        .optional_argument(TCLOO_METHOD_VISIBILITY_ARGUMENT),
    // These operations may transform names, rewrite bodies, add methods or
    // change dispatch. They supply no unchanged method-body declaration.
    MemberSpec::flat("constructor", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("destructor", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("superclass", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("mixin", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("variable", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("forward", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("filter", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("export", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("unexport", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("self", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("clay", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("aliases", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("Array", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("Dict", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("Variable", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("Delegate", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("Option", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("Option_Class", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("Method", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("Class_Method", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("class_method", NO_ROLES, MemberEffect::Configuration),
    MemberSpec::flat("Ensemble", NO_ROLES, MemberEffect::Configuration),
];

/// Original class/member syntax only. Package loading, class creation,
/// configuration, generated methods and actual receiver entry remain unknown.
pub const CLAY_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Clay,
    members: MEMBERS,
    implicit_vars: &[],
    implicit_receiver_vars: &[],
    member_body_namespace_path: TCLOO_MEMBER_BODY_NAMESPACE_PATH,
    builtin_type_methods: &[],
    builtin_object_methods: &[],
    builtin_terminating_methods: &[],
    member_body_commands: &[],
    bare_word_construction: false,
    bare_word_construction_hint: None,
    bare_word_construction_source_hint: None,
    dynamic_method_dispatch: true,
    manufacturers: &[],
    unknown_dispatch_method: None,
    property_accessor_methods: &[],
};

#[cfg(test)]
mod tests {
    use super::CLAY_GRAMMAR;
    use crate::definer::{DefinitionMemberNamePurpose, MemberEffect};

    #[test]
    fn source_methods_do_not_supply_native_factory_or_transformed_body_authority() {
        let grammar = &CLAY_GRAMMAR;
        let method = grammar.member("method").unwrap();
        assert_eq!(
            grammar.member_name_purpose(method),
            Some(DefinitionMemberNamePurpose::SourceValue)
        );
        assert!(grammar.command_installation().is_none());
        assert!(grammar.manufacturer("create").is_none());
        assert!(grammar.manufacturer("new").is_none());
        assert!(grammar.default_construction_lookup_dependencies().is_none());
        for name in [
            "constructor",
            "destructor",
            "Method",
            "Class_Method",
            "class_method",
            "Ensemble",
        ] {
            let member = grammar.member(name).unwrap();
            assert_eq!(member.effect, MemberEffect::Configuration);
            assert!(member.arg_roles.is_empty());
            assert!(grammar.source_special_member_kind(member).is_none());
            assert!(grammar.member_name_purpose(member).is_none());
        }
    }
}
