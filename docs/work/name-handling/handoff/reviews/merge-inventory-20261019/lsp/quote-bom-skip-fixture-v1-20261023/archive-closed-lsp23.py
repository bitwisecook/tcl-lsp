#!/usr/bin/env python3
"""Archive one already closed actual run losslessly; never launch or repair it."""
import argparse,pathlib,json,hashlib,gzip,datetime
p=argparse.ArgumentParser();p.add_argument('--run',required=True,type=pathlib.Path);p.add_argument('--root-parent-closure',required=True,type=pathlib.Path);p.add_argument('--destination',required=True,type=pathlib.Path);a=p.parse_args()
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
launch=json.loads(a.root_parent_closure.read_text());assert launch.get('kind')=='root-observed-direct-parent-closure' and launch.get('tool_session_id')==10730 and launch.get('observed_utc') and type(launch.get('exit_code')) is int,'actual root-observed direct parent not closed'
released=json.loads((a.run/'lock-released.json').read_text());assert released.get('released') is True,'actual suite lock-release receipt absent/incomplete'
summary=a.run/'summary.json';failure=a.run/'failure.json';assert summary.is_file() or failure.is_file(),'no terminal run receipt'
plan=json.loads((a.run/'plan.json').read_text());rows=json.loads((a.run/'results.json').read_text()) if (a.run/'results.json').is_file() else []
roster=plan['selectors'];assert len(roster)==23 and len(rows)<=23
for row,want in zip(rows,roster,strict=False):assert row['selector']==want['selector'] and row['package']==want['package'] and row['actual_selected']==1
full=len(rows)==23 and summary.is_file();terminal=json.loads((summary if summary.is_file() else failure).read_text())
base=pathlib.Path('/tmp/lsp-quote-body-finite-v5-source46-repaired-direct23');manifest=json.loads((base/'runner-input-manifest.json').read_text());groups={'run':a.run,'protocol':base};files={'root-parent-closure/'+a.root_parent_closure.name:a.root_parent_closure}
for group,d in groups.items():
 for q in sorted(d.rglob('*')):
  if q.is_file():files[group+'/'+str(q.relative_to(d))]=q
for index,path in enumerate(manifest['inputs']):
 q=pathlib.Path(path);assert q.is_file() and sha(q)==manifest['inputs'][path],str(q);files['transitive-inputs/'+str(index)+'-'+q.name]=q
pins={name:sha(q) for name,q in files.items()};assert not a.destination.exists(),'immutable destination exists';a.destination.mkdir(parents=True)
for name,q in files.items():
 dest=a.destination/(name+'.gz');dest.parent.mkdir(parents=True,exist_ok=True)
 with dest.open('wb') as stream:
  with gzip.GzipFile(filename='',fileobj=stream,mode='wb',mtime=0) as z:z.write(q.read_bytes())
 with gzip.open(dest,'rb') as z:assert hashlib.sha256(z.read()).hexdigest()==pins[name]
assert all(sha(q)==pins[name] for name,q in files.items()),'evidence changed during archive'
receipt={'actual_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'Closed actual run archival only; incomplete/unexecuted controls remain unknown','run':str(a.run),'root_parent_closure':str(a.root_parent_closure),'root_parent_exit':launch['exit_code'],'launch_kind':'root observed direct sh run.sh tool session10730 (not PID); protocol launch.py was NOT used','observed_rows':len(rows),'observed_selected':sum(r['actual_selected'] for r in rows),'passed':sum(r['actual_passed'] for r in rows),'failed':sum(r['actual_failed'] for r in rows),'unexecuted_selectors':[r['selector'] for r in roster[len(rows):]],'full23_terminal':full,'terminal_original':terminal,'source_provider_stable':terminal.get('source_provider_stable') is True,'artifact_stable':terminal.get('artifact_stable') is True,'files':{n:{'original_path':str(files[n]),'sha256':h} for n,h in pins.items()},'all_gzip_roundtrips_verified':True,'all_original_bytes_stable_during_copy':True};(a.destination/'archive-summary.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({k:v for k,v in receipt.items() if k!='files'},indent=2))
