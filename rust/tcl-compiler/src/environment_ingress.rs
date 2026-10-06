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

//! The compiler's dialect-name ingress — a thin delegation to the shared
//! seam in [`tcl_registry::model::ingress`].
//!
//! The implementation lives in the registry model so `tcl-lsp-core`,
//! `tcl-lsp-db`, and `tcl-lsp-server` resolve names through the *same* seam
//! rather than a second copy. The re-exports below keep every compiler call
//! site (and this module's tests) unchanged; see the shared module's docs
//! for the resolution rules and the three accepted micro-unifications.

use std::sync::{Mutex, OnceLock};

pub(crate) use tcl_registry::model::ingress::{
    DocumentEnvironment, context_for_profile, irules_context, resolve_environment,
};

/// The compiler convenience driver's native target contract. An explicitly
/// execution release takes precedence. An exact environment-owned profile can
/// select its separately declared execution default. An unprofiled catalogue
/// selects the compiler's documented Tcl 9.0 target;
/// this choice is made by the driver, never inferred by the source interpreter.
pub(crate) fn authoring_invocation_dialect(
    registry: &tcl_registry::CommandRegistry,
    profile: Option<&tcl_dialect::DialectProfile>,
    config: tcl_lexer::LexerConfig,
) -> tcl_registry::InvocationDialect {
    let mut dialect = profile.or_else(|| registry.profile()).map_or_else(
        || tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0),
        |profile| {
            let selected = tcl_registry::InvocationDialect::of_profile(profile);
            if selected.core_point.is_some() || selected.tcl_version.is_some() {
                return selected;
            }
            tcl_registry::model::ingress::default_execution_point_for_profile(profile)
                .map_or(selected, tcl_registry::InvocationDialect::of_point)
        },
    );
    dialect.lexer_grammar = config.grammar_over(dialect.lexer_grammar);
    dialect.word_values =
        tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    dialect
}

/// A source document is evaluated as a file: native command selection is late.
/// Bytecode and procedure drivers must supply their own explicit compilation entry.
pub(crate) const fn authoring_native_compilation()
-> tcl_registry::native_compilation::NativeCompilationContext {
    use tcl_registry::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    };
    NativeCompilationContext {
        mode: NativeCompilationMode::Direct,
        frame: NativeCompilationFrame::ScriptCode,
        loop_depth: 0,
        catch_depth: Some(0),
    }
}

/// Intern `name` as a `&'static str` — transitional plumbing for the
/// version-gate axis, whose `Package` arm predates the model's
/// `Arc<str>` package names. Bounded by the compiled placement
/// vocabulary (each distinct name leaks once). Compiler-local: it is not a
/// dialect ingress, so it stays out of the shared seam.
#[must_use]
pub(crate) fn interned_package_name(name: &str) -> &'static str {
    static CELL: OnceLock<Mutex<Vec<&'static str>>> = OnceLock::new();
    let interned = CELL.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = interned.lock().expect("package-name intern mutex");
    if let Some(&existing) = guard.iter().find(|&&existing| existing == name) {
        return existing;
    }
    let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
    guard.push(leaked);
    leaked
}

/// Capture the real interpreter's original command and fixed-math entry.
/// This attests registration only: it grants no literal-pool object class,
/// operand effects or successful execution of the fixture being analysed.
#[cfg(test)]
pub(crate) fn captured_native_entry(
    profile: &'static tcl_dialect::DialectProfile,
) -> tcl_runtime_api::NativeCompilationEntry {
    captured_native_entry_with_owner(profile).1
}

