from pathlib import Path
import datetime,hashlib,json,os,subprocess
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-upvar-logical-frame-20261014/oracles');source='proc helper {source} {upvar 1 $source local; set local FORMAL}\nproc caller {} {helper destination; puts $destination}\ncaller\nproc literal {} {upvar 1 {$scalar(open} {$alias}; set {$alias} LITERAL}\nproc literalcaller {} {literal; puts [set {$scalar(open}]}\nliteralcaller\n' ;records=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
providers=[(f'tcl{v}',root/f'tmp/tcl{v}/unix/tclsh',root/f'tmp/tcl{v}/library') for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']]+[('jim',root/'tmp/jimtcl-oracle/jimsh',root/'tmp/jimtcl-oracle')]
for name,binary,library in providers:
 case=out/name;case.mkdir(parents=True,exist_ok=True);script=case/'source.tcl';script.write_text(source);env=os.environ.copy()
 if name!='jim':env['TCL_LIBRARY']=str(library);env['LD_LIBRARY_PATH']=str(binary.parent)
 before=sha(binary);started=datetime.datetime.now(datetime.timezone.utc).isoformat();result=subprocess.run([str(binary),str(script)],capture_output=True,env=env,timeout=10)
 (case/'stdout').write_bytes(result.stdout);(case/'stderr').write_bytes(result.stderr);prior=Path(f'/tmp/name-issues-refresh-20261014/oracles-02/{name}/provider-identity/receipt.json')
 record={'provider':name,'binary':str(binary),'binary_sha256':before,'binary_unchanged':before==sha(binary),'library':str(library),'source_sha256':sha(script),'started_utc':started,'exit_code':result.returncode,'uses_tcltest':False,'prior_complete_provider_inventory':str(prior),'prior_complete_provider_inventory_sha256':sha(prior),'stdout':result.stdout.decode(),'stderr':result.stderr.decode()};(case/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');records.append(record);print(name,result.returncode,repr(result.stdout.decode()),repr(result.stderr.decode()))
(out/'results.json').write_text(json.dumps(records,indent=2)+'\n');files={str(p.relative_to(out)):sha(p) for p in sorted(out.rglob('*')) if p.is_file()};(out/'manifest.json').write_text(json.dumps({'source_domain':'Original caller aliases use actual formal values and preserve literal dollar/open-parenthesis names','files':files},indent=2)+'\n');print('manifest',sha(out/'manifest.json'))
