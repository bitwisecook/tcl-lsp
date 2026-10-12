from pathlib import Path
import hashlib,json,os,subprocess,time
P=Path(__file__).resolve().parent
request_bytes=(P/'request.json').read_bytes();request=json.loads(request_bytes)
hash_bytes=lambda b:hashlib.sha256(b).hexdigest()
for path,expected in request['required_sha256'].items():
 actual=hash_bytes(Path(path).read_bytes())
 if actual!=expected:raise RuntimeError('Required input mismatch: '+path)
receipts=[]
for provider in request['providers']:
 directory=P/'providers'/provider['provider'];directory.mkdir(parents=True,exist_ok=True)
 stdout=directory/'execute.stdout';stderr=directory/'execute.stderr'
 environment=dict(os.environ);environment.update(provider['environment'])
 start=time.time();status='closed';timeout=False
 with stdout.open('wb')as out,stderr.open('wb')as err:
  try:
   process=subprocess.run(provider['command'],env=environment,stdout=out,stderr=err,timeout=30)
   returncode=process.returncode
  except subprocess.TimeoutExpired:
   timeout=True;returncode=None
 receipt={'provider':provider['provider'],'request_sha256':hash_bytes(request_bytes),'command':provider['command'],'selected_environment':provider['environment'],'source_sha256':request['probe_sha256'],'executable':provider['executable'],'executable_sha256':request['required_sha256'][provider['executable']],'version_receipt':provider['version_receipt'],'version_receipt_sha256':provider['version_receipt_sha256'],'returncode':returncode,'signal':-returncode if returncode is not None and returncode<0 else None,'timeout':timeout,'start_unix_seconds':start,'end_unix_seconds':time.time(),'stdout_file':str(stdout),'stdout_sha256':hash_bytes(stdout.read_bytes()),'stderr_file':str(stderr),'stderr_sha256':hash_bytes(stderr.read_bytes()),'closed_marker':b'CLOSED_CASES_3\n' in stdout.read_bytes(),'status':status}
 (directory/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');receipts.append(receipt)
(P/'closure.json').write_text(json.dumps({'request_sha256':hash_bytes(request_bytes),'records':receipts,'all_processes_closed':len(receipts)==6,'all_succeeded':all(r['returncode']==0 and not r['timeout'] and r['closed_marker'] for r in receipts)},indent=2)+'\n')
print('CLOSED original processes:',len(receipts),'all succeeded:',all(r['returncode']==0 and not r['timeout'] and r['closed_marker'] for r in receipts))
