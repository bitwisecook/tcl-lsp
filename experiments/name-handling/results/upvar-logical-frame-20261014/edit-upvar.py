from pathlib import Path
r=Path('/tmp/name-upvar-logical-frame-20261014/after/rust/tcl-compiler/src')
p=r/'registry_invocation.rs';s=p.read_text();s=s.replace('pub(crate) use logical_frame::original_logical_frame_invocation_with_metadata_context;','pub(crate) use logical_frame::{\n    OriginalLogicalFrameInvocation, original_logical_frame_invocation_with_metadata_context,\n};',1);p.write_text(s)
p=r/'cfg_builder/upvar_info.rs';s=p.read_text();s=s.replace('''    input: Option<&crate::analyser::ResolvedAnalysisInput>,
) -> UpvarInfo {
    let Some((input, metadata)) = input.and_then(|input| {
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, input)
            .map(|metadata| (input, metadata))
    })''','''    module: &crate::ir::Module,
) -> UpvarInfo {
    let Some((input, metadata)) = crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)
        .and_then(|metadata| metadata.source_analysis_input().map(|input| (input, metadata)))''',1)
s=s.replace('''        source: bindings,
        input,
        metadata,
''','''        source: bindings,
        input,
        metadata,
        module,
''',1)
s=s.replace('''struct UpvarBindings<'a> {
    source:''','''struct UpvarBindings<'a> {
    module: &'a crate::ir::Module,
    source:''',1)
# Retain the actual Script carrier at each recursive source boundary.
s=s.replace('''    stmts: &[Statement],''','''    script: &Script,''',1)
s=s.replace('''    for stmt in stmts {
        walk_stmt(stmt, params, registry, bindings, namespace, info);
''','''    for stmt in &script.statements {
        walk_stmt(stmt, script.retained_source_tokens_for_statement(stmt), params, registry, bindings, namespace, info);
''',1)
s=s.replace('''fn walk_stmt(
    stmt: &Statement,
''','''fn walk_stmt(
    stmt: &Statement,
    tokens: Option<&CommandTokens>,
''',1)
# All walk_script calls use owned Scripts rather than losing side-table carriers.
for name in ['body','c.body','e','init','next','b','h.body','f','module.top_level']:
 s=s.replace('&'+name+'.statements,', '&'+name+',')
# Body is already &Script at the function entry, and local variables may Box-deref.
s=s.replace('''    walk_script(
        &body,''','''    walk_script(
        body,''',1)
needle='''    match stmt {
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {'''
repl='''    if let Some(bindings) = bindings
        && bindings.input.has_logical_source_name_context()
        && matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. })
    {
        let selected = tokens.and_then(|tokens| {
            let metadata = tokens.source_binding.as_ref()?
                .original_invocation_metadata_for_module(tokens, bindings.module, registry)?;
            crate::registry_invocation::original_logical_frame_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
        });
        match (tokens, selected) {
            (Some(tokens), Some(selected)) => record_logical_frame_invocation(
                &selected, tokens, FrameContext::Own, params, registry, bindings,
                namespace, info, 0,
            ),
            _ => {
                info.caller_frame_opaque_writes = true;
                info.caller_frame_opaque_reads = true;
            }
        }
        return;
    }
    match stmt {
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {'''
assert needle in s;s=s.replace(needle,repl,1)
# Replace the incomplete fixture ingress with a genuine same-registry Logical request.
a=s.index('''    fn module_proc_info(src: &str, qname: &str) -> UpvarInfo {''');b=s.index('''        let bindings = ModuleCommandBindings::analyse''',a)
s=s[:a]+'''    fn module_proc_info(src: &str, qname: &str) -> UpvarInfo {
        let context = crate::environment_ingress::resolve_environment("tcl9.0")
            .default_context_registry();
        let registry = context.commands();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile, profile, std::sync::Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(registry, input.lexer_config())
                .with_resolved_analysis_input(input), src,
        );
'''+s[b:]
s=s.replace('''            module.source_metadata_input.as_ref(),
''','''            &module,
''',1)
p.write_text(s)
p=r/'cfg_builder/mod.rs';s=p.read_text();needle='''                crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)
                    .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input))''';assert needle in s;s=s.replace(needle,'''                module)''',1);p.write_text(s)
