#!/usr/bin/env python3
"""Root-only explicit targeted12 OR unchanged retained584 canonical replay launcher."""
import argparse,datetime,json,subprocess,sys
from pathlib import Path
sys.dont_write_bytecode=True
import guards
if __name__=='__main__':
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--mode',choices=['targeted12','retained584'],required=True);ap.add_argument('--build-dir',required=True,type=Path);ap.add_argument('--manifest',required=True,type=Path);ap.add_argument('--manifest-sha256',required=True);ap.add_argument('--out',required=True,type=Path);args=ap.parse_args()
 assert not args.out.exists() and not args.out.resolve().is_relative_to(guards.ROOT.resolve()),'New external evidence directory required'
 assert guards.sha(args.manifest)==args.manifest_sha256,'Explicit selection digest mismatch'
 m=json.loads(args.manifest.read_text());fullpath=args.manifest
 if args.mode=='targeted12':
  fullpath=Path(m['full584']['path']);assert guards.sha(fullpath)==m['full584']['sha256']
 full,binding,inputs,artifacts=guards.validate(fullpath,args.build_dir);required=guards.environment()
 if args.mode=='targeted12':
  assert m['total_exact_selectors']==12 and m['execution_order']==guards.subset_rows(full) and m['full584_selected'] is False
  assert m['actual_successful_freeze_binding']==binding and Path(m['actual_successful_build_dir']).resolve()==args.build_dir.resolve()
  assert m['preparation_guard_sha256']==guards.sha(guards.PACKAGE/'guards.py')
  assert m['preparation_generator_sha256']==guards.sha(guards.PACKAGE/'generate_subset.py')
  for k in ['environment','runtime_guards','test_timeout_seconds','listing_timeout_seconds']:assert m[k]==full[k]
 else:assert m==full and m['total_exact_selectors']==584
 assert sum(map(len,m['groups'].values()))==len(m['execution_order'])==m['total_exact_selectors']
 assert {t:len(v) for t,v in m['groups'].items()}==m['expected_group_counts']
 assert {(r['target'],r['selector']) for r in m['execution_order']}=={(t,s) for t,ss in m['groups'].items() for s in ss}
 argv=['python3',str(guards.RUNNER),'--manifest',str(args.manifest.resolve()),'--manifest-sha256',args.manifest_sha256,'--artifacts',str(args.build_dir.resolve()/'artifacts.json'),'--root',str(guards.ROOT),'--out',str(args.out.resolve())]
 receiptpath=args.out.with_name(args.out.name+'-invocation.json');assert not receiptpath.exists();receiptpath.parent.mkdir(parents=True,exist_ok=True)
 receipt={'status':'PREFLIGHT_PASS_'+args.mode.upper(),'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'argv':argv,'scope':args.mode,'full584_selected':args.mode=='retained584','selected_count':m['total_exact_selectors'],'actual_successful_binding':binding,'full39001_current_source_checked':True,'all_five_actual_frozen_ELFs_checked':True,'required_actual_provider_environment':required,'canonical_runner_sha256':guards.RUNNER_SHA}
 receiptpath.write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({'status':receipt['status'],'selected_count':m['total_exact_selectors']}),flush=True)
 code=1
 try:
  code=subprocess.run(argv,cwd=guards.ROOT).returncode
 finally:
  receipt['canonical_exit_code']=code
  try:
   guards.exact_source(inputs);guards.exact_artifacts(artifacts,args.build_dir.resolve());assert guards.sha(args.manifest)==args.manifest_sha256
   receipt['all39001_input_and_all5_ELF_postflight_match']=True
  except Exception as error:
   receipt['postflight_error']=repr(error);code=1
  receipt['completed_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat();receiptpath.write_text(json.dumps(receipt,indent=2)+'\n')
 raise SystemExit(code)
