from pathlib import Path
import collections,datetime,hashlib,json,shutil,subprocess,tarfile
r=Path('/workspace/tcl-lsp');b=Path('/workspace/naming-validation/lsp-configured-consumer-source-successor-repair2');out=r/'experiments/name-handling/results/owner-adoption-20261017/lsp-configured-consumer-source-successor';d=out/'completed-extra18';d.mkdir(exist_ok=False)
sha=lambda p:hashlib.file_digest(Path(p).open('rb'),'sha256').hexdigest()
read=lambda p:json.loads(Path(p).read_text())
inv=read(b/'extra18-results-invocation.json');assert inv['preflight']==inv['postflight'] and inv['attempted_groups']==4 and 'postflight_error'not in inv
runs=[];previous=[];errors=[]
for target in ['tcl_lexer','tcl_syntax','tcl_spectcl','xtask']:
 j=read(b/'extra18-results'/target/'result.json');assert not j['artifact_drift']and j['selected_vector_complete'];runs+=j['runs'];errors+=j['errors'];previous+=read(b.parent/'lsp-studio-action-source-successor-repair3/extra18-results'/target/'result.json')['runs']
assert len(runs)==len(previous)==18 and not errors
assert [(x['target'],x['selector'])for x in runs]==[(x['target'],x['selector'])for x in previous]
status=lambda v:'TIMEOUT'if v['timed_out']else'PASS'if v['test_passed']else'FAIL'
changes=[{'target':n['target'],'selector':n['selector'],'before':status(o),'after':status(n)}for o,n in zip(previous,runs)if status(o)!=status(n)]
summary={'selected':18,'counts':dict(collections.Counter(map(status,runs))),'by_target':{t:dict(collections.Counter(status(x)for x in runs if x['target']==t))for t in ['tcl_lexer','tcl_syntax','tcl_spectcl','xtask']},'harness_errors':errors,'actual_wrapper_exit_code':inv['wrapper_exit_code'],'completed_utc':inv['completed_utc'],'all39058_inputs_nine_ELFs_providers_postflight_match':True,'retained_extra18_status_changes':changes,'prior_PASS_regressions':[x for x in changes if x['before']=='PASS'],'ordinary_acceptance':False,'limits':'One original SpecTcl Native command-name protocol refusal remains; all original assertions and guards preserved. Standard994 separately running; global/host gates pending.','launch_preparation_error':'Initial Root invocation supplied a mistyped binding SHA and was refused before capture; corrected exact independently peered SHA launched successfully. No selections or outputs from rejected invocation.'}
(d/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');shutil.copy2(b/'extra18-results-invocation.json',d/'invocation.json')
files={str(p.relative_to(b/'extra18-results')):p for p in sorted((b/'extra18-results').rglob('*'))if p.is_file()}
for root,prefix in [(b/'extra18-bound-v1','bound'),(Path('/tmp/lsp-configured-extra18-binder-template-v2'),'prepared-package'),(Path('/tmp/sol-configured-extra18-actual-bound-v1-independent'),'independent-bound-review'),(Path('/tmp/sol-configured-extra18-template-v2-independent'),'independent-template-review')]:
 for p in sorted(root.rglob('*')):
  if p.is_file()and '__pycache__'not in p.parts:files[prefix+'/'+str(p.relative_to(root))]=p
expected={k:{'sha256':sha(p),'bytes':p.stat().st_size}for k,p in files.items()};assert sum(k.startswith(('tcl_lexer/','tcl_syntax/','tcl_spectcl/','xtask/'))and Path(k).name.startswith('test-')and k.endswith('.log')for k in files)==18
archive=d/'raw-results-binding-and-package.tar.zst'
with archive.open('wb')as stream:
 proc=subprocess.Popen(['zstd','-q','-9','-c'],stdin=subprocess.PIPE,stdout=stream)
 with tarfile.open(fileobj=proc.stdin,mode='w|')as tar:
  for k,p in files.items():tar.add(p,arcname=k,recursive=False)
 proc.stdin.close();assert proc.wait()==0
proc=subprocess.Popen(['zstd','-q','-dc',str(archive)],stdout=subprocess.PIPE);seen=set()
with tarfile.open(fileobj=proc.stdout,mode='r|')as tar:
 for m in tar:
  assert m.isfile()and m.name in expected and m.name not in seen and m.size==expected[m.name]['bytes'];assert hashlib.file_digest(tar.extractfile(m),'sha256').hexdigest()==expected[m.name]['sha256'];seen.add(m.name)
assert proc.wait()==0 and seen==set(expected)
(d/'raw-archive.json').write_text(json.dumps({'sha256':sha(archive),'bytes':archive.stat().st_size,'members':expected},indent=2)+'\n')
x=out/'extra-xtask-build';x.mkdir(exist_ok=False)
for name in ['source-binding.json','build.log','artifacts.json']:shutil.copy2(b/'extra-xtask'/name,x/name);assert sha(b/'extra-xtask'/name)==sha(x/name)
shutil.copy2('/tmp/naming-build-configured-consumer-extra-v1.py',x/'build.py');shutil.copy2(__file__,d/'preserve.py');print(json.dumps(summary,indent=2));print('archive',len(expected),archive.stat().st_size,sha(archive))
