#!/usr/bin/env python3
"""Portable retained583 successor. Source/hash preparation only; never execute.

--source-only reads actual integrated source and never binds a guessed ELF.
--freeze requires Root's successful ordinary five-library receipts, full input
index and all five actual frozen ELF bytes. Listing/singleton and 180/30 second
budgets remain the canonical runner's duties. Historical receipts in inherited
metadata remain historical, never the successor's current binary authority.
"""
import argparse,copy,gzip,hashlib,json,sys
from collections import Counter
from pathlib import Path
sys.dont_write_bytecode=True
import source_tools as tools
P=Path(__file__).resolve().parent
BASE_SHA='1049737d008e3d215bc9552943f4513adc2bc2dfe757a388d36ed30a423b3911'
GUARDS=['test_timeout_seconds','listing_timeout_seconds','environment','runtime_guards','validation_note','host_scope']

def sha(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def generate(args):
 tools.ROOT=args.root.resolve()
 tools.TARGETS={n.replace('-','_'):tools.ROOT/'rust'/n/'src/lib.rs' for n in ['tcl-compiler','tcl-lsp-core','tcl-lsp-db','tcl-registry','tcl-lsp-server']}
 raw=gzip.decompress((P/'retained583.json.gz').read_bytes());assert hashlib.sha256(raw).hexdigest()==BASE_SHA
 old=json.loads(raw);sel=copy.deepcopy(old);prior=copy.deepcopy(old['execution_order'])
 assert len(prior)==old['total_exact_selectors']==583
 assert old['test_timeout_seconds']==180 and old['listing_timeout_seconds']==30
 assert Counter(r['target'] for r in prior)==old['expected_group_counts']
 seen={(r['target'],r['selector']) for r in prior};assert len(seen)==583
 req=json.loads((P/'requests.json').read_text());requests=copy.deepcopy(req['requests']);added=[]
 for row in requests:
  key=row['target'],row['selector'];assert row['target'] in tools.TARGETS
  row['already_retained']=key in seen
  if key not in seen:
   seen.add(key);added.append({k:row[k] for k in ['target','selector','purpose']})
 sel['execution_order'].extend(added);assert sel['execution_order'][:583]==prior
 sel['groups']={t:[r['selector'] for r in sel['execution_order'] if r['target']==t] for t in old['groups']}
 sel['expected_group_counts']={t:len(v) for t,v in sel['groups'].items()}
 sel['total_exact_selectors']=len(seen)
 sel['group']='Retained583 + exact actual conditional-case consumer refusal regression; all historical460 remain ordered and unchanged'
 sel['successor_destination']=str(args.destination) if args.destination else None
 index=tools.SourceIndex()
 evidence=[tools.source_eligibility(index,index.resolve(r['target'],r['selector'],r['purpose'])) for r in sel['execution_order']]
 files={}
 for row in evidence:
  for item in [row]+row['module_route']:
   p,s=item['source_path'],item['source_sha256'];assert p not in files or files[p]==s;files[p]=s
 # No changed source after first capture can be silently accepted.
 assert all(sha(tools.ROOT/p)==s for p,s in files.items())
 binding=dict(status='SOURCE_ONLY_AWAITING_ROOT_SUCCESSFUL_FIVE_LIBRARY_BUILD',binary_bindings=None,current_listing_bindings=None,no_guessed_or_predecessor_elf=True)
 if args.freeze:
  assert args.destination is not None,'--freeze requires actual Root destination'
  d=args.destination.resolve();bp=d/'source-binding.json';ip=d/'input-before.json';ap=d/'artifacts.json'
  build=json.loads(bp.read_text());artifacts=json.loads(ap.read_text());inputs=json.loads(ip.read_text())
  assert build['ordinary'] is True and build['instrumentation'] is False and build['exit_code']==0 and build['input_drift_count']==0
  assert len(build['packages'])==5 and {v.replace('-','_') for v in build['packages']}==set(tools.TARGETS)==set(artifacts)
  assert build['input_before_count']==build['input_after_count']==len(inputs)
  frozen={r['path']:r['sha256'] for r in inputs};assert len(frozen)==len(inputs)
  assert all(frozen.get(p)==s for p,s in files.items()),'Selected current source/module routes do not match frozen build'
  for row in inputs:
   path=tools.ROOT/row['path'];assert path.is_file() and sha(path)==row['sha256'],('Full build input drift',row['path'])
  for target,row in artifacts.items():
   p=Path(row['path']);assert p.resolve().is_relative_to((d/'frozen-binaries').resolve())
   assert p.stat().st_size==row['bytes'] and sha(p)==row['sha256'],('ELF drift',target)
   with p.open('rb') as f:assert f.read(4)==b'\x7fELF'
  diff=d/'source.diff';assert sha(diff)==build['rust_diff_sha256']
  binding.update(status='BOUND_ACTUAL_ROOT_SUCCESSFUL_ORDINARY_FIVE_LIBRARY_BUILD',destination=str(d),binary_bindings=artifacts,
    receipts={n:dict(path=str(p),sha256=sha(p)) for n,p in [('source_binding',bp),('input_manifest',ip),('artifacts_manifest',ap),('source_diff',diff)]},
    full_input_count=len(inputs),all_current_inputs_match=True,all_selected_source_routes_match=True)
 if args.listings_dir:
  assert args.freeze,'No listing binding without actual successful frozen build'
  listing={}
  for target,names in sel['groups'].items():
   p=args.listings_dir/(target+'.list');rows=Counter(l[:-6] for l in p.read_text().splitlines() if l.endswith(': test'))
   assert rows and all(rows[n]==1 for n in names),('Actual list missing/multiple selected singleton',target)
   listing[target]=dict(path=str(p),sha256=sha(p),selected_singletons=len(names),artifact_sha256=binding['binary_bindings'][target]['sha256'])
  binding['current_listing_bindings']=listing
 sel['conditional_case_successor']=dict(predecessor=dict(uncompressed_sha256=BASE_SHA,rows=583,ordered_rows_verbatim=True,all_inherited_metadata_verbatim_except_summary_fields=True),
  provenance=req['bindings'],requests=requests,current_source_evidence=evidence,current_source_files=[dict(path=p,sha256=s) for p,s in sorted(files.items())],freeze_binding=binding,
  non_native_scope=req['non_native_scope'],metadata_scope='Only conditional_case_successor current_source_* and freeze_binding describe this successor; every inherited source/ELF/listing binding remains historical provenance.',
  execution='Source/manifest/hash preparation only. No build/list/test/Git/main mutation. Root alone executes canonical runner with exact current singleton listings, provider setup, 180s test and30s list budgets and artifact before/after hashes.')
 changed={'execution_order','groups','expected_group_counts','total_exact_selectors','group','successor_destination'}
 assert all(sel[k]==v for k,v in old.items() if k not in changed)
 assert all(sel[k]==old[k] for k in GUARDS)
 assert len(sel['execution_order'])==len(seen)==sum(sel['expected_group_counts'].values())
 assert all(sha(tools.ROOT/p)==s for p,s in files.items())
 summary=dict(retained=583,new_deduplicated=len(added),total=len(seen),by_target=sel['expected_group_counts'],requested_categories=dict(Counter(r['category'] for r in requests)),new_categories=dict(Counter(r['category'] for r in requests if not r['already_retained'])),source_files_and_module_routes=len(files),freeze_status=binding['status'],all583_ordered_rows_and_inherited_guards_environment_budgets_preserved=True,no_executable_run=True)
 return sel,summary

if __name__=='__main__':
 ap=argparse.ArgumentParser(description=__doc__);mode=ap.add_mutually_exclusive_group(required=True);mode.add_argument('--source-only',action='store_true');mode.add_argument('--freeze',action='store_true')
 ap.add_argument('--root',type=Path,default=Path('/workspace/tcl-lsp'));ap.add_argument('--output',type=Path,required=True);ap.add_argument('--destination',type=Path);ap.add_argument('--listings-dir',type=Path)
 args=ap.parse_args();assert not args.output.resolve().is_relative_to(args.root.resolve()),'No repository writes';assert not args.output.exists(),'Immutable output already exists; choose a new path'
 selection,summary=generate(args);args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(selection,indent=2)+'\n')
 summary.update(selection_path=str(args.output),selection_sha256=sha(args.output),generator_sha256=sha(Path(__file__)))
 sp=args.output.with_name(args.output.stem+'-summary.json');assert not sp.exists();sp.write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
