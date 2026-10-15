from pathlib import Path
import hashlib,json,datetime,subprocess,os
base=Path('/workspace/name-reference-implicit-path-oracles-20261015');base.mkdir(exist_ok=False)
root=Path('/workspace/tcl-lsp')
probe='''if {[llength [info commands ::oo::class]] == 0} {
    puts {TCLOO unavailable}
} else {
    proc ::oo::Helpers::sharedHelper {} {return HELPERS}
    proc ::oo::sharedDirect {} {return OO}
    ::oo::class create NamingProbe {
        method examine {} {list [sharedHelper] [sharedDirect] [namespace path]}
    }
    NamingProbe create namingProbe
    puts [namingProbe examine]
    puts [list [catch {sharedHelper} a] $a [catch {sharedDirect} b] $b]
}
'''
(base/'probe.tcl').write_text(probe)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
rows=[]
for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','Jim']:
 tree=root/'tmp'/('jimtcl-oracle' if v=='Jim' else 'tcl'+v)
 exe=tree/('jimsh' if v=='Jim' else 'unix/tclsh')
 env=os.environ.copy();env.pop('TCLLIBPATH',None);env.pop('TCL_LIBRARY',None)
 originals=[exe]
 if v!='Jim':env['TCL_LIBRARY']=str(tree/'library');originals.append(tree/'library/init.tcl')
 before={str(p):sha(p) for p in originals}
 r=subprocess.run([str(exe)],input=probe.encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env)
 (base/(v+'.stdout')).write_bytes(r.stdout);(base/(v+'.stderr')).write_bytes(r.stderr)
 after={str(p):sha(p) for p in originals};assert before==after
 rows.append({'version':v,'argv':[str(exe)],'original_files_before':before,'original_files_after':after,'exit_code':r.returncode,'stdout':r.stdout.decode(),'stderr':r.stderr.decode(),'selected_original_library':env.get('TCL_LIBRARY')})
manifest={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'probe_sha256':sha(base/'probe.tcl'),'scope':'Actual interpreter TclOO member implicit namespace path and top-level refusal; no tcltest used, no F5 observations. Jim uses existing locally pinned oracle, latest verification receipt retained separately.','rows':rows}
(base/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
for row in rows:print(row['version'],row['exit_code'],row['stdout'].strip(),row['stderr'].strip())
