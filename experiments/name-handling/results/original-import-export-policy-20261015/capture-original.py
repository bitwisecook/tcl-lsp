from pathlib import Path
import hashlib,json,subprocess,os,datetime
r=Path('/workspace/tcl-lsp');p=Path('/tmp/name-original-import-export-policy-20261015');p.mkdir(exist_ok=True)
probe='''namespace eval ::Source {proc p {} {return SOURCE}}
namespace eval ::Target {}
set c [catch {namespace eval ::Target {namespace import ::Source::*}} m]
puts [list NOEXPORT_IMPORT $c $m]
puts [list NOEXPORT_CALL [catch {::Target::p} m] $m]
namespace eval ::Self {proc p {} {return SELF}}
puts [list SELF_IMPORT [catch {namespace eval ::Self {namespace import ::Self::*}} m] $m]
puts [list SELF_CALL [catch {::Self::p} m] $m]
'''
(p/'probe.tcl').write_text(probe);h=lambda f:hashlib.sha256(f.read_bytes()).hexdigest();rows=[]
for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','Jim']:
 t=r/'tmp'/('jimtcl-oracle' if v=='Jim' else 'tcl'+v);exe=t/('jimsh' if v=='Jim' else 'unix/tclsh');env=os.environ.copy();env.pop('TCLLIBPATH',None);env.pop('TCL_LIBRARY',None);files=[exe]
 if v!='Jim':env['TCL_LIBRARY']=str(t/'library');files.append(t/'library/init.tcl')
 else:files.append(t/'nshelper.tcl')
 before={str(f):h(f) for f in files};x=subprocess.run([str(exe)],input=probe.encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env);(p/(v+'.stdout')).write_bytes(x.stdout);(p/(v+'.stderr')).write_bytes(x.stderr);after={str(f):h(f) for f in files};assert before==after;rows.append({'version':v,'argv':[str(exe)],'provider_before':before,'provider_after':after,'exit_code':x.returncode,'stdout_sha256':h(p/(v+'.stdout')),'stderr_sha256':h(p/(v+'.stderr'))});print(v,x.returncode,x.stdout.decode().strip())
(p/'manifest.json').write_text(json.dumps({'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'probe_sha256':h(p/'probe.tcl'),'scope':'Actual C5 and pinned Jim namespace import without export plus self import; no conflict/alias/census/lifecycle completion or Runtime execution claim. Pinned Jim timed upstream evidence remains independently captured at04:25.','rows':rows},indent=2)+'\n');print('manifest',h(p/'manifest.json'))
