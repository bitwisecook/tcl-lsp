from pathlib import Path
import subprocess,difflib,json,hashlib
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-diagnostic-shadowing-20261014')
paths=['rust/tcl-compiler/src/compiler_checks.rs','rust/tcl-compiler/src/taint.rs','rust/tcl-compiler/src/interprocedural.rs','rust/tcl-compiler/src/unit_scope.rs','rust/tcl-compiler/src/optimiser/helpers/naming.rs','rust/tcl-compiler/src/compilation_unit.rs']
for p in paths:
 before=(root/p).read_text();after=before
 if p.endswith('compiler_checks.rs'):
  start=after.index('    let proc_qnames: std::collections::HashSet<&str> = cu');end=after.index('    // `trace add variable`',start);after=after[:start]+after[end:]
  start=after.index('        // A namespace-scoped proc of the same name shadows a builtin');end=after.index('        // Conditional setter metadata follows',start)
  after=after[:start]+'''        if let Some(shadowed) =
            crate::taint::shadowed_builtin_names_for_module_function(fu, cu, registry)
        {
            for warning in find_taint_warnings_for_module_function(
                fu,
                &taints,
                &fu.sccp.executable_blocks,
                registry,
                &cu.ir_module,
                &shadowed,
                module_traces,
            ) {
                out.push(shift(fu, Diagnostic::from_taint(&warning)));
            }
        }
'''+after[end:]
 elif p.endswith('taint.rs'):
  after=after.replace('shadowed_builtin_names_for_module_function(fu, &cu.ir_module, registry)','shadowed_builtin_names_for_module_function(fu, cu, registry)',1)
  start=after.rfind('///',0,after.index('pub fn shadowed_builtin_names<'))
  # Replace the complete obsolete public helper doc, preserving all fixture logic.
  start=after.index('/// Names of builtin commands shadowed',0) if '/// Names of builtin commands shadowed' in after else after.rfind('\n/// ',0,after.index('/// {…}` shadows'))
  # Locate its exact doc block from the prior non-doc line.
  target=after.index('pub fn shadowed_builtin_names<');lines=after[:target].splitlines(keepends=True)
  cut=len(lines)
  while cut and (lines[cut-1].startswith('///') or lines[cut-1].startswith('#[must_use]')):cut-=1
  docstart=sum(map(len,lines[:cut]))
  after=after[:docstart]+'''/// Constructed publication labels used by the isolated lookup fixtures.
#[cfg(test)]
#[must_use]
'''+after[target:]
  after=after.replace('pub fn shadowed_builtin_names<','fn shadowed_builtin_names<',1)
  after=after.replace('   [`shadowed_builtin_names`].','   [`shadowed_builtin_names_for_module_function`].',1)
  after=after.replace('/// `shadowed_builtins` (see [`shadowed_builtin_names`]) lists the bare','/// `shadowed_builtins` (see [`shadowed_builtin_names_for_module_function`]) lists the bare',1)
  old='''    module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> Option<HashSet<String>> {
    use crate::command_binding::SourceNamespaceKey;'''
  new='''    unit: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> Option<HashSet<String>> {
    if !unit.all_body_function_units().any(|owned| std::ptr::eq(owned, function)) {
        return None;
    }
    let module = &unit.ir_module;
    use crate::command_binding::SourceNamespaceKey;'''
  assert old in after;after=after.replace(old,new,1)
  old='''    let namespace = if let Some(id) = function.body_id {
        if let Some(procedure) = module'''
  new='''    let namespace = if let Some(id) = function.body_id {
        module.procedure_ids.owns(id).then_some(())?;
        if let Some(procedure) = module'''
  assert old in after;after=after.replace(old,new,1)
  old='''    } else if let Some(context) = module.top_level_namespace_context.as_ref() {
        path(context)?
    } else if module.source_entry.metadata_context.is_standalone() {
        tcl_core_types::ByteNamespacePath::root()
    } else {
        return None;
    };'''
  new='''    } else {
        std::ptr::eq(function, &unit.top_level).then_some(())?;
        if let Some(context) = module.top_level_namespace_context.as_ref() {
            path(context)?
        } else if module.source_entry.metadata_context.is_standalone() {
            tcl_core_types::ByteNamespacePath::root()
        } else {
            return None;
        }
    };'''
  assert old in after;after=after.replace(old,new,1)
  after=after.replace('''    for procedure in module.procedures.values() {
        let slot = procedure.publication.as_ref()?.slot();''','''    for (id, procedure) in &module.procedures {
        module.procedure_ids.owns(*id).then_some(())?;
        let slot = procedure.publication.as_ref()?.slot();''',1)
  marker='''    mod shadowed_builtin_names_tests {
        use super::*;
''';test='''
        #[test]
        fn retained_body_scope_ignores_display_and_refuses_detached_units() {
            let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let mut unit = crate::compilation_unit::CompilationUnit::build_for_profile(
                "namespace eval ns {proc puts {args} {}; proc run {value} {puts $value}}",
                &registry, false, profile);
            let id = crate::test_body_identity::fixture_id(&unit.procedures, "::ns::run");
            let function = unit.procedures.get_mut(&id).unwrap();
            function.name = "::counterfactual::run".to_owned();
            assert_eq!(shadowed_builtin_names_for_module_function(
                unit.procedures.get(&id).unwrap(), &unit, &registry),
                Some(HashSet::from(["puts".to_owned()])));
            let detached_top = unit.top_level.clone();
            assert!(shadowed_builtin_names_for_module_function(
                &detached_top, &unit, &registry).is_none());
            let detached_body = unit.procedures.get(&id).unwrap().clone();
            assert!(shadowed_builtin_names_for_module_function(
                &detached_body, &unit, &registry).is_none());
            let foreign_unit = crate::compilation_unit::CompilationUnit::build_for_profile(
                "proc foreign {} {}", &registry, false, profile);
            let foreign = crate::test_body_identity::fixture_id(&foreign_unit.procedures, "::foreign");
            unit.procedures.get_mut(&id).unwrap().body_id = Some(foreign);
            assert!(shadowed_builtin_names_for_module_function(
                unit.procedures.get(&id).unwrap(), &unit, &registry).is_none());
        }
'''
  assert marker in after;after=after.replace(marker,marker+test,1)
 elif p.endswith('interprocedural.rs'):
  for name in ['resolve_internal_call_with','resolve_internal_call','resolve_call_target','namespace_parts_from_proc']:
   marker=('pub(crate) fn ' if name=='resolve_internal_call_with' else 'pub fn ')+name
   assert marker in after;after=after.replace(marker,'#[cfg(test)]\nfn '+name,1)
 elif p.endswith('unit_scope.rs'):
  start=after.index('/// Each call site is resolved to its callee via');end=after.index('pub(crate) fn collect_call_site_constants(',start)
  after=after[:start]+'''/// Original call allocations join the Module's issued body coordinates.
/// Unknown dispatch remains in the caller evidence and withdraws parameter seeds.
'''+after[end:]
  start=after.index('    // The top level has no qualified name of its own;');end=after.index('    let round = |previous:',start);after=after[:start]+after[end:]
 elif p.endswith('optimiser/helpers/naming.rs'):
  after=after.replace('''/// single separator), shared with the interprocedural pass's
/// `namespace_parts_from_proc` so the two cannot drift.''','''/// single separator).''',1)
 else:
  start=after.index('        /// FN regression: a call reached only via a `namespace import` alias');end=after.index('        #[test]',start)
  after=after[:start]+'''        /// Imported calls retain the original implementation allocation and
        /// contribute their differing arguments to the shared caller evidence.
'''+after[end:]
 for stage,text in [('before',before),('after',after)]:
  dest=out/stage/p;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(text)
 env={'RUSTUP_HOME':'/workspace/.rustup','CARGO_HOME':'/workspace/.cargo','PATH':'/usr/bin:/bin'}
 subprocess.run(['/workspace/.cargo/bin/rustfmt','--edition','2024','--config','skip_children=true',str(out/'after'/p)],env=env,check=True)
patch=''.join(''.join(difflib.unified_diff((out/'before'/p).read_text().splitlines(keepends=True),(out/'after'/p).read_text().splitlines(keepends=True),fromfile='a/'+p,tofile='b/'+p)) for p in paths)
(out/'implementation.patch').write_text(patch);r=subprocess.run(['git','apply','--check',str(out/'implementation.patch')],cwd=root,capture_output=True,text=True)
manifest={'validation':'rustfmt parsed; no Cargo or assertions executed','apply_check':r.returncode,'patch_sha256':hashlib.sha256(patch.encode()).hexdigest(),'before':{p:hashlib.sha256((out/'before'/p).read_bytes()).hexdigest() for p in paths},'after':{p:hashlib.sha256((out/'after'/p).read_bytes()).hexdigest() for p in paths}}
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(len(patch),manifest['patch_sha256'],r.returncode,r.stderr)
