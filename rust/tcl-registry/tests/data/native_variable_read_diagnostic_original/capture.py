from pathlib import Path
import subprocess,hashlib,json,os,time
out=Path(__file__).parent
providers=json.loads(Path('/workspace/.proofs/native-info-inventory193/queue.json').read_text())['providers']
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
for provider in providers:
 root=Path(provider['library']).parent.parent
 exe=Path('/workspace/.proofs/native-providers/jimtcl/jimsh') if provider['id']=='jim' else root/'unix/tclsh'
 pins=provider['required_sha256']
 for path,digest in pins.items():assert sha(path)==digest,path
 dest=out/provider['id'];dest.mkdir()
 command=[str(exe),str(out/'probe.tcl')];start=time.monotonic()
 r=subprocess.run(command,env={**os.environ,**provider['environment']},capture_output=True,timeout=15)
 (dest/'stdout').write_bytes(r.stdout);(dest/'stderr').write_bytes(r.stderr)
 rows=r.stdout.decode('ascii').splitlines();version=rows[0].split(' ',1)[1]
 assert version.startswith('0.84-9-g5bac7c9') if provider['id']=='jim' else version==provider['id'],version
 receipt={'command':command,'exit':r.returncode,'seconds':time.monotonic()-start,'reported_version':version,'executable_path':str(exe),'executable_sha256':sha(exe),'source_sha256':sha(out/'probe.tcl'),'runner_sha256':sha(__file__),'stdout_sha256':sha(dest/'stdout'),'stderr_sha256':sha(dest/'stderr'),'environment':provider['environment'],'required_sha256':pins,'rows':rows,'input_channel':'ASCII LF source-file CLI; six scalar/element variable read cases','boundary':'Six labelled variable read cases across literal, dynamic and procedure source entries. Full native result and error outcomes are retained independently by provider. No physical object/header/cache or compiler admission proof is inferred from CLI results.'}
 (dest/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
 print(provider['id'],r.returncode,rows,flush=True)
 assert r.returncode==0 and not r.stderr
