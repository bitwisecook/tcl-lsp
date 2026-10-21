from pathlib import Path
import hashlib,json,shutil,subprocess,sys
N=sys.argv[1];extra=sys.argv[2:]

from _paths import R,W;L=R/'tmp/logs';B=W/('lsp-build-'+N)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
packages=['tcl-lsp-core','tcl-lsp-db','tcl-lsp-server','tcl-registry','tcl-spectcl','tcl-spec-studio']+extra
B.mkdir(exist_ok=False)
shutil.copy(L/('build'+N+'.jsonl'),B/'build.jsonl');shutil.copy(L/('build'+N+'.stderr'),B/'build.stderr')
shutil.copy(W/('pre-build'+N+'-inputs.json'),B/'build-inputs.json');shutil.copy(W/('pre-build'+N+'-providers.json'),B/'build-providers.json')
code=int((L/('build'+N+'.exit')).read_text().split()[-1])
for script,name in [('capture-inputs.py','build-post-inputs.json'),('capture-providers.py','build-post-providers.json')]:subprocess.run([sys.executable,str(Path(__file__).resolve().parent/script),str(B/name)],check=True)
record={'argv':['cargo','test','--locked','--no-run','--message-format=json','--lib','--bins']+[x for p in packages for x in ('-p',p)],'environment':{'CARGO_INCREMENTAL':'0','CARGO_PROFILE_TEST_DEBUG':'0','TCL_LSP_REQUIRE_TCLSH':'9.0,8.6','TCL_LSP_JIMSH':str(R/'tmp/jimtcl-oracle/jimsh'),'CARGO_BUILD_JOBS':'default (4)'},'exit':code,'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=R,text=True).strip(),'stdout_sha256':sha(B/'build.jsonl'),'stderr_sha256':sha(B/'build.stderr'),'source_root':str(R),'tests_executed':False,'working_tree_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','HEAD'],cwd=R)).hexdigest(),'guard_note':'Pre-build source/provider snapshot taken after cargo started with no source edits during the build; post-build snapshot compared byte-exact. git_head plus working_tree_diff_sha256 identify the composed source.'}
(B/'build-result.json').write_text(json.dumps(record,indent=2)+'\n')
assert (B/'build-inputs.json').read_bytes()==(B/'build-post-inputs.json').read_bytes(),'source drift during build'
assert (B/'build-providers.json').read_bytes()==(B/'build-post-providers.json').read_bytes(),'provider drift during build'
assert code==0,'build failed'
artifacts={};extra=[];finished=[]
for line in (B/'build.jsonl').open():
 j=json.loads(line)
 if j.get('reason')=='build-finished':finished.append(j['success'])
 if j.get('reason')!='compiler-artifact' or not j.get('executable'):continue
 t=j['target'];name=t['name']
 if 'lib' not in t['kind']:extra.append(j);continue
 assert name in [x.replace('-','_') for x in packages] and name not in artifacts
 assert j['profile']['test'] is True and j['profile']['debuginfo']==0 and j['profile']['debug_assertions'] is True
 p=Path(j['executable']);assert p.resolve().is_relative_to((R/'target').resolve()) and p.read_bytes()[:4]==b'\x7fELF'
 artifacts[name]={'path':str(p),'sha256':sha(p),'target':t,'features':j['features'],'profile':j['profile'],'fresh':j['fresh']}
assert finished==[True] and len(artifacts)==len(packages),(finished,list(artifacts))
(B/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n');(B/'additional-test-artifact-rows.json').write_text(json.dumps(extra,indent=2)+'\n')
print(json.dumps({'status':'DIRECT_LSP_BUILD'+N+'_SOURCE_PROVIDER_GUARDS_PASS','artifacts':len(artifacts),'artifacts_sha256':sha(B/'artifacts.json')}))
