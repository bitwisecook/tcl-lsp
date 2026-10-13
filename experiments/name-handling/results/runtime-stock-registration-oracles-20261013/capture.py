from pathlib import Path
import hashlib,json,subprocess,time
out=Path('/tmp/runtime-stock-registration-oracles-20261013');root=Path('/workspace/tcl-lsp/tmp');records=[]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']:
 tree=root/('tcl'+version);exe=tree/'unix/tclsh';paths={exe}
 paths.update(p for p in (tree/'library').rglob('*') if p.is_file())
 for line in subprocess.check_output(['ldd',str(exe)],text=True).splitlines():
  words=line.split();paths.update(Path(w) for w in words if w.startswith('/') and Path(w).is_file())
 before={str(p):sha(p) for p in sorted(paths)};start=time.monotonic();process=subprocess.run([str(exe),str(out/'input.tcl')],capture_output=True,timeout=30)
 d=out/version;d.mkdir(exist_ok=True);(d/'stdout.bin').write_bytes(process.stdout);(d/'stderr.bin').write_bytes(process.stderr)
 after={str(p):sha(p) for p in sorted(paths)}
 row={'provider':version,'argv':[str(exe),str(out/'input.tcl')],'input_sha256':sha(out/'input.tcl'),'stdout_sha256':sha(d/'stdout.bin'),'stderr_sha256':sha(d/'stderr.bin'),'exit_code':process.returncode,'elapsed_seconds':time.monotonic()-start,'before':before,'after':after,'drift':{p:[before[p],after[p]] for p in before if before[p]!=after[p]},'observations':sum(line.startswith(b'OBSERVATION ') for line in process.stdout.splitlines()),'scope':'Original C namespace private worker, configured relative map holder, relocation and replacement; counted source input; no tcltest'}
 (d/'record.json').write_text(json.dumps(row,indent=2)+'\n');records.append(row)
 print(version,process.returncode,row['observations'],len(row['drift']))
(out/'records.json').write_text(json.dumps(records,indent=2)+'\n')
