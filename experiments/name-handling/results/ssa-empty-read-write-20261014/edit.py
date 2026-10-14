from pathlib import Path
p=Path('/tmp/name-ssa-empty-read-write-20261014/after/rust/tcl-compiler/src/ssa.rs');s=p.read_text()
def replace(old,new,count=1):
 global s
 assert s.count(old)==count,(old,s.count(old),count)
 s=s.replace(old,new)
replace('use crate::naming::normalise_var_name;\n','')
replace('''            .map(std::borrow::Cow::into_owned)
            .filter(|name| !name.is_empty())
            .collect()''','''            .map(std::borrow::Cow::into_owned)
            .collect()''')
replace('''            })
            .filter(|n| !n.is_empty()),''','''            }),''')
replace('''                        .map(std::borrow::Cow::into_owned)
                        .filter(|name| !name.is_empty()),''','''                        .map(std::borrow::Cow::into_owned),''')
replace('.filter(|(v, _)| !v.is_empty() && (!defs.contains(v) || reads_own_def.contains(v)))','.filter(|(v, _)| !defs.contains(v) || reads_own_def.contains(v))')
replace('''            let name = normalise_var_name(name).to_owned();
            if !name.is_empty() {
                found.by_name.insert(name.clone());
                reads_own_def.insert(name);
            }''','''            let name = tcl_syntax::naming::normalise_var_name_braced(name, true).to_owned();
            found.by_name.insert(name.clone());
            reads_own_def.insert(name);''')
# Preserve the explicit detached standalone compatibility interpretation; empty
# is still a legal resulting name once this independent role route is selected.
replace('''            .map(|name| normalise_var_name(name).to_owned())
            && !name.is_empty()''','''            .map(|name| crate::naming::normalise_var_name(name).to_owned())''')
replace('if !norm.is_empty() && vars_found.contains(norm) {','if vars_found.contains(norm) {')
replace('''                if !norm.is_empty() {
                    by_name.insert(norm.to_owned());
                    reads_own_def.insert(norm.to_owned());
                }''','''                by_name.insert(norm.to_owned());
                reads_own_def.insert(norm.to_owned());''')
# Keep every existing fixture source unchanged while sharing its actual ingress.
replace('''    fn logical_role_metadata_fixture() -> (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        crate::compilation_unit::CompilationUnit,
    ) {
        use std::sync::Arc;''','''    fn logical_role_metadata_fixture() -> (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        crate::compilation_unit::CompilationUnit,
    ) {
        logical_metadata_fixture("info exists selected")
    }

    fn logical_metadata_fixture(source: &str) -> (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        crate::compilation_unit::CompilationUnit,
    ) {
        use std::sync::Arc;''')
replace('''        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            "info exists selected",''','''        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,''')
replace('''    #[test]
    fn uses_of_assign_const() {''','''    #[test]
    fn original_literal_empty_reads_and_mutations_keep_name_data() {
        // Native cases: experiments/name-handling/results/ssa-empty-read-write-names.
        let source = "info exists {}; info exists {$name}; incr {}; incr {$counter}; dict with {} {}; dict with {$dict} {}";
        let (context, unit) = logical_metadata_fixture(source);
        let registry = context.commands();
        let function = &unit.top_level;
        let metadata = function
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .expect("actual retained Logical source input");
        let selection = SsaInvocationContext::supplied(registry, Some(metadata));
        let mut scanner = VarReferenceScanner::with_config(
            VarScanOptions::default(),
            function.source_lexer_config(),
        );
        let statements = &unit.ir_module.top_level.statements;
        assert_eq!(statements.len(), 6);
        for (statement, expected) in statements
            .iter()
            .zip(["", "$name", "", "$counter", "", "$dict"])
        {
            let reads = uses_of_classified_in_context(statement, &mut scanner, selection);
            assert!(
                reads.contains(&(expected.to_owned(), UseClass::Name)),
                "{expected:?}: {reads:?}"
            );
            if let Some(unprefixed) = expected.strip_prefix('$') {
                assert!(!reads.iter().any(|(name, _)| name == unprefixed));
            }
        }
    }

    #[test]
    fn original_loop_name_lists_keep_empty_and_dollar_elements() {
        // Native foreach cases: experiments/name-handling/results/ssa-empty-read-write-names.
        let source = "foreach {{} {$name}} $values {}";
        let (context, unit) = logical_metadata_fixture(source);
        let registry = context.commands();
        let function = &unit.top_level;
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .expect("actual original loop words");
        let metadata = function
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .expect("actual retained Logical source input");
        let selected = selected_barrier_defs(
            SsaInvocationContext::supplied(registry, Some(metadata)),
            tokens,
        );
        assert!(selected.contains(&String::new()), "{selected:?}");
        assert!(selected.contains(&"$name".to_owned()), "{selected:?}");
        assert!(!selected.contains(&"name".to_owned()), "{selected:?}");
        assert!(
            selected_barrier_defs(SsaInvocationContext::supplied(registry, None), tokens)
                .is_empty()
        );
    }

    #[test]
    fn uses_of_assign_const() {''')
replace('''//! The full SSA rename pass ([`build_ssa`]) and variable-use scanner
//! ([`uses_of`]) are now implemented, completing the SSA construction
//! pipeline. The rename pass walks the dominator tree, assigns SSA''','''//! The SSA rename pass ([`build_ssa`]) and variable-use scanner ([`uses_of`])
//! walk the dominator tree, assigning SSA''')
p.write_text(s)
