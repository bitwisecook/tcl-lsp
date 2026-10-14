import datetime, hashlib, json, os, subprocess
from pathlib import Path
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-compiler19-corrections-20261014/oracles')
source='''namespace eval ::inner {proc p {} {return GLOBAL}}
namespace eval ::ns::inner {proc p {} {return LOCAL}}
proc ::ns::doit {} {rename inner::p inner::saved}
::ns::doit
puts [list [::inner::p] [::ns::inner::saved] [llength [info commands ::ns::inner::p]]]
'''
providers=[(f'tcl{v}',root/f'tmp/tcl{v}/unix/tclsh',root/f'tmp/tcl{v}/library') for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']]+[('jim',root/'tmp/jimtcl-oracle/jimsh',root/'tmp/jimtcl-oracle')]
records=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for name,binary,library in providers:
 case=out/name;case.mkdir(parents=True,exist_ok=True);script=case/'source.tcl';script.write_text(source)
 env=os.environ.copy()
 if name!='jim':env['TCL_LIBRARY']=str(library);env['LD_LIBRARY_PATH']=str(binary.parent)
 before=sha(binary);started=datetime.datetime.now(datetime.timezone.utc).isoformat()
 result=subprocess.run([str(binary),str(script)],capture_output=True,env=env,timeout=10)
 (case/'stdout').write_bytes(result.stdout);(case/'stderr').write_bytes(result.stderr)
 receipt={'provider':name,'binary':str(binary),'binary_sha256':before,'binary_unchanged':before==sha(binary),'source_sha256':sha(script),'started_utc':started,'exit_code':result.returncode,'command':[str(binary),str(script)],'library':str(library),'uses_tcltest':False,'prior_complete_provider_inventory':f'/tmp/name-issues-refresh-20261014/oracles-02/{name}/provider-identity/receipt.json','stdout':result.stdout.decode(),'stderr':result.stderr.decode()}
 (case/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');records.append(receipt)
 print(name,result.returncode,result.stdout.decode().strip(),result.stderr.decode().strip())
(out/'results.json').write_text(json.dumps(records,indent=2)+'\n')
