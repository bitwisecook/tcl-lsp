#!/usr/bin/env python3
"""Root-only source/hash subset preparation after real successful584 freeze; no lists/tests."""
import argparse,copy,json,sys
from pathlib import Path
sys.dont_write_bytecode=True
import guards
if __name__=='__main__':
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--build-dir',required=True,type=Path);ap.add_argument('--frozen584',required=True,type=Path);ap.add_argument('--output',required=True,type=Path);args=ap.parse_args()
 assert not args.output.exists() and not args.output.resolve().is_relative_to(guards.ROOT.resolve()),'Immutable TMP/evidence output required'
 full,binding,inputs,artifacts=guards.validate(args.frozen584,args.build_dir);rows=guards.subset_rows(full)
 groups={}
 for row in rows:groups.setdefault(row['target'],[]).append(row['selector'])
 selection={'group':'Targeted12 ONLY: original222/223/224/232/242/270 + six exact actual owner/adopter controls','scope':'Twelve only; no full584 replay/completion/acceptance claim','total_exact_selectors':12,'groups':groups,'expected_group_counts':{k:len(v) for k,v in groups.items()},'execution_order':rows,'prepared_only':True,'full584_selected':False,'full584':{'path':str(args.frozen584.resolve()),'sha256':guards.sha(args.frozen584),'ordered_rows_retained':584},'actual_successful_build_dir':str(args.build_dir.resolve()),'actual_successful_freeze_binding':copy.deepcopy(binding),'preparation_package':str(guards.PACKAGE),'preparation_guard_sha256':guards.sha(guards.PACKAGE/'guards.py'),'preparation_generator_sha256':guards.sha(Path(__file__))}
 for k in ['environment','runtime_guards','test_timeout_seconds','listing_timeout_seconds']:selection[k]=copy.deepcopy(full[k])
 args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(selection,indent=2)+'\n')
 summary={'status':'PREPARED_TARGETED12_ONLY_NO_EXECUTION','manifest':str(args.output.resolve()),'sha256':guards.sha(args.output),'expected_group_counts':selection['expected_group_counts'],'actual_build_dir':selection['actual_successful_build_dir'],'all39001_actual_current_inputs_checked':True,'all_five_actual_frozen_ELFs_checked':True,'full584_untouched_sha256':guards.sha(args.frozen584),'execution':'No listing/test/build/main invocation'}
 receipt=args.output.with_name(args.output.stem+'-preparation.json');assert not receipt.exists();receipt.write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
