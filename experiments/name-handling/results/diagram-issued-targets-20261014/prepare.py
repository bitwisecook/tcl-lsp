from pathlib import Path
import re
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-diagram-issued-targets-20261014')
paths=['rust/tcl-compiler/src/source_graph.rs','rust/tcl-diagram/src/data.rs']
for p in paths:
 before=(root/p).read_text();after=before
 if p.endswith('source_graph.rs'):
  after=after.replace('    let first = bodies.next()?;\n    bodies.next().is_none().then_some(first)','    let first = bodies.next()?;\n    module.procedure_ids.owns(first.0).then_some(())?;\n    bodies.next().is_none().then_some(first)',1)
  marker='/// The actual Registry-selected event declaration belonging to this procedure.'
  addition='''/// Select a unanimous source call body from the actual Module's original words.
/// The issued ID supplies readonly reachability, without Native dispatch,
/// argument evaluation, frame, completion or erasure permission.
#[must_use]
pub fn procedure_body_for_original_call<'a>(
    source: &str,
    analysis: &AnalysisResult,
    module: &'a Module,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<(ProcedureId, &'a Procedure)> {
    module_matches_original_call(source, analysis, module, tokens, registry).then_some(())?;
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    let ids = match binding.original_invocation_source_purpose(tokens, metadata, registry)? {
        crate::command_binding::OriginalInvocationSourcePurpose::Logical => {
            crate::registry_invocation::original_logical_procedure_calls_for_module(tokens, module, registry)?
                .into_iter().map(|call| call.id()).collect::<Vec<_>>()
        }
        crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation => {
            crate::registry_invocation::original_authored_procedure_calls_for_module(tokens, module, registry)?
                .into_iter().map(|call| call.id()).collect::<Vec<_>>()
        }
        crate::command_binding::OriginalInvocationSourcePurpose::Execution
        | crate::command_binding::OriginalInvocationSourcePurpose::Standalone => {
            let site = binding.invocation_site()?;
            let config = binding.original_lexer_config_for_tokens(tokens)?;
            let words = crate::registry_invocation::original_native_compiler_words(
                &module.source, tokens.words(), site.offset, config)?;
            let head = String::from_utf8(tcl_syntax::word_rules::original_static_word_source_bytes(words.first()?)?).ok()?;
            crate::registry_invocation::original_source_procedure_ids_for_word_for_module(tokens, &head, module, registry)?
        }
    };
    let id = *ids.first()?;
    ids.iter().all(|candidate| *candidate == id).then_some(())?;
    module.procedure_ids.owns(id).then_some(())?;
    Some((id, module.procedures.get(&id)?))
}

/// Read the shared original Registry source schema for this actual Module call.
/// Printed command names and independently reconstructed namespaces cannot
/// select a schema or supply Native invocation or completion permission.
#[must_use]
pub fn registry_words_for_original_call(
    source: &str,
    analysis: &AnalysisResult,
    module: &Module,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
    module_matches_original_call(source, analysis, module, tokens, registry).then_some(())?;
    crate::registry_invocation::source_structure::original_registry_words_for_tokens(source, analysis, tokens)
}

fn module_matches_original_call(
    source: &str,
    analysis: &AnalysisResult,
    module: &Module,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let Some((image, config)) = current_analysis(source, analysis) else { return false; };
    module.source == image
        && module.lexer_config == config
        && module.source_metadata_input.as_ref().is_some_and(|input| analysis.resolved_input.as_ref() == Some(input))
        && module.registry_snapshot.as_ref().is_some_and(|snapshot| snapshot.semantic_key() == registry.snapshot().semantic_key())
        && tokens.synthetic.is_none()
        && tokens.source_binding.as_ref().is_some_and(|binding|
            binding.original_invocation_metadata_for_module(tokens, module, registry).is_some())
}

'''
  assert marker in after;after=after.replace(marker,addition+marker,1)
 else:
  after=after.replace('use tcl_compiler::interprocedural::{namespace_parts_from_proc, resolve_internal_call};\n','')
  after=after.replace('use tcl_compiler::realm::{CommandBindingRealm, RealmBinding};\n','')
  after=after.replace('''/// source graph owner. The explicitly selected lexical compatibility view
/// retains its independent resolver; printed names are labels in all views.''','''/// source graph owner and the actual Module issuer. Printed names are labels.''')
  after=after.replace("    procedure_names: &'a HashSet<String>,\n    identities: &'a CommandBindingRealm,", "    module: &'a Module,")
  start=after.index('/// The registry command a source head denotes at one exact source position.')
  end=after.index('/// Unique source declaration allocation at this authentic current call site.',start)
  after=after[:start]+'''/// Shared original Registry source schema at an authentic current call site.
fn registry_command(
    at: u32,
    tokens: Option<&CommandTokens>,
    context: &DiagramContext<'_>,
) -> Option<String> {
    let tokens = tokens?;
    if tokens.source_binding.as_ref()?.invocation_site()?.offset != at {
        return None;
    }
    let selected = tcl_compiler::source_graph::registry_words_for_original_call(
        context.source, context.analysis, context.module, tokens, context.registry)?;
    if !selected.matches_registry(context.registry)
        || selected.context() != Some(context.availability.context()) {
        return None;
    }
    Some(selected.command().to_owned())
}

struct ProcedureTarget {
    id: tcl_compiler::ir::ProcedureId,
    declaration: Option<usize>,
}

/// Join the original call's issued body and its independent source declaration.
fn procedure_target_at(
    at: u32,
    tokens: Option<&CommandTokens>,
    context: &DiagramContext<'_>,
) -> Option<ProcedureTarget> {
    let tokens = tokens?;
    if tokens.source_binding.as_ref()?.invocation_site()?.offset != at {
        return None;
    }
    let (id, _) = tcl_compiler::source_graph::procedure_body_for_original_call(
        context.source, context.analysis, context.module, tokens, context.registry)?;
    let declaration = if context.analysis.allows_lexical_declaration_advice() {
        None
    } else {
        let ordinal = original_procedure_target_at(at, context)?;
        let declaration = context.analysis.original_procedure_declarations().nth(ordinal)?;
        let (declared_id, _) = tcl_compiler::source_graph::procedure_body_for_declaration(
            context.source, context.analysis, context.module, declaration)?;
        (declared_id == id).then_some(())?;
        Some(ordinal)
    };
    Some(ProcedureTarget {id, declaration})
}

'''+after[end:]
  old='''    // The lowerer records the target of a statically-known alias. Plain calls
    // deliberately retain their source spelling; `resolve_internal_call`
    // below gives those Tcl's caller-namespace lookup rather than treating
    // equal short tails as interchangeable identities.
'''
  assert old in after;after=after.replace(old,'')
  after=after.replace('    if is_procedure_call(command, canonical_command, caller_qname, at, context) {','    if let Some(target) = procedure_target_at(at, tokens, context) {',1)
  old='''            "declaration_id": (!context.analysis.allows_lexical_declaration_advice())
                .then(|| original_procedure_target_at(at, context))
                .flatten().map(|ordinal| format!("declaration-{ordinal}")),'''
  new='''            "bodyId": target.id.ordinal(),
            "bodyKey": target.id.reporting_key(),
            "declaration_id": target.declaration.map(|ordinal| format!("declaration-{ordinal}")),'''
  assert old in after;after=after.replace(old,new)
  old='''    let registry_command = registry_command(
        command,
        canonical_command,
        caller_qname,
        at,
        tokens,
        context,
    );''';assert old in after;after=after.replace(old,'    let registry_command = registry_command(at, tokens, context);')
  # No walk consumes a caller label after original token selection.
  after=re.sub(r'^\s+caller_qname: &str,\n','',after,flags=re.M)
  after=re.sub(r'^\s+caller_qname,\n','',after,flags=re.M)
  after=after.replace(', caller_qname,', ',')
  after=after.replace(', &procedure.qualified_name,', ',')
  after=after.replace('''    let procedure_names = regular_procs
        .iter()
        .map(|(_, procedure)| procedure.qualified_name.clone())
        .collect();
''','')
  after=after.replace('''        procedure_names: &procedure_names,
        identities: analysis
            .retained_command_realm()
            .expect("current analysis retains its realm"),''','''        module,''',1)
  # The authorised convenience seam uses the very input retained by its CU.
  start=after.index('    let availability = std::sync::Arc::new(', after.index('pub fn diagram_data_for_dialect('))
  end=after.index('    analyser.set_cu_override(',start)
  after=after[:start]+'''    let cu = CompilationUnit::build_for_profile(source, registry, false, dialect);
    let Some(input) = cu.ir_module.source_metadata_input.clone() else {
        return json!({"events": [], "procedures": [], "unavailable": "current-source-input"});
    };
    let availability = input.context_registry();
    let mut analyser = Analyser::new().with_resolved_input(input);
'''+after[end:]
  # Same exact input/owner transport for the original test setup.
  start=after.index('        let context = tcl_registry::model::ingress::context_for_profile(profile);',after.index('    fn analysed('))
  end=after.index('        analyser.set_cu_override(',start)
  after=after[:start]+'''        let initial = tcl_registry::model::ingress::context_for_profile(profile);
        let unit = CompilationUnit::build_for_profile(source, initial.commands(), false, profile);
        let input = unit.ir_module.source_metadata_input.clone().expect("actual fixture source input");
        let context = input.context_registry();
        let mut analyser = Analyser::new().with_resolved_input(input);
'''+after[end:]
  after=after.replace('        let names = HashSet::from(["counterfactual".to_owned()]);\n','')
  after=after.replace('            let names = HashSet::new();\n','')
  after=after.replace('            identities: analysis.retained_command_realm().unwrap(),\n            procedure_names: &names,','            module: &unit.ir_module,')
  after=after.replace('                    procedure_names: &names,\n                    identities: analysis.retained_command_realm().unwrap(),','                    module: &unit.ir_module,')
  marker='        let at = u32::try_from(source.find("; target").unwrap() + 2).unwrap();'
  replacement=marker+'''
        let tokens = unit.ir_module.top_level.statements.iter().find_map(|statement| {
            let tokens = unit.ir_module.top_level.retained_source_tokens_for_statement(statement)?;
            (tokens.source_binding.as_ref()?.invocation_site()?.offset == at).then_some(tokens)
        }).expect("actual original call tokens");'''
  assert marker in after;after=after.replace(marker,replacement,1)
  old='''        assert!(is_procedure_call(
            "counterfactual",
            None,
            "counterfactual",
            at,
            &context
        ));'''
  assert old in after;after=after.replace(old,'        assert!(procedure_target_at(at, Some(tokens), &context).is_some());')
  old='''        assert!(!is_procedure_call(
            "target",
            Some("::target"),
            "::",
            at,
            &context
        ));'''
  assert old in after;after=after.replace(old,'        assert!(procedure_target_at(at, Some(tokens), &context).is_none());')
  after=after.replace('        let _ = unit;\n','')
  after=after.replace('registry_command("counterfactual", None, "::", 0, Some(tokens), &context)','registry_command(0, Some(tokens), &context)')
  assert 'caller_qname' not in after
 for stage,text in [('before',before),('after',after)]:
  path=out/stage/p;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(text)