/// Keep the original interpreter alive when a test consumes a weak observation.
/// Command snapshots alone deliberately retain no registration-world owner.
#[cfg(test)]
pub(crate) fn captured_native_entry_with_owner(
    profile: &'static tcl_dialect::DialectProfile,
) -> (tcl_vm::Vm, tcl_runtime_api::NativeCompilationEntry) {
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Capture(Rc<RefCell<Option<tcl_runtime_api::NativeCompilationEntry>>>);
    impl tcl_runtime_api::CompileService for Capture {
        type Module = tcl_bytecode::ModuleAsm;
        fn compile_script_bytes_with_entry(
            &self,
            _: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
            _: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            *self.0.borrow_mut() = Some(entry.clone());
            self.compile("")
        }
        fn script_command_plan_bytes_with_entry(
            &self,
            source: &tcl_runtime_api::SourceImage,
            _: &'static tcl_dialect::DialectProfile,
            _: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
            Ok(tcl_runtime_api::ScriptCommandPlan::complete(source.len()))
        }

        fn compile(&self, _: &str) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            Err(tcl_runtime_api::CompileError::Unsupported(
                "entry capture only".into(),
            ))
        }

        fn compile_script_with_entry(
            &self,
            _: tcl_runtime_api::ScriptCompileTarget<'_>,
            _: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            *self.0.borrow_mut() = Some(entry.clone());
            self.compile("")
        }
    }

    let captured = Rc::new(RefCell::new(None));
    let mut vm = tcl_vm::Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("authentic native registration before entry capture");
    vm.set_compiler(Box::new(Capture(Rc::clone(&captured))));
    assert!(vm.try_eval_source("set entry_probe 1").is_err());
    let entry = captured
        .borrow_mut()
        .take()
        .expect("actual interpreter compilation entry");
    (vm, entry)
}

/// Test owner keeps its actual interpreter live through subsequent fact queries.
#[cfg(test)]
pub(crate) struct RetainedNativeUnit {
    unit: crate::compilation_unit::CompilationUnit,
    _owner: tcl_vm::Vm,
}

#[cfg(test)]
impl RetainedNativeUnit {
    pub(crate) fn new(unit: crate::compilation_unit::CompilationUnit, owner: tcl_vm::Vm) -> Self {
        Self {
            unit,
            _owner: owner,
        }
    }
}

#[cfg(test)]
impl std::ops::Deref for RetainedNativeUnit {
    type Target = crate::compilation_unit::CompilationUnit;
    fn deref(&self) -> &Self::Target {
        &self.unit
    }
}

#[cfg(test)]
impl std::ops::DerefMut for RetainedNativeUnit {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.unit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tcl_registry::model::KeyedVersions;

