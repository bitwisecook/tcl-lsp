#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
import argparse,hashlib,json,os,re,subprocess,time
from pathlib import Path
parser=argparse.ArgumentParser(description="Build, run both complete report-control inventories, and delete executables")
parser.add_argument('root',type=Path)
parser.add_argument('--evidence',type=Path,required=True)
parser.add_argument('--target-dir',type=Path)
parser.add_argument('--opt-level',choices=['0','1'],default='0')
args=parser.parse_args()
root=args.root.resolve()
evidence=args.evidence.resolve()
evidence.mkdir(parents=True,exist_ok=False)
os.environ.update(CARGO_INCREMENTAL='0',CARGO_PROFILE_TEST_DEBUG='0',
 CARGO_PROFILE_TEST_OPT_LEVEL=args.opt_level,CARGO_PROFILE_DEV_OPT_LEVEL=args.opt_level,
 CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true',CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
 CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true')
os.environ.pop('ZZ_OLD_ENV',None)
if args.target_dir:os.environ['CARGO_TARGET_DIR']=str(args.target_dir.resolve())
artifacts=set(); results=[]
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def seal():
 files={}
 for top in ['rust','runtime','specs']:
  for p in sorted((root/top).rglob('*')):
   if p.is_file(): files[str(p.relative_to(root))]=sha(p)
 for name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml']:
  files[name]=sha(root/name)
 return files
def record():
 (evidence/'progress.json').write_text(json.dumps({'results':results,'updated_unix':time.time()},indent=2)+'\n')
def command(label,argv,extra=None,timeout=None):
 env=os.environ.copy();env.update(extra or {})
 start=time.time()
 with (evidence/(label+'.out')).open('w') as out:
  try: code=subprocess.run(argv,cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT,timeout=timeout).returncode
  except subprocess.TimeoutExpired: code='timeout'
 row={'label':label,'argv':list(map(str,argv)),'extra_env':extra or {},'exit':code,'wall_s':time.time()-start}
 results.append(row);record();print(json.dumps(row),flush=True);return code
before=seal();(evidence/'source-before.json').write_text(json.dumps(before,sort_keys=True,indent=2)+'\n')
inputs={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'profile':f'dev/test opt-level={args.opt_level} debug=0 incremental=0; debug/overflow assertions enabled','rust_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','--','rust'],cwd=root)).hexdigest(),'acceptance':'all three report comparisons Equal; strict runtime equality recorded separately','started_unix':time.time()}
(evidence/'inputs.json').write_text(json.dumps(inputs,indent=2)+'\n')
def build(label,argv):
 start=time.time()
 with (evidence/(label+'.jsonl')).open('w') as out,(evidence/(label+'.stderr')).open('w') as err:
  code=subprocess.run(argv,cwd=root,stdout=out,stderr=err).returncode
 found={};warnings=[]
 for line in (evidence/(label+'.jsonl')).read_text().splitlines():
  try: row=json.loads(line)
  except json.JSONDecodeError: continue
  if row.get('reason')=='compiler-artifact' and row.get('executable'):
   p=Path(row['executable']);artifacts.add(p);found[row['target']['name']]=p
  if row.get('reason')=='compiler-message' and row['message']['level']=='warning': warnings.append(row['message'])
 results.append({'label':label,'argv':argv,'exit':code,'wall_s':time.time()-start,'warnings':len(warnings),'executables':{n:str(p) for n,p in found.items()}});record()
 (evidence/(label+'-warnings.json')).write_text(json.dumps(warnings,indent=2)+'\n')
 print(label,'exit',code,'warnings',len(warnings),flush=True)
 if code or warnings: raise RuntimeError(f'{label}: exit={code}, warnings={len(warnings)}')
 return found
try:
 exes=build('build-controls',['cargo','test','--locked','-j2','-p','tcl-compiler','--lib','--no-run','--message-format=json'])
 exe=exes['tcl_compiler'];(evidence/'controls-binary.json').write_text(json.dumps({'path':str(exe),'sha256':sha(exe)},indent=2)+'\n')
 command('controls-list',[str(exe),'authenticated_report','full_report_candidates_require_the_actual_diagnostic_output_realm','--list'])
 command('controls-new',[str(exe),'authenticated_report','full_report_candidates_require_the_actual_diagnostic_output_realm','--test-threads=2'],timeout=2400)
 command('controls-old',[str(exe),'authenticated_report','full_report_candidates_require_the_actual_diagnostic_output_realm','--test-threads=2'],{'ZZ_OLD_ENV':'1'},timeout=2400)
 listed=set(re.findall(r'^(.+): test$',(evidence/'controls-list.out').read_text(),re.M))
 assert listed,'empty control inventory'
 for mode in ['new','old']:
  observed=dict(re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)$',(evidence/('controls-'+mode+'.out')).read_text(),re.M))
  assert set(observed)==listed,'incomplete control inventory'
  assert all(value=='ok' for value in observed.values()),mode+' controls failed'

finally:
 after=seal();(evidence/'source-after.json').write_text(json.dumps(after,sort_keys=True,indent=2)+'\n')
 cleanup=[]
 for exe in artifacts:
  aliases=[exe]
  if exe.parent.name=='examples': aliases+=list(exe.parent.glob(exe.name+'-*'))
  for p in aliases:
   if p.is_file() and p.suffix not in ['.d','.rmeta','.rlib']:
    p.unlink();cleanup.append(str(p))
 for name in ['zz_authenticated_perfprobe.rs','zz_spectclprobe.rs']:(root/'rust/tcl-compiler/examples'/name).unlink(missing_ok=True)
 (evidence/'final.json').write_text(json.dumps({'results':results,'source_unchanged':before==after,'removed_executables':cleanup,'finished_unix':time.time()},indent=2)+'\n')
 print('FINISHED source_unchanged',before==after,flush=True)
