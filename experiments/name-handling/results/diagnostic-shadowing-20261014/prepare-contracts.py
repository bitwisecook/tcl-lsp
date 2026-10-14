from pathlib import Path
import json,importlib.util,difflib,hashlib,subprocess
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-diagnostic-shadowing-20261014/contracts')
p='docs/design/analysis/name-resolution-proofs/manifest.json';before=(root/p).read_text();manifest=json.loads(before);records=[]
for identifier in ['naming.consumer.original-structural-diagrams','naming.compiler.original-analysis-metadata-context']:
 record=next(row for row in manifest['proofs'] if row['id']==identifier)
 if identifier.endswith('original-structural-diagrams'):
  record['conclusion']=record['conclusion'].replace('Independently selected lexical compatibility keeps its own source-advice branch.','Logical, authored-simulation and Execution/Standalone source calls retain distinct original producers. Every call joins a unanimous actual Module-issued ProcedureId and reports its complete bodyKey; no caller label or lexical existence table selects a body. The convenience diagram ingress uses the actual CU-retained input and ContextRegistry. A foreign issuer, duplicate body row, missing actual input or incomplete current source vector withdraws the join.')
  newowners=[
   {'path':'rust/tcl-compiler/src/source_graph.rs','symbol':'procedure_body_for_original_call','purpose':'Validate current complete analysis, actual Module/input/Registry and original token metadata; select unanimous issued body IDs through the retained Logical, authored-simulation or typed original command-reference allocation producer without name inversion or Native callable permission.'},
   {'path':'rust/tcl-compiler/src/source_graph.rs','symbol':'registry_words_for_original_call','purpose':'Read the shared sealed Registry source schema only when actual Module/input/source/Registry and original call vector agree; no caller-name reconstruction or execution/completion grant.'},
   {'path':'rust/tcl-compiler/src/source_graph.rs','symbol':'module_matches_original_call','purpose':'Require whole-image/configuration, the exact retained ResolvedAnalysisInput owner, actual Registry and genuine unsynthesized token correspondence before readonly source-call projection.'},
   {'path':'rust/tcl-diagram/src/data.rs','symbol':'procedure_target_at','purpose':'Require actual issued source body and independently retained original declaration allocation to agree, reporting full bodyKey and preserving missing-lookup refusal.'},
   {'path':'rust/tcl-diagram/src/data.rs','symbol':'diagram_data_for_dialect','purpose':'Reuse the actual compiled unit source input and availability at the authorised convenience ingress; missing metadata remains explicitly unavailable without fabricating Native entry.'}]
  newtest={'path':'rust/tcl-diagram/src/data.rs','selector':'data::original_source_tests::diagram_source_calls_keep_real_body_owners_and_refuse_foreign_ids','assertion':'Two original same-label declarations retain distinct actual issued body IDs and full body keys; real foreign-issued replacement, duplicate body row, missing input and changed source refuse the call join. Counterfactual reporting labels do not select calls.','status':'linked'}
  record['scope']+=' The actual body-owner/duplicate/foreign-ID controls are linked software assertions without an executed acceptance receipt.'
 else:
  record['conclusion']+=' Namespace shadowing for taint diagnostics uses original publication slots and the actual body namespace, without reading FunctionUnit display names. The shared helper receives the real CompilationUnit, checks actual function pointer membership and issuer ownership, and treats only the actual top-level pointer as Top. Unavailable namespace/publication ownership withdraws only the sink-warning projection; other diagnostic families remain independent.'
  newowners=[
   {'path':'rust/tcl-compiler/src/taint.rs','symbol':'shadowed_builtin_names_for_module_function','purpose':'Require actual CompilationUnit function membership, precise Top/body ownership and genuine Module-issued IDs before deriving protective builtin shadow names from original body namespace and publication slots.'},
   {'path':'rust/tcl-compiler/src/compiler_checks.rs','symbol':'push_taint_and_module_checks','purpose':'Use shared actual-body publication shadowing rather than FunctionUnit names; unavailable shadow advice skips only sink warnings while preserving independent setter/path/destructive-file diagnostics.'}]
  newtest={'path':'rust/tcl-compiler/src/taint.rs','selector':'taint::tests::shadowed_builtin_names_tests::retained_body_scope_ignores_display_and_refuses_detached_units','assertion':'Actual namespaced publication shadows puts despite counterfactual FunctionUnit label; detached Top/body clones and a genuine foreign Module-issued body ID cannot borrow the current namespace projection.','status':'linked'}
 for owner in newowners:
  prior=next((o for o in record['owners'] if o['path']==owner['path'] and o['symbol']==owner['symbol']),None)
  if prior is None:record['owners'].append(owner)
  else:prior.update(owner)
 if not any(t['selector']==newtest['selector'] for t in record['tests']):record['tests'].append(newtest)
 records.append(record)
# Reviewed records are independently mergeable against later Root manifest changes.
out.mkdir(parents=True,exist_ok=True);(out/'reviewed-records.json').write_text(json.dumps(records,indent=2,ensure_ascii=False)+'\n')
spec=importlib.util.spec_from_file_location('proof_renderer',root/'scripts/dev/update-name-resolution-proof-index.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
texts={p:json.dumps(manifest,indent=2,ensure_ascii=False)+'\n'}
for record in records:texts[record['document']]=module.render(record)
patch=''
for path,after in texts.items():
 b=(root/path).read_text()
 for stage,text in [('before',b),('after',after)]:
  target=out/stage/path;target.parent.mkdir(parents=True,exist_ok=True);target.write_text(text)
 patch+=''.join(difflib.unified_diff(b.splitlines(keepends=True),after.splitlines(keepends=True),fromfile='a/'+path,tofile='b/'+path))
(out/'implementation.patch').write_text(patch);r=subprocess.run(['git','apply','--check',str(out/'implementation.patch')],cwd=root,capture_output=True,text=True)
receipt={'validation':'Main apply-check; record schema validation pending; no Rust executed','patch_sha256':hashlib.sha256(patch.encode()).hexdigest(),'apply_check':r.returncode,'reviewed_records_sha256':hashlib.sha256((out/'reviewed-records.json').read_bytes()).hexdigest()}
(out/'manifest.json').write_text(json.dumps(receipt,indent=2)+'\n');print(len(patch),receipt['patch_sha256'],r.returncode,r.stderr)
