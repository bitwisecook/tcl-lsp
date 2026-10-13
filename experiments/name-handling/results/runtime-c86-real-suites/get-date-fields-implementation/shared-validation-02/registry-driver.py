#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Capture the separately authorised first Registry build and exact control."""
from pathlib import Path
import datetime, gzip, hashlib, json, os, subprocess, sys
ROOT=Path('/workspace/tcl-lsp')
OUT=Path(__file__).resolve().parent
TARGET=ROOT/'tmp/name-target-runtime'
expected=json.loads((OUT/'source-readiness.json').read_text())['source_ready_sha256']
record={'cwd':str(ROOT),'target_dir':str(TARGET),'steps':[],'status':'initialising','observed_exact_names':[]}
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def save():(OUT/'registry-execution.json').write_text(json.dumps(record,indent=2)+'\n')
def freeze(label):
 actual={name:sha(ROOT/name) for name in expected}
 (OUT/('registry-'+label+'-source-hashes.json')).write_text(json.dumps(actual,indent=2)+'\n')
 assert actual==expected,'Frozen five sources changed'
def run(label,argv):
 step={'label':label,'argv':argv,'start_utc':now(),'stdout':label+'.stdout.gz','stderr':label+'.stderr.gz'}
 record['steps'].append(step);save()
 result=subprocess.run(argv,cwd=ROOT,capture_output=True)
 step.update(exit_code=result.returncode,end_utc=now())
 for role,data in [('stdout',result.stdout),('stderr',result.stderr)]:
  with gzip.open(OUT/(label+'.'+role+'.gz'),'wb') as stream:stream.write(data)
 save();print(label,'exit',result.returncode,flush=True)
 if result.returncode:
  print(result.stderr.decode(errors='replace')[-8000:],flush=True)
  raise RuntimeError('First Registry failure preserved: '+label)
 return result.stdout
try:
 assert os.environ['CARGO_TARGET_DIR']==str(TARGET)
 freeze('before')
 output=run('tcl-registry-compile',['cargo','test','--offline','-j2','-p','tcl-registry','--lib','--no-run','--message-format=json'])
 artifacts=[json.loads(line) for line in output.decode().splitlines() if line.startswith('{') and json.loads(line).get('reason')=='compiler-artifact']
 record['artifacts']=artifacts
 bins=[m['executable'] for m in artifacts if m.get('executable') and m['target']['name']=='tcl_registry' and m['profile']['test']]
 assert len(bins)==1,bins
 binary=Path(bins[0]);record['binary_path']=str(binary);record['before_binary_sha256']=sha(binary);save()
 listing=run('tcl-registry-list',[str(binary),'--list'])
 names=[line.removesuffix(': test') for line in listing.decode().splitlines() if line.endswith(': test')]
 test='commands::tcl::clock_::tests::private_date_worker_is_c86_only_shipped_dict_and_null_hook'
 assert test in names,test
 record['observed_exact_names']=[test];record['listed_test_count']=len(names);save()
 result=run('tcl-registry-exact-1',[str(binary),'--exact',test,'--nocapture'])
 assert b'running 1 test' in result and b'1 passed; 0 failed' in result,result
 record['steps'][-1]['executed_test_count']=1
 record['after_binary_sha256']=sha(binary)
 assert record['before_binary_sha256']==record['after_binary_sha256']
 with gzip.open(OUT/'tcl-registry-native-test-binary.gz','wb') as stream:stream.write(binary.read_bytes())
 freeze('after')
 record['status']='first-registry-build-and-exact-control-passed-cache-released'
except Exception as error:
 record['status']='first-registry-failure-preserved-cache-released';record['error']=str(error)
 try:freeze('after-failure')
 except Exception as hash_error:record['hash_error']=str(hash_error)
 print(record['status'],record['error'],flush=True)
finally:
 record['cache_released_utc']=now();save()
print(record['status'],flush=True)
sys.exit(0 if record['status']=='first-registry-build-and-exact-control-passed-cache-released' else 1)
