// SPDX-License-Identifier: AGPL-3.0-or-later
//! List and dictionary materialization purposes, independent of source grammar.

use super::{InterpState, NativeBootstrapPurpose};
use tcl_registry::native_string_materialization::{
    LogicalStringProvider, NativeStringMaterialization,
};
use tcl_syntax::native_string::NativeStringProtocol;

/// A selected recipe together with the independent purpose that supplied it.
/// Standalone compatibility and authored simulation establish no native entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObjectMaterialization {
    /// Actual native activation or independently retained engine.
    Native(NativeStringMaterialization),
    /// Explicitly installed F5 object-string simulation.
    AuthoredLogical(NativeStringMaterialization),
    /// Portable Distribution constructor and getter compatibility only.
    StandaloneCompatibility(NativeStringMaterialization),
}

impl ObjectMaterialization {
    const fn recipe(self) -> NativeStringMaterialization {
        match self {
            Self::Native(recipe)
            | Self::AuthoredLogical(recipe)
            | Self::StandaloneCompatibility(recipe) => recipe,
        }
    }

    pub(crate) const fn protocol(self) -> NativeStringProtocol {
        self.recipe().protocol()
    }

    pub(crate) fn byte_array_string_recipe(
        self,
    ) -> Option<tcl_registry::native_string_materialization::ByteArrayStringRecipe> {
        self.recipe().byte_array_string_recipe()
    }

    pub(crate) const fn length_protocol(
        self,
    ) -> tcl_registry::native_stock_list::NativeObjectLengthProtocol {
        tcl_registry::native_stock_list::NativeObjectLengthProtocol::from_materialization(
            self.recipe(),
        )
    }
}

impl InterpState {
    /// Select the actual object carrier or the separately installed logical
    /// materialization provider. An unavailable selected logical purpose is
    /// terminal; the physical engine cannot fill that missing premise.
    ///
    /// Distribution bootstrap keeps its existing standalone object operations
    /// under an explicitly tagged compatibility purpose. Its descriptive source
    /// recipe authenticates neither an actual engine nor a command activation.
    /// The installed F5 object-string simulation, independent of any active
    /// native host activation. Authored logical providers interpret their
    /// operands under this one recipe whichever realm calls them.
    pub(crate) fn authored_logical_object_materialization(&self) -> Option<ObjectMaterialization> {
        let provider = self.logical_providers.names?;
        let dialect = tcl_registry::InvocationDialect::of_profile(self.logical_execution_profile());
        dialect.authored_logical_name_simulation(provider)?;
        let provider = LogicalStringProvider::Tcl84CoreSimulation;
        dialect
            .native_string_materialization(Some(provider))
            .filter(|recipe| recipe.logical_provider() == Some(provider))
            .map(ObjectMaterialization::AuthoredLogical)
    }

    /// The list and dictionary recipe every object producer and consumer
    /// takes: one selector, whichever realm calls it.
    pub(crate) fn object_protocol(&self) -> Option<NativeStringProtocol> {
        self.object_materialization()
            .map(ObjectMaterialization::protocol)
    }

    pub(crate) fn object_materialization(&self) -> Option<ObjectMaterialization> {
        // A reached, independently selected native host activation supplies its
        // own purpose. This is not fallback from a refused logical source call.
        if let Some(profile) = self.active_native_profile {
            return tcl_registry::InvocationDialect::of_profile(profile)
                .native_string_materialization(None)
                .map(ObjectMaterialization::Native);
        }
        if self.logical_providers.names.is_some() {
            return self.authored_logical_object_materialization();
        }
        if let Some(profile) = self.actual_engine_profile {
            return tcl_registry::InvocationDialect::of_profile(profile)
                .native_string_materialization(None)
                .map(ObjectMaterialization::Native);
        }
        if !matches!(self.bootstrap, NativeBootstrapPurpose::Distribution) {
            return None;
        }
        self.native_invocation_dialect()
            .native_string_materialization(None)
            .map(ObjectMaterialization::StandaloneCompatibility)
    }
}

