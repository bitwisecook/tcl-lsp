from pathlib import Path
import datetime,hashlib,json,os,subprocess,time
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-propagation-original-word-oracles-20261014')
rows=[('tcl8.4','tcl8.4.20'),('tcl8.5','tcl8.5.19'),('tcl8.6','tcl8.6.18'),('tcl9.0','tcl9.0.4'),('tcl9.1','tcl9.1.0'),('jim','jimtcl-oracle')]
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
 return h.hexdigest()
results=[];source=(out/'probe.tcl').read_bytes()
for profile,folder in rows:
 provider=root/'tmp'/folder;exe=provider/('jimsh' if profile=='jim' else 'unix/tclsh');exe=exe.resolve();env=os.environ.copy()
 for name in ['TCL_LIBRARY','TCLLIBPATH','JIMLIB']:
  env.pop(name,None)
 env['TCLLIBPATH']=''
 if profile!='jim':env['TCL_LIBRARY']=str(provider/'library')
 # Same existing matching shared library tree; no provider/library files modified.
 env['LD_LIBRARY_PATH']=str(exe.parent)+(':'+env['LD_LIBRARY_PATH'] if env.get('LD_LIBRARY_PATH') else '')
 init=provider/'library/init.tcl' if profile!='jim' else None
 before=sha(exe);init_before=sha(init) if init else None;clock=time.monotonic()
 p=subprocess.run([str(exe),str(out/'probe.tcl')],cwd=out,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=10)
 stdout=out/(profile+'.stdout');stderr=out/(profile+'.stderr');stdout.write_bytes(p.stdout);stderr.write_bytes(p.stderr)
 results.append({'profile':profile,'command':[str(exe),str(out/'probe.tcl')],'executable_sha256_before':before,'executable_sha256_after':sha(exe),'matching_init':str(init) if init else None,'matching_init_sha256_before':init_before,'matching_init_sha256_after':sha(init) if init else None,'TCL_LIBRARY':env.get('TCL_LIBRARY'),'TCLLIBPATH':env['TCLLIBPATH'],'elapsed_seconds':time.monotonic()-clock,'exit_code':p.returncode,'stdout':str(stdout),'stdout_sha256':sha(stdout),'stderr':str(stderr),'stderr_sha256':sha(stderr),'init_query':'Original info library command result is retained verbatim; Jim may not support this C query.'})
 assert before==sha(exe) and (not init or init_before==sha(init))
assert source==(out/'probe.tcl').read_bytes()
receipt={'started_scope':'Root granted lightweight six-provider read-only scratch probe lease during current semantic capture; no Cargo, provider or Main mutation. No tcltest used.','finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_sha256':sha(out/'probe.tcl'),'source':str(out/'probe.tcl'),'results':results,'all_exit_zero':all(r['exit_code']==0 for r in results),'claim':'Six-provider original propagation original-word observations, no Rust execution or Native admission acceptance; F5 pending.'}
(out/'results.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps({k:receipt[k] for k in ['all_exit_zero']}))
