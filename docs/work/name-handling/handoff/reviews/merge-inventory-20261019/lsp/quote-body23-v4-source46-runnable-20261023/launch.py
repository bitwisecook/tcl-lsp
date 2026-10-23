#!/usr/bin/env python3
"""Durable file-backed launcher; no root GO means no runner, lock or Cargo."""
from pathlib import Path
import datetime, hashlib, json, os, subprocess
BASE=Path('/tmp/lsp-quote-body-finite-v4-source46-direct23')
GO=BASE/'root-go.json'; MANIFEST=BASE/'runner-input-manifest.json'
RUNS=Path('/tmp/lsp-quote-body-finite-v4-source46-direct23-launches')
def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def environment_override_keys(plan, environment):
    exact=set(plan['refused_environment_exact'])
    return sorted(key for key in environment if key in exact or any(key.startswith(pattern['prefix']) and key.endswith(pattern['suffix']) for pattern in plan['refused_environment_patterns']))

def utc(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
if not GO.is_file():
    (BASE/'launch-preflight-refusal.json').write_text(json.dumps({'status':'PASS no-GO launcher refusal','utc':utc(),'cargo_started':False,'runner_started':False},indent=2)+'\n')
    print('STOP: no root GO; no runner or Cargo started'); raise SystemExit(2)
if json.loads(GO.read_text()).get('runner_input_manifest_sha256')!=sha(MANIFEST):
    print('STOP: GO runner manifest pin mismatch'); raise SystemExit(2)
inputs=json.loads(MANIFEST.read_text())['inputs']
for path,expected in inputs.items():
    if sha(path)!=expected: print('STOP: runner input changed: '+path); raise SystemExit(2)
plan=json.loads((BASE/'plan.json').read_text())
if environment_override_keys(plan,os.environ): raise SystemExit('STOP: unsupported build environment override')
if sha(plan['cargo_path'])!=plan['cargo_sha256'] or sha(plan['rustc_path'])!=plan['rustc_sha256']: raise SystemExit('STOP: pinned Cargo/rustc hash mismatch')
env=os.environ.copy(); env.update(plan['required_environment']); env['PATH']=plan['path_prefix']+':'+env.get('PATH','')
RUNS.mkdir(exist_ok=True)
tag=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
out=RUNS/tag; out.mkdir(); transcript=out/'launcher.transcript'
meta={'started_utc':utc(),'pins_before':{p:sha(p) for p in inputs},'root_go_sha256':sha(GO),'manifest_sha256':sha(MANIFEST),'durable_file_backed_output':True}
(out/'launcher.json').write_text(json.dumps(meta,indent=2)+'\n')
with transcript.open('xb') as log:
    child=subprocess.run(['bash',str(BASE/'run.sh')],stdout=log,stderr=subprocess.STDOUT,check=False,env=env)
meta.update({'finished_utc':utc(),'exit_code':child.returncode,'pins_after':{p:sha(p) for p in inputs},'root_go_sha256_after':sha(GO),'manifest_sha256_after':sha(MANIFEST)})
meta['inputs_stable']=meta['pins_before']==meta['pins_after']
meta['go_stable']=meta['root_go_sha256']==meta['root_go_sha256_after']
meta['manifest_stable']=meta['manifest_sha256']==meta['manifest_sha256_after']
(out/'launcher.json').write_text(json.dumps(meta,indent=2)+'\n')
try: print(transcript.read_text(errors='replace')[-20000:],end='')
except (BrokenPipeError,OSError): pass
raise SystemExit(child.returncode if meta['inputs_stable'] and meta['go_stable'] and meta['manifest_stable'] else 2)
