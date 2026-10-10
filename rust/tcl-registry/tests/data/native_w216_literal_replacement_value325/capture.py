from pathlib import Path
import hashlib,json,os,subprocess,time,concurrent.futures
base=Path('/workspace/.proofs');folder=base/'native-w216-literal-replacement-value325'
request=json.loads((folder/'request.json').read_text());source=folder/'probe.tcl'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(source)==request['source_sha256']
def capture(version):
 old=json.loads((base/'native-jim-class-source269'/version/'receipt.json').read_text())
 exe=Path(old['executable_path']);pins=old['required_sha256']
 assert sha(exe)==old['executable_sha256']
 for p,h in pins.items():assert sha(Path(p))==h,p
 output=folder/version;output.mkdir();command=[str(exe),str(source)];start=time.monotonic()
 result=subprocess.run(command,env={**os.environ,**old['environment']},stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=30)
 (output/'stdout').write_bytes(result.stdout);(output/'stderr').write_bytes(result.stderr)
 for p,h in pins.items():assert sha(Path(p))==h,p
 assert sha(source)==request['source_sha256'] and sha(exe)==old['executable_sha256']
 row={'command':command,'exit':result.returncode,'seconds':time.monotonic()-start,'provider':version,'provider_version_association':old['reported_version_row'],'version_association_receipt':str(base/'native-jim-class-source269'/version/'receipt.json'),'executable_path':str(exe),'executable_sha256':sha(exe),'source_sha256':sha(source),'stdout_sha256':sha(output/'stdout'),'stderr_sha256':sha(output/'stderr'),'environment':old['environment'],'required_sha256':pins,'rows':result.stdout.decode().splitlines(),'input_channel':request['channel'],'question':request['question']}
 (output/'receipt.json').write_text(json.dumps(row,indent=2)+'\n')
 return dict(provider=version,exit=result.returncode,rows=row['rows'],stderr=result.stderr.decode())
with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
 for result in pool.map(capture,request['required_providers']):print(json.dumps(result))
