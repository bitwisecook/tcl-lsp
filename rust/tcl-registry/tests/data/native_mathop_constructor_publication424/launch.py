from pathlib import Path
import hashlib,json,os,subprocess,time
base=Path(__file__).parent
sha=lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest()
request=json.loads((base/'request.json').read_text())
assert sha(request['probe_file'])==request['probe_sha256']
processes=[]
for provider in request['provider_builds']:
 name=provider['provider'];directory=base/'providers'/name;directory.mkdir(parents=True,exist_ok=True)
 required=provider['required_sha256'];env={**os.environ,**provider['environment']}
 for path,digest in required.items():assert sha(path)==digest,path
 for channel,command in [('compile',provider['command']),('execute',[provider['command'][-1]])]:
  start=time.monotonic();timed_out=False
  try:
   process=subprocess.run(command,env=env,cwd=base,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=60)
   code=process.returncode;stdout=process.stdout;stderr=process.stderr
  except subprocess.TimeoutExpired as error:
   timed_out=True;code=None;stdout=error.stdout or b'';stderr=error.stderr or b''
  out=directory/(channel+'.stdout');err=directory/(channel+'.stderr');out.write_bytes(stdout);err.write_bytes(stderr)
  receipt={'question':request['question'],'provider':name,'channel':channel,'command':command,'cwd':str(base),'environment':provider['environment'],'exit':code,'timeout':timed_out,'seconds':time.monotonic()-start,'request_sha256':sha(base/'request.json'),'probe_source_sha256':sha(request['probe_file']),'required_sha256':required,'stdout_sha256':sha(out),'stderr_sha256':sha(err),'version_receipt':provider['version_receipt'],'version_receipt_sha256':provider['version_receipt_sha256'],'reported_version_row':provider['reported_version_row']}
  if channel=='execute':receipt['probe_executable_sha256']=sha(command[0]);receipt['compile_receipt_sha256']=sha(directory/'compile.receipt.json')
  path=directory/(channel+'.receipt.json');path.write_text(json.dumps(receipt,indent=2)+'\n');processes.append({'path':str(path),'sha256':sha(path)})
  for path,digest in required.items():assert sha(path)==digest,path
  if code!=0 or timed_out:break
(base/'closed-processes.json').write_text(json.dumps({'question':request['question'],'request_sha256':sha(base/'request.json'),'processes':processes},indent=2)+'\n')
