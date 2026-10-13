import pathlib,json,subprocess,time,hashlib
p=pathlib.Path('/tmp/runtime-naming-final-capture-20261013');root=pathlib.Path('/workspace/name-implementation-worktrees/runtime');r=json.loads((p/'engine-guest-input-seal.json').read_text());sha=lambda q:hashlib.sha256(q.read_bytes()).hexdigest();names=r['plan'];b=r['artifact'];allnames=[l.removesuffix(': test') for l in subprocess.check_output([b,'--list'],text=True).splitlines() if l.endswith(': test')];assert set(names)<=set(allnames)
args=[b,'--nocapture','--test-threads=1']
for n in allnames:
 if n not in names:args+=['--skip',n]
(p/'engine-exact-batch-plan.json').write_text(json.dumps({'selected':names,'excluded':[n for n in allnames if n not in names],'args':args},indent=2)+'\n');log=p/'engine-batch-01.log';start=time.monotonic()
try:
 with log.open('w') as f:c=subprocess.run(args,cwd=root,stdout=f,stderr=subprocess.STDOUT,timeout=600)
 code=c.returncode;timeout=False
except subprocess.TimeoutExpired:code=None;timeout=True
text=log.read_text();rows=[{'selector':n,'passed':f'test {n} ... ok\n' in text,'failed':f'test {n} ... FAILED\n' in text} for n in names];r.update({'batch_exit':code,'batch_timeout':timeout,'elapsed_seconds':time.monotonic()-start,'exact_seven_tests':'running 7 tests\n' in text,'log':str(log),'log_sha256':sha(log),'runs':rows,'skip_output':'SKIPPING' in text});r['changed_host_inputs']={f:sha(pathlib.Path(f)) for f,h in r['compiled_inputs'].items() if sha(pathlib.Path(f))!=h};r['changed_guest_inputs']={f:sha(pathlib.Path(f)) for f,h in r['guest_compiled_inputs'].items() if sha(pathlib.Path(f))!=h};r['changed_artifacts']={f:sha(pathlib.Path(f)) for f,h in [(b,r['artifact_sha256']),(r['guest_artifact'],r['guest_artifact_sha256'])] if sha(pathlib.Path(f))!=h};(p/'engine-result.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps({'exit_code':code,'timeout':timeout,'elapsed':r['elapsed_seconds'],'exact7':r['exact_seven_tests'],'runs':rows}),flush=True)
