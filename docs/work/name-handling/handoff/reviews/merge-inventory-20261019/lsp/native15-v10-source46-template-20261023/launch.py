#!/usr/bin/env python3
"""Durable file-backed launcher; no root GO means no runner, lock or Cargo."""
from pathlib import Path
import datetime, hashlib, json, subprocess
BASE=Path('/tmp/lsp-native-protocol-v10-source46-direct15-template')
GO=BASE/'root-go.json'; MANIFEST=BASE/'runner-input-manifest.json'
RUNS=Path('/tmp/lsp-native-protocol-v10-source46-direct15-template-launches')
def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def utc(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
if not GO.is_file():
    (BASE/'launch-preflight-refusal.json').write_text(json.dumps({'status':'PASS no-GO launcher refusal','utc':utc(),'cargo_started':False,'runner_started':False},indent=2)+'\n')
    print('STOP: no root GO; no runner or Cargo started'); raise SystemExit(2)
if json.loads(GO.read_text()).get('runner_input_manifest_sha256')!=sha(MANIFEST):
    print('STOP: GO runner manifest pin mismatch'); raise SystemExit(2)
inputs=json.loads(MANIFEST.read_text())['inputs']
for path,expected in inputs.items():
    if sha(path)!=expected: print('STOP: runner input changed: '+path); raise SystemExit(2)
RUNS.mkdir(exist_ok=True)
tag=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
out=RUNS/tag; out.mkdir(); transcript=out/'launcher.transcript'
meta={'started_utc':utc(),'pins_before':{p:sha(p) for p in inputs},'root_go_sha256':sha(GO),'manifest_sha256':sha(MANIFEST),'durable_file_backed_output':True}
(out/'launcher.json').write_text(json.dumps(meta,indent=2)+'\n')
with transcript.open('xb') as log:
    child=subprocess.run(['bash',str(BASE/'run.sh')],stdout=log,stderr=subprocess.STDOUT,check=False)
meta.update({'finished_utc':utc(),'exit_code':child.returncode,'pins_after':{p:sha(p) for p in inputs},'root_go_sha256_after':sha(GO),'manifest_sha256_after':sha(MANIFEST)})
meta['inputs_stable']=meta['pins_before']==meta['pins_after']
meta['go_stable']=meta['root_go_sha256']==meta['root_go_sha256_after']
meta['manifest_stable']=meta['manifest_sha256']==meta['manifest_sha256_after']
(out/'launcher.json').write_text(json.dumps(meta,indent=2)+'\n')
try: print(transcript.read_text(errors='replace')[-20000:],end='')
except (BrokenPipeError,OSError): pass
raise SystemExit(child.returncode if meta['inputs_stable'] and meta['go_stable'] and meta['manifest_stable'] else 2)
