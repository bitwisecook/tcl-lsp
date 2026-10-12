// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual interpreter execution constants, independent of artifacts and literal arrays.

use super::Vm;
use crate::Value;
use tcl_registry::native_tcloo_compilation::NativeTclOoObjectInfo;
use tcl_syntax::value::ValueError;

impl Vm {
    pub(crate) fn native_oo_predicate_result(
        &mut self,
        present: bool,
    ) -> Result<Value, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native object predicate execution constants",
            ));
        }
        let version = dialect
            .tcl_version
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native object predicate execution constants",
            ))?;
        if !NativeTclOoObjectInfo::IsObject.uses_execution_constant(version) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native object predicate execution constants",
            ));
        }
        self.native_c_execution_boolean(present, version)
    }

    pub(crate) fn native_c_execution_boolean(
        &mut self,
        present: bool,
        version: tcl_dialect::TclVersion,
    ) -> Result<Value, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version != Some(version)
            || !tcl_registry::native_string_compilation::uses_execution_constant(version)
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "native C execution constants",
            ));
        }
        let constants = self
            .native_execution_booleans
            .get_or_insert_with(|| [Value::int(0), Value::int(1)]);
        Ok(constants[usize::from(present)].clone())
    }
    pub(crate) fn native_compiled_match_result(
        &mut self,
        pattern: Value,
        matched: bool,
        version: tcl_dialect::TclVersion,
        operation: tcl_registry::native_string_compilation::NativeStringMatchOperation,
    ) -> Result<Value, ValueError> {
        if tcl_registry::native_string_compilation::uses_execution_constant(version) {
            return self.native_c_execution_boolean(matched, version);
        }
        let dialect = self.actual_native_invocation_dialect();
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version != Some(version)
            || version != tcl_dialect::TclVersion::V8_4
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "native C84 match result",
            ));
        }
        if !operation.reuses_unshared_pattern(version) || pattern.native_object_is_shared() {
            let result = Value::int(i64::from(matched));
            result.set_native_unshared_integer(i64::from(matched), version)?;
            Ok(result)
        } else {
            pattern.set_native_unshared_integer(i64::from(matched), version)?;
            Ok(pattern)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;
    use tcl_registry::native_string_compilation::NativeStringMatchOperation as Match;
    use tcl_syntax::scalar_getter::NativeScalarCache;

    #[test]
    fn array_existence_results_share_only_the_selected_native_environment_constant() {
        // naming.compiler.introspection-source-and-effect-frontiers
        // docs/design/analysis/name-resolution-proofs/compiler-introspection-source-and-effect-frontiers.md
        // Original C case15/16/17 header windows bind the producer selection.
        // This VM control separately checks owned model identity, not a C pointer.
        use tcl_syntax::value::ValueOps;
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::core(profile);
            for present in [false, true] {
                let first = vm.array_existence_result(present).unwrap();
                let second = vm.array_existence_result(present).unwrap();
                let constant = matches!(
                    vm.actual_native_invocation_dialect().tcl_version,
                    Some(TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
                );
                assert_eq!(
                    first.native_object_identity() == second.native_object_identity(),
                    constant,
                    "{engine} {present}"
                );
                assert_eq!(
                    first.string_bytes().as_ref(),
                    if present { b"1" } else { b"0" }
                );
                if constant {
                    let version = vm.actual_native_invocation_dialect().tcl_version.unwrap();
                    let instruction = vm.native_c_execution_boolean(present, version).unwrap();
                    assert_eq!(
                        first.native_object_identity(),
                        instruction.native_object_identity()
                    );
                }
            }
        }
    }

    #[test]
    fn compiled_match_results_keep_native_constant_and_c84_pattern_owners() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let mut vm = crate::native_fixture::core(profile);
            let version = vm.actual_native_invocation_dialect().tcl_version.unwrap();
            let pattern = Value::string("*");
            let identity = pattern.native_object_identity();
            let result = vm
                .native_compiled_match_result(pattern, true, version, Match::Glob { nocase: false })
                .unwrap();
            assert_eq!(
                result.native_object_identity() == identity,
                version == TclVersion::V8_4,
                "{engine}"
            );
            assert!(result.resident_string_bytes().is_none());
            if version == TclVersion::V8_4 {
                assert_eq!(
                    result.native_scalar_cache(),
                    Some(NativeScalarCache::Tcl84Long(1))
                );
            } else {
                let again = vm
                    .native_compiled_match_result(Value::string("A"), true, version, Match::Equal)
                    .unwrap();
                assert_eq!(
                    result.native_object_identity(),
                    again.native_object_identity(),
                    "{engine}"
                );
            }
            let pattern = Value::string("A");
            let identity = pattern.native_object_identity();
            let equal = vm
                .native_compiled_match_result(pattern, true, version, Match::Equal)
                .unwrap();
            assert_ne!(equal.native_object_identity(), identity, "{engine}");
            if version == TclVersion::V8_4 {
                assert_eq!(
                    equal.native_scalar_cache(),
                    Some(NativeScalarCache::Tcl84Long(1))
                );
            }
        }
    }

    #[test]
    fn original_match_source_visits_pattern_then_subject_under_authentic_compiler() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let mut vm = crate::native_fixture::interpreter(profile);
            let completion = vm.eval_source("set order {}; proc pattern {} {global order; set order pattern; return *}; proc subject {} {global order; set order [list $order subject]; return A}; proc match {} {string match [pattern] [subject]}; list [match] $order").unwrap();
            assert_eq!(
                completion.code,
                crate::Code::Ok,
                "{engine}: {}",
                completion.result.to_str()
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                b"1 {pattern subject}",
                "{engine}"
            );
        }
    }
}
