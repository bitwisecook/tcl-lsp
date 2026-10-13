import pathlib, subprocess, os, json, hashlib, gzip, shutil, sys, time
root=pathlib.Path('/workspace/tcl-lsp')
base=root/'experiments/name-handling/results/runtime-c86-real-suites'
outs={'parse':base/'parse-format-active-call-lifetime/validation-01','glob':base/'glob-type-union-runtime/validation-01'}
for out in outs.values():out.mkdir(parents=True,exist_ok=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
ready={'runtime/rust/src/cmd_clock.rs':'668d1f282cc3ed8d94a84da72b1cbc8ab41bbe258c917b95ba83f53f36cde711','runtime/rust/src/cmd_fs.rs':'23e477126275515b9ce5fcf0384cdeb8fc657cce895888c720262daaa4187c73'}
for path,digest in ready.items():assert sha(root/path)==digest,(path,'not frozen ready')
toolbin=root/'tmp/name-tools/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin'
env=os.environ.copy();env['CARGO_HOME']=str(root/'tmp/name-tools/cargo');env['RUSTUP_HOME']=str(root/'tmp/name-tools/rustup');env['CARGO_TARGET_DIR']=str(root/'tmp/name-target-runtime');env['PATH']=str(toolbin)+':'+str(root/'tmp/name-tools/cargo/bin')+':'+env.get('PATH','')
metadata_argv=[str(toolbin/'cargo'),'metadata','--offline','--manifest-path','runtime/rust/Cargo.toml','--format-version=1']
mr=subprocess.run(metadata_argv,cwd=root,env=env,capture_output=True)
for out in outs.values():
 (out/'metadata.stderr.txt').write_bytes(mr.stderr)
 (out/'runtime-cargo-metadata.json.gz').write_bytes(gzip.compress(mr.stdout,mtime=0))
assert mr.returncode==0,mr.stderr.decode(errors='replace')
m=json.loads(mr.stdout);pkgs={p['id']:p for p in m['packages']};nodes={n['id']:n for n in m['resolve']['nodes']};rid=m['resolve']['root'];todo=[rid];seen=set()
while todo:
 p=todo.pop()
 if p in seen:continue
 seen.add(p)
 todo.extend(d['pkg'] for d in nodes[p]['deps'] if p==rid or any(k['kind'] in (None,'build') for k in d['dep_kinds']))
local=[pkgs[p] for p in seen if pathlib.Path(pkgs[p]['manifest_path']).is_relative_to(root) and not pathlib.Path(pkgs[p]['manifest_path']).is_relative_to(root/'tmp')]
assert len(local)==14,sorted(p['name'] for p in local)
fixtures=['experiments/name-handling/results/runtime-c86-real-suites/glob-type-union-question/capture-01/8.6.18/stdout.txt','experiments/name-handling/results/runtime-c86-real-suites/glob-type-union-rwx-question/capture-01/8.6.18/stdout.txt','experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-question/input.tcl','experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-question/capture-01/8.6.18/values/stdout.txt']
def snapshot():
 selected={}
 for p in local:
  directory=pathlib.Path(p['manifest_path']).parent
  for path in directory.rglob('*'):
   if path.is_file() and (path.suffix in ('.rs','.c','.h','.tcl','.txt','.json') or path.name in ('Cargo.toml','Cargo.lock')) and not any(x in ('target','.git') for x in path.relative_to(directory).parts):selected[str(path.relative_to(root))]=sha(path)
 for rel in fixtures+['Cargo.toml','Cargo.lock','runtime/rust/Cargo.lock','.cargo/config.toml','rust-toolchain.toml']:
  if (root/rel).is_file():selected[rel]=sha(root/rel)
 return dict(sorted(selected.items()))
before=snapshot()
scope={'metadata_argv':metadata_argv,'metadata_exit_code':mr.returncode,'metadata_sha256':hashlib.sha256(mr.stdout).hexdigest(),'packages':sorted(p['name'] for p in local),'count':len(before),'scope':'Standalone Runtime default-feature lib-test graph: root all dependency kinds; non-root normal/build edges. Local rs/c/h/tcl/txt/json and manifests/locks conservatively captured, plus exact test fixtures. Command Core and Registry INCLUDED; LSP Core, Compiler, IRules, MCP and xtask EXCLUDED. Not a whole-workspace or all-target freshness claim.','frozen_ready_sha256':ready}
for out in outs.values():
 (out/'scope.json').write_text(json.dumps(scope,indent=2)+'\n')
 (out/'runtime-dependency-source-before.json.gz').write_bytes(gzip.compress(json.dumps(before,indent=2).encode(),mtime=0))
 for rel in ready:(out/(rel.replace('/','--')+'.gz')).write_bytes(gzip.compress((root/rel).read_bytes(),mtime=0))
build_argv=[str(toolbin/'cargo'),'test','--offline','-j2','--manifest-path','runtime/rust/Cargo.toml','--lib','--no-run','--message-format=json']
parseout=outs['parse']
invocation={'argv':build_argv,'cwd':str(root),'environment_selectors':{k:env[k] for k in ('CARGO_HOME','RUSTUP_HOME','CARGO_TARGET_DIR')},'status':'running','started_utc':time.strftime('%Y-%m-%d %H:%M:%S UTC',time.gmtime())}
(parseout/'build-invocation.json').write_text(json.dumps(invocation,indent=2)+'\n')
print('BUILD START',len(local),'local packages',len(before),'captured files',flush=True)
with (parseout/'runtime-build-messages.jsonl').open('wb') as stdout,(parseout/'runtime-build.stderr.txt').open('wb') as stderr:
 run=subprocess.run(build_argv,cwd=root,env=env,stdout=stdout,stderr=stderr)
invocation.update(exit_code=run.returncode,status='completed',finished_utc=time.strftime('%Y-%m-%d %H:%M:%S UTC',time.gmtime()),messages_sha256=sha(parseout/'runtime-build-messages.jsonl'),stderr_sha256=sha(parseout/'runtime-build.stderr.txt'))
(parseout/'build-invocation.json').write_text(json.dumps(invocation,indent=2)+'\n')
for name in ['build-invocation.json','runtime-build-messages.jsonl','runtime-build.stderr.txt']:shutil.copyfile(parseout/name,outs['glob']/name)
after_build=snapshot();changed=[p for p in sorted(before.keys()|after_build.keys()) if before.get(p)!=after_build.get(p)]
for out in outs.values():(out/'source-comparison-after-build.json').write_text(json.dumps({'changed_dependency_sources':changed,'actual_ready_sha256':{p:sha(root/p) for p in ready}},indent=2)+'\n')
print('BUILD EXIT',run.returncode,'source changes',changed,flush=True)
names={'parse':['cmd_clock::get_date_fields_tests::parse_format_conflict_pool_lease_survives_all_three_real_worker_deletions','cmd_clock::get_date_fields_tests::parse_format_success_pool_lease_survives_all_three_real_worker_deletions','cmd_clock::get_date_fields_tests::active_call_pool_lease_survives_all_three_real_worker_deletions','cmd_clock::bootstrap_tests::parse_format_args_retains_supplied_objects_and_pool_members_after_retirement','cmd_clock::bootstrap_tests::deleting_getenv_first_keeps_parse_format_args_pool_until_its_retirement'],'glob':['cmd_fs::tests::glob_type_regular_file_directory_unions_match_original_c86_rows','cmd_fs::tests::glob_type_original_tcltest_selector_keeps_supported_kind_alternatives','cmd_fs::tests::glob_type_kind_union_still_requires_executable_qualification','cmd_fs::tests::glob_type_readable_writable_executable_qualifiers_remain_conjunctive']}
records={k:{'build':invocation,'checks':[],'unexecuted_names':v,'status':'Build failed; zero tests executed.' if run.returncode else 'Build complete; controls unexecuted.','comparison_scope':'Software Runtime control execution; original oracle fixtures reused without oracle reruns.'} for k,v in names.items()}
def save():
 for k,record in records.items():(outs[k]/'validation.json').write_text(json.dumps(record,indent=2)+'\n')
save()
if run.returncode or changed:sys.exit(run.returncode or 2)
items=[json.loads(line) for line in (parseout/'runtime-build-messages.jsonl').read_text().splitlines() if line.startswith('{')]
finished=[m for m in items if m.get('reason')=='build-finished'];assert len(finished)==1 and finished[0]['success']
artifacts=[m for m in items if m.get('reason')=='compiler-artifact' and m['target']['name']=='tcl_runtime' and m['profile']['test'] and m['executable']];assert len(artifacts)==1
binary=pathlib.Path(artifacts[0]['executable']);binary_sha=sha(binary)
assert binary.is_relative_to(root/'tmp/name-target-runtime')
listed=subprocess.run([str(binary),'--list'],cwd=root,capture_output=True)
for out in outs.values():
 (out/'runtime-tests-list.stdout.txt').write_bytes(listed.stdout);(out/'runtime-tests-list.stderr.txt').write_bytes(listed.stderr)
assert listed.returncode==0
for selected in names.values():
 for name in selected:assert listed.stdout.decode().splitlines().count(name+': test')==1,name
for rec in records.values():rec['build'].update(binary=str(binary),binary_sha256=binary_sha,cargo_test_artifact_fresh=artifacts[0].get('fresh'),test_list_exit_code=listed.returncode)
save()
failed=False
for key,selected in names.items():
 for index,name in enumerate(selected,1):
  argv=[str(binary),name,'--exact','--test-threads=1','--nocapture']
  run=subprocess.run(argv,cwd=root,capture_output=True)
  out=outs[key];log=out/f'{index:02d}-{name.rsplit("::",1)[1]}.log';log.write_bytes(run.stdout+run.stderr)
  records[key]['checks'].append({'name':name,'argv':argv,'exit_code':run.returncode,'log':log.name,'log_sha256':sha(log),'binary_sha256_before':binary_sha,'binary_sha256_after':sha(binary)})
  records[key]['unexecuted_names']=selected[index:]
  records[key]['status']='Focused control failed; remaining controls unexecuted.' if run.returncode else 'Selected controls in progress.'
  save();print(key,index,name,'EXIT',run.returncode,flush=True)
  if run.returncode:
   print(run.stdout.decode(errors='replace')+run.stderr.decode(errors='replace'),flush=True);failed=True;break
 if failed:break
 if not records[key]['unexecuted_names']:records[key]['status']='PASS: all selected exact controls each executed once from the same fresh Runtime binary.';save()
after=snapshot();changed=[p for p in sorted(before.keys()|after.keys()) if before.get(p)!=after.get(p)]
for out in outs.values():
 (out/'runtime-dependency-source-after.json.gz').write_bytes(gzip.compress(json.dumps(after,indent=2).encode(),mtime=0))
 (out/'source-comparison.json').write_text(json.dumps({'changed_dependency_sources':changed,'actual_ready_sha256':{p:sha(root/p) for p in ready},'binary_sha256_after':sha(binary)},indent=2)+'\n')
 shutil.copyfile('/tmp/name-joint-runtime-validation.py',out/'capture-validation.py')
print('FINAL source changes',changed,'binary unchanged',sha(binary)==binary_sha,flush=True)
sys.exit(1 if failed or changed or sha(binary)!=binary_sha else 0)
