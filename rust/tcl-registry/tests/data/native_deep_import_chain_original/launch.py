from pathlib import Path
import hashlib,json,os,subprocess,time
P=Path(__file__).resolve().parent
request_bytes=(P/'request.json').read_bytes();request=json.loads(request_bytes)
def sha(b):return hashlib.sha256(b).hexdigest()
def verify():
 for path,expected in request['required_sha256'].items():
  if sha(Path(path).read_bytes())!=expected:raise RuntimeError('Required input mismatch: '+path)
verify();receipts=[]
for provider in request['providers']:
 directory=P/'providers'/provider['provider'];directory.mkdir(parents=True,exist_ok=True)
 stdout=directory/'execute.stdout';stderr=directory/'execute.stderr'
 environment=dict(os.environ);environment.update(provider['environment'])
 start=time.time();timed_out=False
 with stdout.open('wb')as out,stderr.open('wb')as err:
  try:
   process=subprocess.run(provider['command'],env=environment,stdout=out,stderr=err,timeout=45)
   returncode=process.returncode
  except subprocess.TimeoutExpired:
   timed_out=True;returncode=None
 raw=stdout.read_bytes();case_count=sum(line.startswith(b'CASE ') for line in raw.splitlines())
 receipt={'provider':provider['provider'],'request_sha256':sha(request_bytes),'command':provider['command'],'selected_environment':provider['environment'],'source_sha256':request['probe_sha256'],'executable':provider['executable'],'executable_sha256':request['required_sha256'][provider['executable']],'version_channel':'CASE version.patchlevel, original public info patchlevel; independent executable path query','returncode':returncode,'signal':-returncode if returncode is not None and returncode<0 else None,'timeout':timed_out,'start_unix_seconds':start,'end_unix_seconds':time.time(),'stdout_file':str(stdout),'stdout_sha256':sha(raw),'stderr_file':str(stderr),'stderr_sha256':sha(stderr.read_bytes()),'observed_case_count':case_count,'required_case_count':95,'closed_marker':b'CLOSED_DEEP_IMPORT_CHAIN_200\n' in raw,'status':'closed'}
 (directory/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');receipts.append(receipt)
verify()
closed=len(receipts)==6 and all(r['returncode']==0 and not r['timeout'] and r['closed_marker'] and r['observed_case_count']==r['required_case_count'] for r in receipts)
(P/'closure.json').write_text(json.dumps({'request_sha256':sha(request_bytes),'records':receipts,'all_processes_closed':len(receipts)==6,'all_succeeded':closed,'required_inputs_unchanged':True,'scope':'Process/source capture closure, not a claim of guest case success or native backend equivalence'},indent=2)+'\n')
print('CLOSED original processes:',len(receipts),'capture succeeded:',closed)
