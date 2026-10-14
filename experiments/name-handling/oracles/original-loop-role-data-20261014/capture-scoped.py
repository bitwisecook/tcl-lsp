from pathlib import Path
import subprocess,hashlib,json,os,shutil,datetime
out=Path('/tmp/name-original-loop-role-oracles-scoped-20261014');out.mkdir(exist_ok=True)
lib=Path('/tmp/name-issues-refresh-20261014/tcllib-tcllib-2-0/modules')
probe=r'''foreach {$literal} {SIGIL} {}
puts [list literal-sigil [set {$literal}]]
foreach {[literal]} {BRACKET} {}
puts [list literal-command-name [set {[literal]}]]
foreach {{}} {EMPTY} {}
puts [list empty-name [set {}]]
foreach {{a b}} {SPACE} {}
puts [list grouped-name [set {a b}]]
foreach a\ b {FIRST SECOND} {}
puts [list escaped-word-list $a $b]
unset a
foreach a\x62 {HEX} {}
puts [list escaped-word-name $ab]
foreach {a($key)} {ELEMENT} {}
puts [list literal-index [set {a($key)}]]
set names {$literal}
foreach $names {DYNAMIC} {}
puts [list substituted-variable-list [set {$literal}]]
'''
(out/'probe.tcl').write_text(probe)
lineprobe=r'''set input [lindex $argv 0]
set channel [open $input w]
puts $channel FIRST
puts $channel SECOND
close $channel
proc inspect_line {command input} {
    foreach name {{a b} {$literal} {[literal]} {} {a($key)}} {
        set seen {}
        set code [catch {
            $command $name $input {lappend seen [set $name]}
        } value]
        puts [list receiver $command $name $code $seen $value]
        catch {unset $name}
    }
}
set core [catch {foreachLine probe $input {}} value]
puts [list core-availability $core $value]
if {!$core} {inspect_line foreachLine $input}
lappend auto_path LIBRARYPATH/fileutil LIBRARYPATH/cmdline
set loaded [catch {package require fileutil} value]
puts [list fileutil-availability $loaded $value]
if {!$loaded} {inspect_line ::fileutil::foreachLine $input}
file delete $input
'''.replace('LIBRARYPATH',str(lib))
(out/'file-line-probe.tcl').write_text(lineprobe)
providers=[(v,Path('/workspace/tcl-lsp/tmp')/('tcl'+v)/'unix/tclsh',Path('/workspace/tcl-lsp/tmp')/('tcl'+v)/'library/init.tcl') for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']]+[('jim-latest',Path('/workspace/tcl-lsp/tmp/jimtcl-oracle/jimsh'),None)]
rows=[]
for label,exe,init in providers:
 d=out/label;d.mkdir(exist_ok=True);env=dict(os.environ);env.pop('TCL_LIBRARY',None);env.pop('TCLLIBPATH',None)
 if init:env['TCL_LIBRARY']=str(init.parent);shutil.copy2(init,d/'original-init.tcl')
 before=hashlib.sha256(exe.read_bytes()).hexdigest();runs=[]
 for name,args in [('probe.tcl',[]),('file-line-probe.tcl',[str(d/'line-input.txt')])]:
  command=[str(exe),str(out/name)]+args;r=subprocess.run(command,capture_output=True,env=env,timeout=15)
  (d/(name+'.stdout.txt')).write_bytes(r.stdout);(d/(name+'.stderr.txt')).write_bytes(r.stderr)
  runs.append({'probe':name,'command':command,'exit_code':r.returncode,'stdout':r.stdout.decode(),'stderr':r.stderr.decode()})
 rows.append({'provider':label,'executable':str(exe),'executable_sha256_before':before,'executable_sha256_after':hashlib.sha256(exe.read_bytes()).hexdigest(),'original_init':str(init) if init else None,'init_sha256':hashlib.sha256(init.read_bytes()).hexdigest() if init else None,'runs':runs})
provenance=[]
for f in [lib/'fileutil/fileutil.tcl',lib/'fileutil/pkgIndex.tcl',lib/'cmdline/cmdline.tcl',lib/'cmdline/pkgIndex.tcl',Path('/workspace/tcl-lsp/tmp/tcl9.0.4/library/foreachline.tcl'),Path('/workspace/tcl-lsp/tmp/tcl9.1.0/library/foreachline.tcl')]:
 dest=out/'original-library'/('tcl'+str(f).split('/tmp/tcl')[-1].split('/library/')[0]+'-foreachline.tcl' if f.name=='foreachline.tcl' else str(f.parent.name)+'-'+f.name);dest.parent.mkdir(exist_ok=True);shutil.copy2(f,dest);provenance.append({'source':str(f),'sha256':hashlib.sha256(f.read_bytes()).hexdigest(),'retained':str(dest.relative_to(out))})
(out/'result.json').write_text(json.dumps({'captured_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'Original foreach list-word and real core/Tcllib file-line receiver executions; unsupported providers retained; matching C init, no tcltest or F5 result','library_sources':provenance,'providers':rows},indent=2)+'\n')
for row in rows:print(row['provider'],[(r['probe'],r['exit_code'],len(r['stderr']),r['stdout']) for r in row['runs']])
