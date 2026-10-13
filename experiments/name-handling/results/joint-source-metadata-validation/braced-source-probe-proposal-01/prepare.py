from pathlib import Path
import hashlib,json,difflib,subprocess
root=Path('/workspace/tcl-lsp');out=Path('/tmp/braced-original-source-probe-01');out.mkdir(exist_ok=True)
paths=['rust/tcl-compiler/src/analyser/commands.rs','rust/tcl-irules/src/source_context.rs']; before={}
def replace(s,a,b):
 assert s.count(a)==1,(a[:80],s.count(a)); return s.replace(a,b)
for path in paths:
 s=(root/path).read_text();before[path]=hashlib.sha256(s.encode()).hexdigest();(out/(Path(path).name+'.before')).write_text(s)
 if path.endswith('commands.rs'):
  s=replace(s,'        let mut pending = vec![(native, None)];','        let trace = std::env::var_os("TCL_LSP_TRACE_BRACED_SOURCE").is_some();\n        let mut pending = vec![(native, None)];')
  s=replace(s,'            self.retain_original_static_source_vector(&native, body_origin.as_ref());','''            self.retain_original_static_source_vector(&native, body_origin.as_ref());
            if trace {
                let retained = native.first().and_then(|head| {
                    self.result.original_vendor_source_names.get(&head.span())
                });
                eprintln!("braced-source compiler vector head={:?} words={} body_origin={} retained={:?}", native.first().map(tcl_lexer::NativeWord::span), native.len(), body_origin.is_some(), retained.map(Option::is_some));
            }''')
  s=replace(s,'                let context = self.analysis_context();\n                for body in words.source_script_bodies_for(','''                let context = self.analysis_context();
                if trace {
                    let input = self.resolved_input.as_ref().or(self.result.resolved_input.as_ref());
                    let expressions = input.and_then(|input| words.source_expression_script_bodies(input));
                    eprintln!("braced-source compiler schema head={:?} roles={:?} retained_input={} expr_issuer={:?}", native.first().map(tcl_lexer::NativeWord::span), words.roles(), input.is_some(), expressions.as_ref().map(Vec::len));
                    for body in expressions.iter().flatten() {
                        eprintln!("braced-source compiler expr child={:?} container={:?} source={} context={} vendor_origin={} queued=false", body.content_span(), body.original_container().span(), body.matches_source(&image, config), body.matches_context(&context), body.vendor_origin().is_some());
                    }
                }
                for body in words.source_script_bodies_for(''')
  s=replace(s,'                    let Some(origin) = body.vendor_origin() else {\n                        continue;\n                    };','''                    let Some(origin) = body.vendor_origin() else {
                        if trace { eprintln!("braced-source compiler syntax child={:?} vendor_origin=false queued=false", body.content_span()); }
                        continue;
                    };''')
  s=replace(s,'                        pending.extend(\n                            plan.commands','''                        if trace { eprintln!("braced-source compiler syntax child={:?} queued={} fatal_tail={}", body.content_span(), plan.commands.len(), plan.fatal_tail.is_some()); }
                        pending.extend(
                            plan.commands''')
  s=replace(s,'                    // A malformed tail supplies no complete vector; any','''                    if trace { eprintln!("braced-source compiler lexical child={:?} queued={} fatal_tail={}", body, plan.commands.len(), plan.fatal_tail.is_some()); }
                    // A malformed tail supplies no complete vector; any''')
  s=replace(s,'                }\n            }\n            for word in &native {','''                }
            } else if trace {
                eprintln!("braced-source compiler schema head={:?} readonly=false", native.first().map(tcl_lexer::NativeWord::span));
            }
            for word in &native {''')
 else:
  s=replace(s,'    let mut vectors = Vec::new();\n    for occurrence','    let trace = std::env::var_os("TCL_LSP_TRACE_BRACED_SOURCE").is_some();\n    let mut vectors = Vec::new();\n    for occurrence')
  s=replace(s,'        if first != occurrence.name_input().original_word() {\n            continue;','''        if first != occurrence.name_input().original_word() {
            if trace { eprintln!("braced-source context vector head={:?} original_head=false", first.span()); }
            continue;''')
  s=replace(s,'        if segments.len() != 1 {\n            continue;','''        if segments.len() != 1 {
            if trace { eprintln!("braced-source context vector head={:?} segments={} admitted=false", first.span(), segments.len()); }
            continue;''')
  s=replace(s,'        let Some(words) = source_registry_words(source, analysis, &segment) else {\n            continue;','''        let Some(words) = source_registry_words(source, analysis, &segment) else {
            if trace { eprintln!("braced-source context vector head={:?} source_registry_words=false admitted=false", first.span()); }
            continue;''')
  s=replace(s,'        if words.matches_source(image, config)\n','''        if trace { eprintln!("braced-source context vector head={:?} source_registry_words=true source={} registry={} context={}", first.span(), words.matches_source(image, config), words.matches_registry(context.commands()), words.context() == Some(context.context())); }
        if words.matches_source(image, config)
''')
  s=replace(s,'            vectors.push((segment.span, words));','''            if trace { eprintln!("braced-source context vector span={:?} admitted=true", segment.span); }
            vectors.push((segment.span, words));''')
  s=replace(s,'    let mut visited = HashSet::new();\n    let mut commands = Vec::new();','''    let trace = std::env::var_os("TCL_LSP_TRACE_BRACED_SOURCE").is_some();
    let mut visited = HashSet::new();
    let mut commands = Vec::new();''')
  s=replace(s,'                continue;\n            }\n            let mut obligations','''                if trace { eprintln!("braced-source context command span={:?} body={:?} excluded=true", span, body); }
                continue;
            }
            if trace { eprintln!("braced-source context command span={:?} body={:?} event={} through_call={} admitted=true", span, body, event, through_call); }
            let mut obligations''')
 dest=out/Path(path).name;dest.write_text(s)
 subprocess.run(['rustfmt','--edition','2024',str(dest)],check=True)
changes=[]
for path in paths:
 old=(out/(Path(path).name+'.before')).read_text();new=(out/Path(path).name).read_text()
 changes.extend(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path))
(out/'probe.patch').write_text(''.join(changes))
(out/'readiness.json').write_text(json.dumps({'preimages':before,'scratch_images':{p:hashlib.sha256((out/Path(p).name).read_bytes()).hexdigest() for p in paths},'original_control':'walker::tests::braced_expr_command_substitution_keeps_its_data_group_reference','environment_gate':'TCL_LSP_TRACE_BRACED_SOURCE','production_applied':False,'builds':0,'tests_executed':0,'expression_children_enqueued':False},indent=2)+'\n')
assert all(hashlib.sha256((root/p).read_bytes()).hexdigest()==h for p,h in before.items())
print((out/'readiness.json').read_text())
