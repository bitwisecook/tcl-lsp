"""Run explicit selectors one per process against a finalized build, 180 s each."""
from pathlib import Path
import hashlib,json,os,subprocess,sys,time

W=Path('/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/w');R=Path('/home/user/tcl-lsp')
build,out,listing=W/sys.argv[1],W/sys.argv[2],Path(sys.argv[3])
artifacts=json.loads((build/'artifacts.json').read_text())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
for item in artifacts.values():assert sha(item['path'])==item['sha256']
env=os.environ.copy();env.update(TCL_LSP_REQUIRE_TCLSH='9.0,8.6',TCL_LSP_JIMSH=str(R/'tmp/jimtcl-oracle/jimsh'))
out.mkdir(exist_ok=False);rows=[]
for i,line in enumerate(listing.read_text().split('\n')):
    if not line.strip():continue
    target,selector=line.split()
    log=out/f'test-{i+1:03d}.log';t=time.monotonic()
    with log.open('wb') as f:
        try:p=subprocess.run([artifacts[target]['path'],selector,'--exact','--nocapture','--test-threads=1'],cwd=R,env=env,stdout=f,stderr=subprocess.STDOUT,timeout=180);code,to=p.returncode,False
        except subprocess.TimeoutExpired:code,to=None,True
    text=log.read_text(errors='replace')
    ok=code==0 and not to and 'running 1 test' in text and 'test result: ok. 1 passed; 0 failed' in text
    rows.append({'target':target,'selector':selector,'passed':ok,'timed_out':to,'seconds':round(time.monotonic()-t,2),'log':log.name,'log_sha256':sha(log)})
    print(('PASS' if ok else 'TIMEOUT' if to else 'FAIL'),rows[-1]['seconds'],target,selector,flush=True)
(out/'result.json').write_text(json.dumps({'build':str(build),'artifacts_sha256':sha(build/'artifacts.json'),'runs':rows},indent=2)+'\n')
print(sum(r['passed'] for r in rows),'/',len(rows))
