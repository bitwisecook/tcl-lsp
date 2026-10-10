import hashlib,json,os,pathlib,subprocess
packet=pathlib.Path('/workspace/.proofs/native-event-original110')
queue=json.loads((packet/'queue.json').read_text())
def sha(p):return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
for name,digest in queue['input_sha256'].items():
 if sha(name)!=digest:raise RuntimeError('input drift: '+name)
for provider in queue['providers']:
 for name,digest in provider['required_sha256'].items():
  if sha(name)!=digest:raise RuntimeError('provider drift: '+name)
output=pathlib.Path(queue['output']);output.mkdir()
captures=[];failed=False
for provider in queue['providers']:
 d=output/provider['id'];d.mkdir();exe=d/'probe';env=os.environ.copy();env.update(provider['environment'])
 command=['cc','-std=c99',*provider['flags'],queue['probe'],provider['library'],*provider['link_flags'],'-o',str(exe)]
 built=subprocess.run(command,capture_output=True,timeout=60)
 (d/'compile.stdout').write_bytes(built.stdout);(d/'compile.stderr').write_bytes(built.stderr)
 receipt={'provider':provider['id'],'required_sha256':provider['required_sha256'],'probe_sha256':sha(queue['probe']),'inputs_sha256':sha(queue['inputs']),'runner_sha256':sha(__file__),'queue_sha256':sha(packet/'queue.json'),'compile_command':command,'environment':provider['environment'],'compile_exit':built.returncode,'compile_stdout_sha256':sha(d/'compile.stdout'),'compile_stderr_sha256':sha(d/'compile.stderr'),'cases':[]}
 if built.returncode==0:
  receipt['executable_sha256']=sha(exe)
  for case in queue['cases']:
   c=d/case['id'];c.mkdir();cmd=[str(exe),case['argument']]
   try:
    ran=subprocess.run(cmd,env=env,capture_output=True,timeout=case['timeout_seconds']);stdout=ran.stdout;stderr=ran.stderr;status='completed';code=ran.returncode
   except subprocess.TimeoutExpired as error:
    stdout=error.stdout or b'';stderr=error.stderr or b'';status='external-timeout';code=None
   (c/'stdout.tsv').write_bytes(stdout);(c/'stderr').write_bytes(stderr)
   rows=stdout.decode('ascii').splitlines();versions=[r for r in rows if r.startswith('VERSION|0|')]
   observed={'id':case['id'],'command':cmd,'timeout_seconds':case['timeout_seconds'],'process_status':status,'process_exit':code,'stdout_sha256':sha(c/'stdout.tsv'),'stderr_sha256':sha(c/'stderr'),'rows':rows,'reported_version':bytes.fromhex(versions[0].split('|')[2]).decode('ascii') if len(versions)==1 else None}
   receipt['cases'].append(observed)
   (c/'receipt.json').write_text(json.dumps(observed,indent=2)+'\n')
   failed|=status=='completed' and (code!=0 or bool(stderr))
 else:failed=True
 (d/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');captures.append(receipt)
 (output/'receipt.json').write_text(json.dumps(captures,indent=2)+'\n')
 print(provider['id'],'compile',built.returncode,'completed',sum(c['process_status']=='completed' for c in receipt['cases']),'timeouts',sum(c['process_status']=='external-timeout' for c in receipt['cases']),flush=True)
if failed:raise RuntimeError('capture contains an attempted harness failure; receipts retained')
