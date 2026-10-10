from pathlib import Path
import hashlib,json,os,subprocess,time
base=Path('/workspace/.proofs');folder=base/'native-jim-dictionary-bootstrap191';jim=base/'native-providers/jimtcl'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
request=json.loads((folder/'request.json').read_bytes()); source=folder/'probe.c'
assert sha(source)==request['source_sha256']
prior_path=base/'native-jim-class-initialisers271/jim/receipt.json';prior=json.loads(prior_path.read_bytes())
pins=dict(prior['required_sha256'])
for name in request['required_source_files']:
 p=jim/name;pins[str(p)]=sha(p)
for name,digest in pins.items():assert sha(Path(name))==digest,name
exe=folder/'probe.elf';assert not exe.exists()
env={**os.environ,**prior['environment']};compile_command=['cc','-std=c99','-I',str(jim),str(source),str(jim/'libjim.a'),'-lm','-lssl','-lcrypto','-lz','-o',str(exe)]
for label,command in [('compile',compile_command),('execute',[str(exe)])]:
 start=time.monotonic();r=subprocess.run(command,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=60)
 (folder/(label+'.stdout')).write_bytes(r.stdout);(folder/(label+'.stderr')).write_bytes(r.stderr)
 record={'command':command,'exit':r.returncode,'seconds':time.monotonic()-start,'environment':prior['environment'],'input_sha256':sha(source),'required_sha256':pins,'stdout_sha256':sha(folder/(label+'.stdout')),'stderr_sha256':sha(folder/(label+'.stderr')),'version_pin_receipt':str(prior_path),'version_pin_receipt_sha256':sha(prior_path),'reported_version_row_association':prior['reported_version_row'],'channel':request['channel']}
 if exe.exists():record['executable_sha256']=sha(exe)
 (folder/(label+'.receipt.json')).write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({k:v for k,v in record.items() if k not in ['required_sha256','environment']}));print(r.stdout.decode());print(r.stderr.decode());assert r.returncode==0
for name,digest in pins.items():assert sha(Path(name))==digest,name
assert sha(source)==request['source_sha256']
