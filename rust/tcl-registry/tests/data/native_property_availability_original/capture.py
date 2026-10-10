from pathlib import Path
import subprocess,json,hashlib,os,time
from concurrent.futures import ThreadPoolExecutor
base=Path(__file__).parent;request=json.loads((base/'queue.json').read_text());providers=json.loads(Path('/workspace/.proofs/native-info-inventory193/queue.json').read_text())['providers']
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
assert sha(base/'probe.c')==request['source_sha256']
for p in providers:
 for path,digest in p['required_sha256'].items():assert sha(path)==digest,path
out=base/'output';out.mkdir();(out/'sdk-bindings.json').write_text(json.dumps(providers,indent=2)+'\n')
def run(p):
 d=out/p['id'];d.mkdir();exe=d/'probe';cmd=['cc','-std=c99',*p['flags'],str(base/'probe.c'),p['library'],*p['link_flags'],'-o',str(exe)];start=time.monotonic();build=subprocess.run(cmd,capture_output=True,timeout=30)
 (d/'compile.stdout').write_bytes(build.stdout);(d/'compile.stderr').write_bytes(build.stderr)
 r={'question':request['question'],'provider':p['id'],'input_channel':request['input_channel'],'scope':request['scope'],'source_sha256':sha(base/'probe.c'),'queue_sha256':sha(base/'queue.json'),'runner_sha256':sha(__file__),'required_sha256':p['required_sha256'],'environment':p['environment'],'compile_command':cmd,'compile_exit':build.returncode,'compile_stdout_sha256':sha(d/'compile.stdout'),'compile_stderr_sha256':sha(d/'compile.stderr'),'process_exit':None}
 if not build.returncode:
  process=subprocess.run([str(exe)],env={**os.environ,**p['environment']},capture_output=True,timeout=30);(d/'stdout.tsv').write_bytes(process.stdout);(d/'stderr').write_bytes(process.stderr);rows=process.stdout.decode('ascii').splitlines();version=[line for line in rows if line.startswith('VERSION|0|')];assert len(version)==1;v=bytes.fromhex(version[0].split('|')[-1]).decode('ascii');expected='0.84-9-g5bac7c9' if p['id']=='jim' else p['id'];assert v==expected
  r.update(executable_sha256=sha(exe),process_exit=process.returncode,stdout_sha256=sha(d/'stdout.tsv'),stderr_sha256=sha(d/'stderr'),reported_version=v,rows=rows)
 r['seconds']=time.monotonic()-start;(d/'receipt.json').write_text(json.dumps(r,indent=2)+'\n');print(p['id'],'compile',r['compile_exit'],'process',r['process_exit'],'rows',r.get('rows'),flush=True);assert not build.returncode and not r['process_exit'] and not process.stderr
 return r
with ThreadPoolExecutor(max_workers=3) as pool:receipts=list(pool.map(run,providers))
(out/'receipt.json').write_text(json.dumps(receipts,indent=2)+'\n')
