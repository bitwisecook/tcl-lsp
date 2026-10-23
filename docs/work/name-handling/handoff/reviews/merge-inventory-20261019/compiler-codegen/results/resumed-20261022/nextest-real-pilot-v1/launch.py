#!/usr/bin/env python3
"""Root-only durable launcher; no root GO means no child or Cargo."""
from pathlib import Path
import argparse,datetime,hashlib,json,subprocess,sys
BASE=Path('/tmp/ccg-nextest-real-pilot-v1')
p=argparse.ArgumentParser();p.add_argument('--go',type=Path,required=True);a=p.parse_args()
if not a.go.is_file():raise SystemExit('STOP: root GO absent; no child/Cargo/queue')
sha=lambda x:hashlib.sha256(Path(x).read_bytes()).hexdigest()
go=json.loads(a.go.read_text());inputs=BASE/'runner-input-manifest.json'
if go.get('purpose')!='nextest-real-pilot-v1' or go.get('runner_input_manifest_sha256')!=sha(inputs) or go.get('plan_sha256')!=sha(BASE/'plan.json'):raise SystemExit('STOP: exact reviewed root GO required')
for path,expected in json.loads(inputs.read_text())['inputs'].items():
 if sha(path)!=expected:raise SystemExit('STOP: input changed '+path)
out=BASE/'launches'/datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ');out.mkdir(parents=True)
with (out/'transcript').open('xb') as stream:
 child=subprocess.run(['bash',str(BASE/'run.sh'),'--go',str(a.go.resolve())],stdout=stream,stderr=subprocess.STDOUT,check=False)
(out/'launcher.json').write_text(json.dumps({'exit_code':child.returncode,'go_sha256':sha(a.go),'manifest_sha256':sha(inputs),'transcript_sha256':sha(out/'transcript')},indent=2)+'\n')
print('pilot launcher receipt='+str(out)+' exit='+str(child.returncode))
raise SystemExit(child.returncode)
