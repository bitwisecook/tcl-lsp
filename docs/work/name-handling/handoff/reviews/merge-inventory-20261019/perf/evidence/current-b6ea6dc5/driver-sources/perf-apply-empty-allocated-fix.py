from pathlib import Path
import subprocess,json,hashlib,time
root=Path('/workspace/tcl-lsp-diagnostic');ev=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence');final=json.loads((ev/'current-b6ea6dc5-native35-unit-trace/final.json').read_text());assert final['api_original_restored'] and final['probe_removed'] and final['executables_removed'];assert not (root/'rust/tcl-compiler/src/analyser/authenticated_report/zz_native35_traceprobe.rs').exists()
p=root/'rust/tcl-compiler/src/analyser/authenticated_report/body_correspondence.rs';before=p.read_text();old='''source.is_empty()
                        && Self::empty_installed_body(module, registry, id, &body).is_none()''';new=old+'''
                        && Self::empty_allocated_declaration_body(module, registry, id, &body)
                            .is_none()''';assert before.count(old)==1;s=before.replace(old,new)
a=before.index('    fn empty_installed_body(');b=before.index('\n    #[cfg(test)]\n    pub(super) fn hosted_domain_for_test',a);helper=before[a:b];helper=helper.replace('fn empty_installed_body(', 'fn empty_allocated_declaration_body(').replace('ReportBodyRole::Unit(ProcedureBodyKind::Installed)','ReportBodyRole::Unit(ProcedureBodyKind::OriginalDeclaration)').replace('body.declaration_purpose.is_none()', 'body.declaration_purpose == Some(ReportDeclarationPurpose::Allocated)').replace('module.installed_procedure_body_units.get(&id)?','module.original_declaration_body_units.get(&id)?').replace('held.body_kind == ProcedureBodyKind::Installed','held.body_kind == ProcedureBodyKind::OriginalDeclaration').replace('.installed_procedure_body_units()', '.original_declaration_body_units()')
anchor='    #[cfg(test)]\n    pub(super) fn hosted_domain_for_test';assert s.count(anchor)==1;s=s.replace(anchor,'    // An empty allocated declaration keeps its original source and namespace\n    // through the complete current declaration card.\n'+helper+'\n'+anchor)
anchor='    #[test]\n    fn empty_installed_body_requires_the_current_complete_source_card() {';assert s.count(anchor)==1
newtests='''    fn allocated_empty() -> super::super::AuthenticatedAnalysisReport {
        let source = r"proc fib {n} {
    if {$n < 2} { return $n }
    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]
}
proc empty {} {}
puts [fib 20]
";
        let report = Analyser::new().analyse_for_report(source, "tcl8.6");
        assert!(report.current_capture().is_ok(), "genuine allocated empty body");
        report
    }

    fn allocated_empty_body(module: &crate::ir::Module) -> ReportBodyOwner {
        let (&id, _) = module.original_declaration_body_units.iter().find(|(id, _)| {
            module.body_units.get(id).is_some_and(|body| body.span.is_empty())
        }).expect("actual allocated empty declaration");
        ReportBodyInventory::procedure(module, id, module.body_units.get(&id).unwrap(),
            ReportBodyRole::Unit(ProcedureBodyKind::OriginalDeclaration)).unwrap()
    }

    #[test]
    fn empty_allocated_body_requires_the_current_complete_source_card() {
        let left = allocated_empty();
        let right = allocated_empty();
        assert_eq!(left.compare(&right), super::super::AnalysisReportComparison::Equal);
        let module = &left.current_capture().unwrap().bodies.prepared.unit().ir_module;
        let registry = left.analysis.resolved_registry().unwrap();
        let body = allocated_empty_body(module);
        let id = body.id;
        assert_eq!(body.declaration_purpose, Some(ReportDeclarationPurpose::Allocated));
        assert!(ReportBodyInventory::empty_allocated_declaration_body(module, registry, id, &body).is_some());
        let mut missing = module.clone();
        missing.original_declaration_body_units.remove(&id);
        assert!(ReportBodyInventory::empty_allocated_declaration_body(&missing, registry, id, &body).is_none());
        let mut no_owner = module.clone();
        no_owner.retained_source_bindings = None;
        assert!(ReportBodyInventory::empty_allocated_declaration_body(&no_owner, registry, id, &body).is_none());
        let foreign = &right.current_capture().unwrap().bodies.prepared.unit().ir_module;
        assert!(ReportBodyInventory::empty_allocated_declaration_body(foreign, registry, id, &body).is_none());
    }

    #[test]
    fn empty_allocated_body_rejects_wrong_roles_geometry_and_duplicates() {
        let report = allocated_empty();
        let module = &report.current_capture().unwrap().bodies.prepared.unit().ir_module;
        let registry = report.analysis.resolved_registry().unwrap();
        let body = allocated_empty_body(module);
        let id = body.id;
        let mut wrong_role = allocated_empty_body(module);
        wrong_role.role = ReportBodyRole::Unit(ProcedureBodyKind::Installed);
        assert!(ReportBodyInventory::empty_allocated_declaration_body(module, registry, id, &wrong_role).is_none());
        let mut wrong_purpose = allocated_empty_body(module);
        wrong_purpose.declaration_purpose = Some(ReportDeclarationPurpose::Native);
        assert!(ReportBodyInventory::empty_allocated_declaration_body(module, registry, id, &wrong_purpose).is_none());
        let mut wrong_span = allocated_empty_body(module);
        wrong_span.declaration = Span::empty(0);
        assert!(ReportBodyInventory::empty_allocated_declaration_body(module, registry, id, &wrong_span).is_none());
        let mut changed = module.clone();
        changed.body_units.get_mut(&id).unwrap().body_offset += 1;
        assert!(ReportBodyInventory::empty_allocated_declaration_body(&changed, registry, id, &allocated_empty_body(&changed)).is_none());
        let mut out_of_range = allocated_empty_body(module);
        out_of_range.declaration = Span::empty(u32::try_from(module.source.bytes().len() + 1).unwrap());
        assert!(ReportBodyInventory::push(&mut Vec::new(), module, registry, id, out_of_range).is_none());
        let mut bodies = Vec::new();
        assert!(ReportBodyInventory::push(&mut bodies, module, registry, id, allocated_empty_body(module)).is_some());
        assert!(ReportBodyInventory::push(&mut bodies, module, registry, id, allocated_empty_body(module)).is_none());
        assert_eq!(bodies.len(), 1);
    }

''';s=s.replace(anchor,newtests+anchor);p.write_text(s)
subprocess.run(['git','apply','/workspace/tcl-lsp-publish/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/perf-report-empty-recursive-call-repro.patch'],cwd=root,check=True)
files=[p,root/'rust/tcl-compiler/src/analyser/authenticated_report.rs'];subprocess.run(['rustfmt','--edition','2024','--config','skip_children=true',*map(str,files)],check=True)
out=ev/'current-b6ea6dc5-empty-allocated-fix';out.mkdir(exist_ok=False);(out/'inputs.json').write_text(json.dumps({'started_unix':time.time(),'root':str(root),'base':'b6ea6dc5ebaa9589f6e1c1747875127fecdbb573','scope':'Candidate actual-report API repair only: add complete original allocation-card admission for empty allocated declaration body; installed helper/guards and all existing assertions unchanged. No program-analysis or global/runtime/cache equality change. Full trace first refusal is body push for allocated OriginalDeclaration empty span.','validation':'scoped rustfmt/parser only; tests/build pending'},indent=2)+'\n');(out/'candidate-rust-diff.patch').write_bytes(subprocess.check_output(['git','diff','--','rust'],cwd=root));(out/'body-correspondence-before.sha256').write_text(hashlib.sha256(before.encode()).hexdigest()+'\n');print('Applied candidate ONLY in isolated diagnostic tree; pending build and original+new controls')
