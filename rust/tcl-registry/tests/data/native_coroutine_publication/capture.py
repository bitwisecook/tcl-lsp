import hashlib,json,os,pathlib,subprocess
packet=pathlib.Path('/workspace/.proofs/native-coroutine-publication094')
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
 receipt={'provider':provider['id'],'required_sha256':provider['required_sha256'],'probe_sha256':sha(queue['probe']),'inputs_sha256':sha(queue['inputs']),'runner_sha256':sha(__file__),'queue_sha256':sha(packet/'queue.json'),'compile_command':command,'environment':provider['environment'],'compile_exit':built.returncode,'compile_stdout_sha256':sha(d/'compile.stdout'),'compile_stderr_sha256':sha(d/'compile.stderr'),'process_exit':None,'rows':[]}
 if built.returncode==0:
  ran=subprocess.run([str(exe)],env=env,capture_output=True,timeout=60)
  (d/'stdout.tsv').write_bytes(ran.stdout);(d/'stderr').write_bytes(ran.stderr)
  rows=ran.stdout.decode('ascii').splitlines();versions=[r for r in rows if r.startswith('VERSION|0|')]
  receipt.update(executable_sha256=sha(exe),process_exit=ran.returncode,stdout_sha256=sha(d/'stdout.tsv'),stderr_sha256=sha(d/'stderr'),rows=rows,reported_version=bytes.fromhex(versions[0].split('|')[-1]).decode('ascii') if len(versions)==1 else None)
  failed|=ran.returncode!=0 or bool(ran.stderr)
 else:failed=True
 (d/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');captures.append(receipt)
 (output/'receipt.json').write_text(json.dumps(captures,indent=2)+'\n')
 print(provider['id'],'compile',built.returncode,'process',receipt['process_exit'],'rows',len(receipt['rows']),flush=True)
if failed:raise RuntimeError('capture contains an attempted harness failure; receipts retained')
