import pathlib,hashlib,subprocess,os,json,time,shutil
root=pathlib.Path('/workspace/name-implementation-worktrees/codegen')
out=root/'experiments/name-handling/results/codegen-structured-procedure-slots/direct-oracles-01'
out.mkdir(parents=True,exist_ok=False)
programs={'same-display-distinct-contexts':pathlib.Path('/tmp/name-codegen-structured-collision.tcl').read_text(),'sequential-same-slot':'''proc audit {} {return FIRST}
puts [list before [audit]]
proc audit {} {return SECOND}
puts [list after [audit]]
'''}
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
record={'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'scope':'Direct actual command publication, invocation and replacement. Compiler maps, native entry admission, tcltest and F5 are not exercised. Jim upstream freshness is not verified by this capture.','git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'runs':[]}
versions={'8.4.20':'TCL_LSP_TCLSH84','8.5.19':'TCL_LSP_TCLSH85','8.6.18':'TCL_LSP_TCLSH86','9.0.4':'TCL_LSP_TCLSH90','9.1.0':'TCL_LSP_TCLSH91','jim':'TCL_LSP_JIMSH'}
env=os.environ.copy();env.pop('TCL_LIBRARY',None)
for case,program in programs.items():
 p=out/(case+'.tcl');p.write_text(program)
 for version,var in versions.items():
  binary=pathlib.Path(os.environ[var]);before=digest(binary);args=[str(binary),str(p)];r=subprocess.run(args,cwd=root,env=env,capture_output=True,timeout=30);label=case+'-'+version
  (out/(label+'.stdout')).write_bytes(r.stdout);(out/(label+'.stderr')).write_bytes(r.stderr)
  record['runs'].append({'case':case,'version':version,'argv':args,'environment_changes':{'TCL_LIBRARY':'unset'},'input_sha256':digest(p),'binary_sha256_before':before,'binary_sha256_after':digest(binary),'exit_code':r.returncode,'stdout':label+'.stdout','stderr':label+'.stderr','latest_upstream_verified':False if version=='jim' else None})
(out/'answers.json').write_text(json.dumps(record,indent=2)+'\n')
shutil.copyfile(__file__,out/'capture.py')
(out/'README.md').write_text('# Procedure slots, display collisions and sequential replacement\n\nC Tcl 8.6.18, 9.0.4 and 9.1.0 keep namespace `a:` / leaf `p` and namespace `a` / leaf `:p` distinct. Both `namespace which -command` results render as `::a:::p`; their namespace contexts and return values distinguish them. Replacing the first command changes LEFT to REPLACED and preserves the second command\'s RIGHT result.\n\nC Tcl 8.4.20 and 8.5.19 reject the second definition: `can\'t create procedure ":p" in non-global namespace with name starting with ":"`. Those two scripts exit1 and their original stderr is retained. The collision is not admitted uniformly across the five C versions.\n\nThe captured Jim binary selects its flat-name policy: both calls return RIGHT after the second definition and REPLACED after replacement. The identical display is an actual shared command identity there. Compiler identity and lookup must retain the chosen naming policy and publication grammar.\n\nAll six interpreters return FIRST before and SECOND after sequential replacement of `audit`. Ten of the twelve runs exit0 with empty stderr; the two older-C collision runs retain the original refusal. Inputs, exact argv, raw outputs and before/after executable hashes are retained in answers.json. No compiler map, native table, Tcltest or F5 acceptance is implied. Jim upstream freshness is independently established elsewhere and is not reverified here.\n\nThe BIG-IP chat can execute both retained programs unchanged and report exact version, completion status, stdout/stderr, namespace contexts, displayed names and before/after results in this branch. The question is whether F5 admits both declarations, keeps the contextual commands separate and preserves one while replacing the other; no provider result is assumed for F5.\n')
checks={p.name:digest(p) for p in sorted(out.iterdir()) if p.is_file()}
(out/'checksums.json').write_text(json.dumps(checks,indent=2)+'\n')
print(out)
for r in record['runs']:print(r['case'],r['version'],r['exit_code'],(out/r['stdout']).read_text().strip())
