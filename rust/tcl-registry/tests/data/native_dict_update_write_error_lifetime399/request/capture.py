from pathlib import Path
import hashlib,json,subprocess,time,concurrent.futures
r=Path(__file__).resolve().parent;request=json.loads((r/'request.json').read_bytes());out=Path(request['output_directory']);out.mkdir(exist_ok=False);sha=lambda b:hashlib.sha256(b).hexdigest();launcher=sha(Path(__file__).read_bytes());request_sha=sha((r/'request.json').read_bytes())
def verify():
 for p,s in request['required_sha256'].items():
  if sha(Path(p).read_bytes())!=s:raise RuntimeError('changed input '+p)
def provider(item):
 version=item['version'];sdk=Path(item['sdk']);directory=out/version;directory.mkdir()
 def run(label,command):
  verify();start=time.monotonic();process=subprocess.run(command,capture_output=True,timeout=60);(directory/(label+'.stdout')).write_bytes(process.stdout);(directory/(label+'.stderr')).write_bytes(process.stderr)
  row={'label':label,'version':version,'command':command,'exit':process.returncode,'seconds':time.monotonic()-start,'stdout_sha256':sha(process.stdout),'stderr_sha256':sha(process.stderr),'request_sha256':request_sha,'launcher_sha256':launcher,'required_sha256':request['required_sha256'],'scope':request['scope'],'observed_binary_sha256':sha((directory/'observed-probe').read_bytes()) if (directory/'observed-probe').exists() else None};(directory/(label+'.receipt.json')).write_text(json.dumps(row,indent=2)+'\n');print(version,label,'exit',process.returncode,'stderr',len(process.stderr),flush=True);verify();return process
 compile=run('compile',['cc','-I'+str(sdk/'unix'),'-I'+str(sdk/'generic'),str(r/'observed-probe.c'),item['observed_archive'],'-lm','-ldl','-lpthread','-lz','-o',str(directory/'observed-probe')]);assert compile.returncode==0
 for case in request['cases']:run('case'+str(case),[str(directory/'observed-probe'),str(case)])
verify()
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:list(pool.map(provider,request['versions']))
verify();(out/'summary.json').write_text(json.dumps({'id':request['id'],'request_sha256':request_sha,'launcher_sha256':launcher,'scope':request['scope']},indent=2)+'\n')
