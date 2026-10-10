from pathlib import Path
import json,hashlib,os,subprocess,time,shutil
b=Path('/workspace/.proofs');f=b/'native-callback-scope240';f.mkdir(exist_ok=False);source=f/'probe.tcl';shutil.copy2(b/'consumer-native-callback-scope240/source.tcl',source);version_source=f/'version.tcl';version_source.write_bytes(b'puts [list version [info patchlevel]]\n')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(source)=='4bf4df72bdc2aea99c1727d47f7e3f66986ba2664f380ba468d5de93b4491514'
for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim']:
 old=json.loads((b/'native-jim-class-source269'/v/'receipt.json').read_text());exe=Path(old['executable_path']);pins=dict(old['required_sha256']);pins[str(exe)]=old['executable_sha256'];env={**os.environ,**old['environment']};d=f/v;d.mkdir()
 if v!='jim':
  root=Path('/workspace/tcl-lsp/tmp')/('tcl'+v)
  for name in ['tclPkg.c','tclBasic.c','tclNamesp.c']:pins[str(root/'generic'/name)]=sha(root/'generic'/name)
 for name,digest in pins.items():assert sha(Path(name))==digest,name
 commands={};version='0.84-9-g5bac7c9' if v=='jim' else v
 for kind,p in [('version',version_source),('original',source)]:
  command=[str(exe),str(p)];start=time.monotonic();r=subprocess.run(command,env=env,capture_output=True,timeout=30);(d/(kind+'.stdout')).write_bytes(r.stdout);(d/(kind+'.stderr')).write_bytes(r.stderr)
  commands[kind]={'command':command,'exit':r.returncode,'seconds':time.monotonic()-start,'source_sha256':sha(p),'stdout_sha256':sha(d/(kind+'.stdout')),'stderr_sha256':sha(d/(kind+'.stderr')),'rows':r.stdout.decode().splitlines()}
  assert r.returncode==0 and not r.stderr,(v,kind,r.stderr)
 assert commands['version']['rows']==['version '+version]
 for name,digest in pins.items():assert sha(Path(name))==digest,name
 record={'provider':v,'reported_version':version,'executions':commands,'environment':old['environment'],'executable_path':str(exe),'executable_sha256':sha(exe),'required_sha256':pins,'channel':'Exact standalone ASCII LF CLI original script, explicit catch as written; version separately executed, no surrounding evaluation wrapper; observed callback markers are public results, no private-frame/TMM claim'}
 (d/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({'provider':v,'rows':commands['original']['rows']}),flush=True)