    #[test]
    fn authoring_target_is_explicit_and_lexical_axes_are_independent() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let config = tcl_lexer::LexerConfig::for_dialect("jim");
        let default = authoring_invocation_dialect(&registry, None, config);
        assert_eq!(default.tcl_version, Some(tcl_dialect::TclVersion::V9_0));
        assert_eq!(
            default.lexer_grammar.word_separators,
            config.word_separators
        );
        assert_eq!(default.numbers, tcl_dialect::NumberSyntax::Tcl90);
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        let jim = authoring_invocation_dialect(&registry, Some(&profile), config);
        assert_eq!(jim.tcl_version, None);
        assert_eq!(jim.family(), Some(tcl_dialect::model::Family::Jim));
        assert_eq!(jim.numbers, tcl_dialect::NumberSyntax::Jim080);
    }

    #[test]
    fn canonical_authoring_profile_has_an_independent_native_execution_default() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let profile = resolve_environment("tcl").unit_profile();
        assert!(
            tcl_registry::InvocationDialect::of_profile(profile)
                .tcl_version
                .is_none()
        );
        let config = tcl_lexer::LexerConfig::default();
        let default = authoring_invocation_dialect(&registry, Some(profile), config);
        assert_eq!(default.tcl_version, Some(tcl_dialect::TclVersion::V9_0));

        let mut custom = profile.clone();
        custom.display_name = "Custom unversioned interpreter";
        let unknown = authoring_invocation_dialect(&registry, Some(&custom), config);
        assert_eq!(unknown.tcl_version, None);
        assert_eq!(unknown.core_point, None);

        let explicit = resolve_environment("tcl8.6").unit_profile();
        let pinned = authoring_invocation_dialect(&registry, Some(explicit), config);
        assert_eq!(pinned.tcl_version, Some(tcl_dialect::TclVersion::V8_6));
    }

    #[test]
    fn explicit_source_options_do_not_inherit_the_convenience_target() {
        use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
        let registry = tcl_registry::CommandRegistry::build_default();
        let config = tcl_lexer::LexerConfig::default();
        let source = "set x 1";
        let default = SourceCommandBindings::analyse(source, config, &registry);
        let explicit = SourceCommandBindings::analyse_with_options(
            source,
            config,
            &registry,
            SourceAnalysisOptions::default(),
        );
        assert_eq!(
            default
                .invocation_at_source("set", 0)
                .variable_context
                .invocation_dialect
                .and_then(|d| d.tcl_version),
            Some(tcl_dialect::TclVersion::V9_0),
        );
        assert_eq!(
            explicit
                .invocation_at_source("set", 0)
                .variable_context
                .invocation_dialect,
            None
        );
    }

    #[test]
    fn names_resolve_as_the_old_ingress_did() {
        // Catalogue names and aliases → their same-named profile.
        for (name, profile) in [
            ("tcl8.6", "tcl8.6"),
            ("f5-irules", "f5-irules"),
            ("irules", "f5-irules"),
            ("tcl-irule", "f5-irules"),
            ("f5-iapps", "f5-iapps"),
            ("xilinx-eda-tcl", "xilinx-eda-tcl"),
        ] {
            let environment = resolve_environment(name);
            assert_eq!(environment.definition.id.as_str(), profile, "{name}");
            assert_eq!(environment.analyser_profile().name, profile, "{name}");
            assert_eq!(environment.unit_profile().name, profile, "{name}");
            assert!(
                !environment.document_context().ambient_package("Tk"),
                "{name}"
            );
        }
        // Unknown names and the bare `tcl` land on the lenient
        // environment and the permissive fallback profile.
        for name in ["", "tcl", "no-such-dialect"] {
            let environment = resolve_environment(name);
            assert_eq!(environment.definition.id.as_str(), "tcl", "{name}");
            assert!(environment.analyser_profile().is_fallback(), "{name}");
            assert!(environment.unit_profile().is_fallback(), "{name}");
            assert!(
                !environment.document_context().ambient_package("Tk"),
                "{name}"
            );
        }
        // The `tk` ingress: permissive analyser profile — matching
        // `DialectProfile::by_name`, since `tk` is a package plus an
        // environment and never a catalogue dialect — typed additive unit
        // profile — matching `DialectProfile::resolve_known` — and the Tk
        // fact as a **placement**: the environment ships Tk ambient, which
        // is what a `wish` shell is.
        let tk = resolve_environment("tk");
        assert!(tk.document_context().ambient_package("Tk"));
        assert!(tk.analyser_profile().is_fallback());
        assert_eq!(tk.unit_profile().name, "tk");
    }

    #[test]
    fn context_registries_carry_the_expected_stores() {
        let environment = resolve_environment("tcl8.5");
        let generation = environment.context_registry(&KeyedVersions::default(), 0);
        assert_eq!(
            generation.context().environment.id.as_str(),
            "tcl8.5",
            "the generation answers under the resolved environment"
        );
        // An uninstalled pack overlay falls back to the un-overlaid
        // generation.
        let fallback = environment.context_registry(&KeyedVersions::default(), 0xDEAD);
        assert!(Arc::ptr_eq(generation.commands(), fallback.commands()));
    }

    #[test]
    fn package_names_intern_stably() {
        let first = interned_package_name("f5-irules-cmds");
        let second = interned_package_name("f5-irules-cmds");
        assert!(std::ptr::eq(first, second));
        assert_eq!(first, "f5-irules-cmds");
    }
}
