from pathlib import Path
import subprocess,hashlib,json,os,shutil,datetime
out=Path('/tmp/name-original-cell-name-data-oracles-corrected-20261014');out.mkdir(exist_ok=True)
probe=r'''set {} EMPTY
puts [list empty [set {}]]
set {$name} SIGIL
puts [list braced-sigil [set {$name}]]
set {[name]} BRACKET
puts [list braced-command [set {[name]}]]
set a\x62 ESCAPED
puts [list escaped [set ab]]
set {a($key)} ELEMENT
puts [list literal-index [set {a($key)}]]
unset {}
set {($key)} ROOTLESS
puts [list empty-array-root [set {($key)}]]
foreach {{for$name}} {LOOP} {}
puts [list foreach-name [set {for$name}]]
catch {error BOOM} {$caught}
puts [list catch-name [set {$caught}]]
puts [list expr-name [expr {${$name} eq "SIGIL"}]]
unset -nocomplain missing
puts [list dynamic-name-missing [catch {set $missing 1}]]
'''
(out/'probe.tcl').write_text(probe)
providers=[]
for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']:
 providers.append((v,Path('/workspace/tcl-lsp/tmp')/('tcl'+v)/'unix/tclsh',Path('/workspace/tcl-lsp/tmp')/('tcl'+v)/'library/init.tcl'))
providers.append(('jim-latest',Path('/workspace/tcl-lsp/tmp/jimtcl-oracle/jimsh'),None))
rows=[]
for label,exe,init in providers:
 d=out/label;d.mkdir(exist_ok=True);env=dict(os.environ);env.pop('TCL_LIBRARY',None)
 if init: env['TCL_LIBRARY']=str(init.parent);shutil.copy2(init,d/'original-init.tcl')
 before=hashlib.sha256(exe.read_bytes()).hexdigest()
 r=subprocess.run([str(exe),str(out/'probe.tcl')],capture_output=True,env=env,timeout=15)
 (d/'stdout.txt').write_bytes(r.stdout);(d/'stderr.txt').write_bytes(r.stderr)
 rows.append({'provider':label,'executable':str(exe),'executable_sha256_before':before,'executable_sha256_after':hashlib.sha256(exe.read_bytes()).hexdigest(),'original_init':str(init) if init else None,'init_sha256':hashlib.sha256(init.read_bytes()).hexdigest() if init else None,'exit_code':r.returncode,'command':[str(exe),str(out/'probe.tcl')],'stdout':r.stdout.decode(),'stderr':r.stderr.decode()})
(out/'result.json').write_text(json.dumps({'captured_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'Original C and latest Jim literal variable-name datum observations; no tcltest used, no F5 claim, no compiler acceptance claim.','providers':rows},indent=2)+'\n')
print(json.dumps(rows,indent=2))
