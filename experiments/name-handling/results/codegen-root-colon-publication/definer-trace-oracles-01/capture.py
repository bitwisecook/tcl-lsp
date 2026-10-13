import pathlib,hashlib,subprocess,os,json,time,shutil
root=pathlib.Path('/workspace/name-implementation-worktrees/codegen');out=root/'experiments/name-handling/results/codegen-root-colon-publication/definer-trace-oracles-01';out.mkdir(parents=True,exist_ok=False)
p=out/'enter-trace-replaces-definer.tcl';shutil.copyfile('/tmp/name-codegen-trace-definer-replacement.tcl',p)
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
record={'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'scope':'Direct actual enter-trace mutation, definer selection and absence of publication. No compiler, native-entry, Tcltest or F5 acceptance. Jim latest upstream freshness is not reverified by this capture.','runs':[]}
env=os.environ.copy();env.pop('TCL_LIBRARY',None)
for version,var in {'8.4.20':'TCL_LSP_TCLSH84','8.5.19':'TCL_LSP_TCLSH85','8.6.18':'TCL_LSP_TCLSH86','9.0.4':'TCL_LSP_TCLSH90','9.1.0':'TCL_LSP_TCLSH91','jim':'TCL_LSP_JIMSH'}.items():
 b=pathlib.Path(os.environ[var]);before=digest(b);args=[str(b),str(p)];r=subprocess.run(args,cwd=root,env=env,capture_output=True,timeout=30);(out/(version+'.stdout')).write_bytes(r.stdout);(out/(version+'.stderr')).write_bytes(r.stderr)
 record['runs'].append({'version':version,'argv':args,'environment_changes':{'TCL_LIBRARY':'unset'},'input_sha256':digest(p),'binary_sha256_before':before,'binary_sha256_after':digest(b),'exit_code':r.returncode,'stdout':version+'.stdout','stderr':version+'.stderr','latest_upstream_verified':False if version=='jim' else None});print(version,r.returncode,r.stdout.decode().strip(),r.stderr.decode().strip())
(out/'answers.json').write_text(json.dumps(record,indent=2)+'\n');shutil.copyfile(__file__,out/'capture.py')
