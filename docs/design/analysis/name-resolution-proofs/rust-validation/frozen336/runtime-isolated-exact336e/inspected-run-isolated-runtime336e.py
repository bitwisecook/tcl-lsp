from pathlib import Path
import subprocess, os, json, hashlib, time, resource
base=Path('/workspace/.proofs');snapshot=Path('/workspace/.checkpoints/naming336');source=json.loads((snapshot/'snapshot.json').read_text());pin=json.loads((base/'pinned-runtime336e.json').read_text());selection=json.loads((base/'runtime-admitted-union336e.json').read_text());out=base/'runtime-isolated-exact336e';out.mkdir(exist_ok=False)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(Path(pin['executable']))==pin['sha256'];assert not selection['zero_matches'];assert selection['strict_image_sha256']==pin['sha256'];env={**os.environ,**json.loads((base/'native-test-env.json').read_text())};records=[];started=time.monotonic()
resource.setrlimit(resource.RLIMIT_CORE,(0,0))
for index,name in enumerate(selection['selected']):
 command=[pin['executable'],name,'--exact','--nocapture','--test-threads=1'];log=out/(str(index).zfill(3)+'.log');start=time.monotonic()
 with log.open('wb') as sink:r=subprocess.run(command,cwd=source['compile_source_root'],env=env,stdout=sink,stderr=subprocess.STDOUT)
 text=log.read_text();status='passed' if r.returncode==0 and '1 passed; 0 failed;' in text else 'failed' if r.returncode==101 and '0 passed; 1 failed;' in text else 'aborted-or-incomplete'
 records.append({'selector':name,'command':command,'exit':r.returncode,'seconds':time.monotonic()-start,'status':status,'log':str(log),'log_sha256':sha(log)})
 (out/'progress.json').write_text(json.dumps({'completed':len(records),'selected':len(selection['selected']),'records':records},indent=2)+'\n')
 if len(records)%25==0:print('Closed isolated cases',len(records),flush=True)
uniform=all(sha(snapshot/name)==h and sha(Path(source['compile_source_root'])/name)==h for name,h in source['source'].items());result={'command_mode':'One original admitted exact selector per process; no assertion or corpus changes. Core dump writing disabled, test execution otherwise unchanged.','source':source,'uniform_source':uniform,'pin':pin,'selection':selection,'seconds':time.monotonic()-started,'counts':{status:sum(r['status']==status for r in records) for status in ['passed','failed','aborted-or-incomplete']},'records':records};(out/'receipt.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k in ('seconds','counts','uniform_source')}),flush=True)
raise SystemExit(0 if uniform and all(r['status']=='passed' for r in records) else 1)
