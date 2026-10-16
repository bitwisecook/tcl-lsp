"""Strict actual frozen Native5 admission; no executable invocation here."""
import copy,gzip,hashlib,json,os,shutil
from pathlib import Path
PACKAGE=Path(__file__).resolve().parent
ROOT=Path('/workspace/tcl-lsp')
RUNNER=ROOT/'experiments/name-handling/validation/retained-selections/replay-selection.py'
RUNNER_SHA='aaa269431e0a0f3024367d0a2528b4671408e11cf7a57e801caa2429f181e6c3'
PACKAGED={'requests.json':'d04327f81dbee5124ec26f4347766c6ce2b208690127e1f6ecbbc080a5131997','retained584-order.json.gz':'1db51f4e240f1b196895114f4335acf58f970520715d835bdbcbd6cc636f992d','inherited-guards.json':'8d5dc5d1be2bd9adf4ba0ee8a900254adbe6df92e78fc2f36519673a1235176d','provider-corrected-setup.json':'cf57d1f2af60b425a4f7c1cb644af1682118ff8173186d43a8f554068a483396'}
TARGETS={'tcl_compiler','tcl_lsp_core','tcl_lsp_db','tcl_registry','tcl_lsp_server'}

def sha(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def packaged():
 for n,h in PACKAGED.items():assert sha(PACKAGE/n)==h,'Preparation package drift: '+n
 return json.loads(gzip.decompress((PACKAGE/'retained584-order.json.gz').read_bytes()))

def exact_source(inputs):
 assert len(inputs)==39001 and len({r['path'] for r in inputs})==39001
 for r in inputs:
  p=ROOT/r['path'];assert p.resolve().is_relative_to(ROOT.resolve()) and p.is_file() and sha(p)==r['sha256'],'Actual frozen input drift: '+r['path']

def exact_artifacts(artifacts,build_dir):
 assert set(artifacts)==TARGETS
 for target,r in artifacts.items():
  p=Path(r['path']);assert p.resolve().is_relative_to((build_dir/'frozen-binaries').resolve())
  assert p.is_file() and p.stat().st_size==r['bytes'] and sha(p)==r['sha256'],'Actual frozen ELF drift: '+target
  with p.open('rb') as f:assert f.read(4)==b'\x7fELF','Not actual Native ELF: '+target

def environment():
 assert not [k for k,v in os.environ.items() if v and (k.startswith('TCL_LSP_TRACE') or k=='TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC')],'Instrumentation environment refused'
 required=json.loads((PACKAGE/'provider-corrected-setup.json').read_text())['environment']
 required['CARGO_PROFILE_TEST_DEBUG']='0'
 for k,v in required.items():assert os.environ.get(k)==v,'Original required provider/debug environment mismatch: '+k
 for name in ['tclsh9.0','tclsh8.6']:assert shutil.which(name)=='/workspace/.toolchains/tcl/bin/'+name,'Actual provider PATH mismatch: '+name
 assert Path(required['TCL_LSP_JIMSH']).is_file()
 return required

def validate(fullpath,build_dir):
 old=packaged();full=json.loads(fullpath.read_text());build_dir=build_dir.resolve()
 assert full['total_exact_selectors']==584 and full['execution_order']==old,'Full584 ordered rows changed'
 inherited=json.loads((PACKAGE/'inherited-guards.json').read_text())
 assert all(full[k]==v for k,v in inherited.items()),'Original environment/budgets/runtime guards changed'
 assert full['test_timeout_seconds']==180 and full['listing_timeout_seconds']==30
 assert {t:len(ns) for t,ns in full['groups'].items()}==full['expected_group_counts']
 assert {t:[r['selector'] for r in old if r['target']==t] for t in full['groups']}==full['groups']
 successor=full['conditional_case_successor'];binding=successor['freeze_binding']
 assert binding['status']=='BOUND_ACTUAL_ROOT_SUCCESSFUL_ORDINARY_FIVE_LIBRARY_BUILD','No successful actual frozen584 binding'
 assert Path(binding['destination']).resolve()==build_dir,'Explicit build directory differs from actual frozen584'
 paths={'source_binding':build_dir/'source-binding.json','input_manifest':build_dir/'input-before.json','artifacts_manifest':build_dir/'artifacts.json','source_diff':build_dir/'source.diff'}
 for k,p in paths.items():
  r=binding['receipts'][k];assert Path(r['path']).resolve()==p.resolve() and sha(p)==r['sha256'],'Actual current receipt drift: '+k
 build=json.loads(paths['source_binding'].read_text());artifacts=json.loads(paths['artifacts_manifest'].read_text());inputs=json.loads(paths['input_manifest'].read_text())
 assert build['status']=='PASS_FIVE_LIBRARY_BUILD' and build['exit_code']==0,'Actual Native5 PASS required'
 assert build['ordinary'] is True and build['instrumentation'] is False and build['single_job'] is True and build['test_debug']==0
 assert build['input_drift_count']==0 and build['input_before_count']==build['input_after_count']==len(inputs)==39001
 assert len(build['packages'])==5 and {x.replace('-','_') for x in build['packages']}==TARGETS
 assert sha(paths['source_diff'])==build['rust_diff_sha256'] and binding['binary_bindings']==artifacts
 assert binding['full_input_count']==39001 and binding['all_current_inputs_match'] and binding['all_selected_source_routes_match']
 exact_source(inputs);exact_artifacts(artifacts,build_dir)
 frozen={r['path']:r['sha256'] for r in inputs}
 for row in successor['current_source_files']:assert frozen[row['path']]==row['sha256']
 evidence=successor['current_source_evidence'];assert len(evidence)==584
 assert {(r['target'],r['selector']) for r in evidence}=={(r['target'],r['selector']) for r in old}
 for row in evidence:
  assert row['current_native_linux_source_eligible'] is True
  for item in [row]+row['module_route']:assert frozen[item['source_path']]==item['source_sha256']
 assert sha(RUNNER)==RUNNER_SHA,'Canonical runner drift'
 return full,binding,inputs,artifacts

def subset_rows(full):
 request=json.loads((PACKAGE/'requests.json').read_text());order=full['execution_order'];lookup={(r['target'],r['selector']):r for r in order}
 rows=[copy.deepcopy(order[i-1]) for i in request['original_ordinals']]
 rows += [copy.deepcopy(lookup[tuple(key)]) for key in request['extra_selectors']]
 assert len(rows)==len({(r['target'],r['selector']) for r in rows})==12
 return rows