impl crate::Vm {
    /// The recipe that materialises `value` as a string: a list or dictionary
    /// is an object of the selected purpose, anything else the physical
    /// engine's.
    pub(crate) fn materialization_protocol(
        &self,
        value: &crate::Value,
    ) -> Option<NativeStringProtocol> {
        if value.is_native_compound() {
            self.object_protocol()
        } else {
            self.actual_native_invocation_dialect()
                .native_string_protocol()
        }
    }

    /// `value` as a list under the selected object recipe.
    pub(crate) fn object_list(
        &self,
        value: &crate::Value,
    ) -> Result<crate::NativeListItems, crate::TclError> {
        let protocol = self.object_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("selected List recipe"),
        )?;
        self.native_object_list_elements_in(value, protocol)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Value, Vm};
    use tcl_dialect::TclVersion;
    use tcl_syntax::value::ValueOps;

    fn native_vm(environment: &str) -> Vm {
        crate::native_fixture::interpreter(
            tcl_registry::model::ingress::resolve_environment(environment).unit_profile(),
        )
    }

    fn unknown_jim() -> &'static tcl_dialect::DialectProfile {
        Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )))
    }

    #[test]
    fn actual_object_materialization_survives_unknown_source_without_losing_original_members() {
        // naming.invocation.original-object-materialization-purpose
        // docs/design/analysis/name-resolution-proofs/invocation-original-object-materialization-purpose.md
        let mut vm = native_vm("tcl8.6");
        vm.set_dialect_profile(unknown_jim());
        let selected = vm.object_materialization().unwrap();
        assert!(matches!(selected, ObjectMaterialization::Native(_)));
        assert_eq!(
            selected.protocol(),
            NativeStringProtocol::C(TclVersion::V8_6)
        );
        let key = Value::int(17);
        let member = Value::int(23);
        let list = vm.new_list(vec![member.clone()]);
        assert_eq!(vm.list_len(&list), Ok(1));
        let elements = vm.list_elements(&list).unwrap();
        assert!(elements[0].is_same_object(&member));
        for dictionary in [
            vm.new_dict(vec![(key.clone(), member.clone())]),
            vm.new_dict_with_hash_bucket_count(vec![(key.clone(), member.clone())], 8),
            vm.new_dict_checked(vec![(key.clone(), member.clone())])
                .unwrap(),
            vm.new_dict_with_hash_bucket_count_checked(vec![(key.clone(), member.clone())], 8)
                .unwrap(),
        ] {
            let original_pairs = <Vm as ValueOps>::dict_pairs(&mut vm, &dictionary).unwrap();
            assert!(original_pairs[0].0.is_same_object(&key));
            assert!(original_pairs[0].1.is_same_object(&member));
            // The test convenience adapter projects key text independently of
            // the original Dictionary key header retained by the native getter.
            let pairs = vm.dict_pairs(&dictionary).unwrap();
            assert_eq!(pairs[0].0, "17");
            assert!(pairs[0].1.is_same_object(&member));
            assert!(
                dictionary
                    .with_native_compound_string_protocol(NativeStringProtocol::C(TclVersion::V9_0))
                    .is_err()
            );
        }
        assert!(key.resident_string_bytes().is_some());
        assert!(member.resident_string_bytes().is_none());
    }

    #[test]
    fn selected_authored_materialization_refuses_without_physical_fallback() {
        // naming.invocation.original-object-materialization-purpose
        // docs/design/analysis/name-resolution-proofs/invocation-original-object-materialization-purpose.md
        let mut vm = native_vm("tcl9.0");
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(TclVersion::V8_4,)
        ));
        let selected = vm.object_materialization().unwrap();
        assert!(matches!(
            selected,
            ObjectMaterialization::AuthoredLogical(_)
        ));
        assert_eq!(
            selected.protocol(),
            NativeStringProtocol::C(TclVersion::V8_4)
        );
        assert_eq!(
            selected.length_protocol().logical_provider(),
            Some(tcl_registry::native_stock_list::LogicalListLengthProvider::Tcl84CoreSimulation),
        );
        let member = Value::int(17);
        let list = vm.new_list(vec![member.clone()]);
        assert_eq!(vm.list_len(&list), Ok(1));
        assert!(vm.list_elements(&list).unwrap()[0].is_same_object(&member));
        vm.set_dialect_profile(unknown_jim());
        assert!(
            vm.actual_native_invocation_dialect()
                .native_string_protocol()
                .is_some()
        );
        assert!(vm.object_materialization().is_none());
        assert!(vm.list_len(&member).is_err());
        assert!(vm.list_elements(&member).is_err());
        assert!(vm.dict_pairs(&member).is_err());
        assert!(vm.new_dict_checked(Vec::new()).is_err());
        assert!(member.resident_string_bytes().is_none());
        // The actual host activation is an independent, positively selected
        // purpose, and leaving it restores the unavailable logical purpose.
        vm.active_native_profile = vm.actual_engine_profile;
        assert!(matches!(
            vm.object_materialization(),
            Some(ObjectMaterialization::Native(_))
        ));
        assert_eq!(
            vm.object_materialization().unwrap().protocol(),
            NativeStringProtocol::C(TclVersion::V9_0)
        );
        vm.active_native_profile = None;
        assert!(vm.object_materialization().is_none());
    }

    #[test]
    fn standalone_compatibility_is_separate_and_unknown_source_cannot_select_a_recipe() {
        // naming.invocation.original-object-materialization-purpose
        // docs/design/analysis/name-resolution-proofs/invocation-original-object-materialization-purpose.md
        let mut vm = Vm::new();
        assert!(vm.actual_engine_profile.is_none());
        assert!(matches!(
            vm.object_materialization(),
            Some(ObjectMaterialization::StandaloneCompatibility(_))
        ));
        let member = Value::int(17);
        let list = vm.new_list(vec![member.clone()]);
        assert_eq!(vm.list_len(&list), Ok(1));
        assert!(vm.list_elements(&list).unwrap()[0].is_same_object(&member));
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        assert!(matches!(
            vm.object_materialization(),
            Some(ObjectMaterialization::StandaloneCompatibility(_))
        ));
        assert_eq!(
            vm.object_materialization().unwrap().protocol(),
            NativeStringProtocol::Jim084
        );
        let source = Value::from_string_bytes(vec![b'{', 0xff]);
        assert_eq!(
            vm.list_elements(&source).unwrap()[0]
                .string_bytes()
                .as_ref(),
            &[0xff]
        );
        vm.set_dialect_profile(unknown_jim());
        assert!(vm.object_materialization().is_none());
        let untouched = Value::int(23);
        assert!(vm.list_len(&untouched).is_err());
        assert!(vm.list_elements(&untouched).is_err());
        assert!(vm.dict_pairs(&untouched).is_err());
        assert!(untouched.resident_string_bytes().is_none());
        assert!(vm.new_dict_checked(Vec::new()).is_err());
        let portable = vm.new_list(vec![untouched]);
        assert!(portable.resident_string_bytes().is_none());
        assert_eq!(portable.native_object_type_name(), "list");
    }

    #[test]
    fn actual_object_materialization_preserves_foreign_compound_refusal() {
        // naming.invocation.original-object-materialization-purpose
        // docs/design/analysis/name-resolution-proofs/invocation-original-object-materialization-purpose.md
        let mut vm = native_vm("tcl8.6");
        vm.set_dialect_profile(unknown_jim());
        let foreign = NativeStringProtocol::C(TclVersion::V9_0);
        let member = Value::int(17);
        let list = Value::native_list_constructor(vec![member.clone()], foreign);
        let dictionary = Value::native_dictionary_constructor(
            vec![(Value::string("k"), member.clone())],
            None,
            foreign,
        )
        .unwrap();
        assert!(vm.list_len(&list).is_err());
        assert!(vm.list_elements(&list).is_err());
        assert!(vm.dict_pairs(&dictionary).is_err());
        assert!(list.resident_string_bytes().is_none());
        assert!(dictionary.resident_string_bytes().is_none());
        assert!(member.resident_string_bytes().is_none());
        assert!(list.with_native_compound_string_protocol(foreign).is_ok());
        assert!(
            dictionary
                .with_native_compound_string_protocol(foreign)
                .is_ok()
        );
    }

    /// The iRules simulation over a Tcl 9 engine, every authored provider set.
    fn simulation() -> Vm {
        use tcl_registry::invocation_words::{
            LogicalExpressionParseProvider, LogicalExpressionQuoteProvider,
            LogicalSourceWordProvider,
        };
        let host = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            std::rc::Rc::new(crate::host_native::NativeHost::new()),
            host,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let profile = tcl_dialect::DialectProfile::irules();
        vm.set_dialect_profile(profile);
        assert!(vm.set_command_surface_profile(host));
        assert!(vm.set_native_engine_profile(host));
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        assert!(vm.set_logical_eval_object_provider(
            tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation
        ));
        assert!(
            vm.set_logical_source_word_provider(LogicalSourceWordProvider::Tcl84CoreSimulation)
        );
        assert!(vm.set_logical_expression_parse_provider(
            LogicalExpressionParseProvider::Tcl84CoreSimulation
        ));
        assert!(vm.set_logical_quote_provider(LogicalExpressionQuoteProvider::Tcl84CoreSimulation));
        assert!(vm.set_logical_numeric_provider(
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
        ));
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(TclVersion::V8_4)
        ));
        assert!(vm.set_logical_compiled_variable_provider(
            tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation
        ));
        vm
    }

    fn answer(completion: &tcl_runtime_api::Completion<Value>, script: &str) -> String {
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok, "{script}");
        completion.result.try_to_str().unwrap().to_string()
    }

    #[test]
    fn one_proc_reads_and_writes_its_lists_under_one_recipe_in_each_realm() {
        // Expected forms are tclsh 8.4.20 and 9.0.4 output for the same script.
        const DEFINE: &str = "proc p {args} {lindex $args 0; linsert $args 0 x}
            proc q {args} {list [lindex $args 0] [linsert $args 0 x]}";
        const CASES: &[(&str, &str, &str)] = &[
            ("p #c d", "x #c d", "x #c d"),
            ("q #c d", "#c {x #c d}", "{#c} {x #c d}"),
            ("list #a b", "#a b", "{#a} b"),
            ("lindex {a\\x4142 b} 0", "aB", "aA42"),
            ("linsert {a\\x4142 b} 0 x", "x aB b", "x aA42 b"),
        ];
        let mut vm = simulation();
        assert_eq!(
            vm.authored_logical_object_materialization()
                .map(ObjectMaterialization::protocol),
            Some(NativeStringProtocol::C(TclVersion::V8_4))
        );
        answer(&vm.try_eval_source(DEFINE).unwrap(), DEFINE);
        let mut framework = simulation();
        answer(
            &framework.try_eval_native_host_source(DEFINE).unwrap(),
            DEFINE,
        );
        let mut differences = Vec::new();
        for (script, tcl84, tcl90) in CASES {
            let authored = answer(&vm.try_eval_source(script).unwrap(), script);
            let host = answer(
                &framework.try_eval_native_host_source(script).unwrap(),
                script,
            );
            if (authored.as_str(), host.as_str()) != (*tcl84, *tcl90) {
                differences.push(format!("{script}: {authored:?} / {host:?}"));
            }
        }
        assert_eq!(differences, Vec::<String>::new());
    }

    #[test]
    fn a_list_sealed_by_one_realm_is_refused_by_the_other() {
        let mut vm = simulation();
        answer(
            &vm.try_eval_native_host_source("set ::crossing [list a b]; llength $::crossing")
                .unwrap(),
            "host list",
        );
        let crossed = vm.try_eval_source("lindex $::crossing 0");
        assert!(
            !crossed
                .as_ref()
                .is_ok_and(|completion| completion.code == tcl_runtime_api::Code::Ok),
            "an authored reader cannot reparse a host-sealed list"
        );
    }
}
