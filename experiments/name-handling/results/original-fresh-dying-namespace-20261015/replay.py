from pathlib import Path
import argparse, hashlib, json, os, subprocess
p=argparse.ArgumentParser();p.add_argument('--out',required=True,type=Path);a=p.parse_args();here=Path(__file__).resolve().parent
if a.out.exists():raise SystemExit('Refusing existing output directory')
sha=lambda q:hashlib.sha256(q.read_bytes()).hexdigest();plans=[]
for label in ['legacy-trace','trace-add']:
 capture=here if label=='legacy-trace' else here/'trace-add';m=json.loads((capture/'manifest.json').read_text());assert sha(capture/'probe.tcl')==m['probe_sha256']
 for row in m['rows']:
  assert row['original_files_before']==row['original_files_after']
  for file,h in row['original_files_before'].items():assert sha(Path(file))==h,file
  plans.append((label,capture,row))
a.out.mkdir(parents=True);runs=[]
for label,capture,row in plans:
 target=a.out/label;target.mkdir(exist_ok=True);env=os.environ.copy();env.pop('TCLLIBPATH',None)
 if row['selected_original_library']:env['TCL_LIBRARY']=row['selected_original_library']
 else:env.pop('TCL_LIBRARY',None)
 run=subprocess.run([row['argv'][0],str(capture/'probe.tcl')],env=env,capture_output=True);stem=row['provider_label'].replace('.','-');(target/(stem+'.stdout')).write_bytes(run.stdout);(target/(stem+'.stderr')).write_bytes(run.stderr)
 for file,h in row['original_files_before'].items():assert sha(Path(file))==h,file
 runs.append({'capture':label,'provider':row['provider_label'],'exit_code':run.returncode,'stdout_sha256':hashlib.sha256(run.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(run.stderr).hexdigest(),'selected_library':row['selected_original_library']})
(a.out/'replay-result.json').write_text(json.dumps({'runs':runs,'scope':'Separate reproduction, no acceptance inferred from exit0 or unavailable trace providers.'},indent=2)+'\n')
