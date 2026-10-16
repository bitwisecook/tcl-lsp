from pathlib import Path
import json,hashlib,re,collections
r=Path('/workspace/tcl-lsp');out=Path('/workspace/naming-validation/lsp-source-path-diagnostics-repair')
sources=['/workspace/naming-validation/lsp-reviewed-82e016-retry4/selection.json','/workspace/naming-validation/lsp-libs-a49f3/retained-selection.json','/workspace/naming-validation/lsp-libs-a49f3/new10-selection.json','/workspace/naming-validation/lsp-libs-a49f3/delta-selection.json','/tmp/compiler-standalone-spectcl-replay-delta-v1/selection.json']
base=json.load(open(sources[0]));order=[];seen=set();groups=collections.defaultdict(list);bindings=[]
def add(target,selector,path=None,line=None,purpose='Exact retained source control; preserve original selector/assertions/budget'):
 if target=='tcl_spectcl':return
 if (target,selector) in seen:return
 seen.add((target,selector));groups[target].append(selector)
 row={'target':target,'selector':selector,'purpose':purpose}
 if path:
  row.update(source_path=path,source_line=line,source_sha256=hashlib.sha256((r/path).read_bytes()).hexdigest())
 order.append(row)
for name in sources:
 p=Path(name);v=json.loads(p.read_text());bindings.append({'path':name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'count':v['total_exact_selectors']})
 if 'execution_order' in v:
  for row in v['execution_order']:add(row['target'],row['selector'])
 else:
  for t,names in v['groups'].items():
   for n in names:add(t,n)
for t,path,prefix in [('tcl_lsp_server','rust/tcl-lsp-server/src/original_document_path_admission_tests.rs','original_document_path_admission_tests::'),('tcl_lsp_server','rust/tcl-lsp-server/src/current_diagnostics.rs','current_diagnostics::tests::'),('tcl_compiler','rust/tcl-compiler/src/parsing/spectcl_source.rs','parsing::spectcl_source::tests::')]:
 text=(r/path).read_text()
 for m in re.finditer(r'#\[(?:test|tokio::test(?:\([^\n]*\))?)\]\s*(?:async\s+)?fn\s+(\w+)\s*\(',text):
  name=m.group(1)
  if name=='actual_windows_scan_path_keeps_drive_and_verbatim_spelling_separate_from_uri':continue
  add(t,prefix+name,path,text.count('\n',0,m.start())+1,'Actual provider/source/currentness/absence/ABA controls; no Native or host acceptance beyond reached target')
path='rust/tcl-lsp-core/src/vfs.rs';text=(r/path).read_text()
for module in ['document_uri_path_tests','actual_source_path_origin_tests']:
 start=text.index('mod '+module+' {');end=text.find('\n#[cfg(test)]',start+1);end=len(text) if end<0 else end
 for m in re.finditer(r'#\[test\]\s*fn\s+(\w+)\s*\(',text[start:end]):
  name=m.group(1)
  if 'wasi' in name:continue
  add('tcl_lsp_core','vfs::'+module+'::'+name,path,text.count('\n',0,start+m.start())+1,'Actual Native/Virtual SourceStore ownership, aliases/generations and publication lease')
for name in ['auto_qualify_differential_against_tclsh','parse_pkg_index_differential_against_pkg_mkindex','parse_tcl_index_differential_against_auto_mkindex']:
 add('tcl_lsp_core','package_resolver::tests::'+name,'rust/tcl-lsp-core/src/package_resolver/tests.rs',None,'Retained original differential with mandatory original pinned9.0/8.6 provider lanes, genuine complete raw process result')
v={**base,'group':'Current input/fixture/editor source batch: union retained exact controls + actual SourcePath/diagnostic/Standalone/SpecTcl source-owner controls','groups':dict(groups),'execution_order':order,'total_exact_selectors':len(order),'expected_group_counts':{k:len(v) for k,v in groups.items()},'predecessor_bindings':bindings,'host_scope':'Actual Linux native binary only. One Windows SourcePath and two real WASI controls remain unexecuted; no silent singleton removal. Three SpecTcl loader controls require a separate ordinary loader artifact and are not included.','source_evidence_note':'Exact new selector source positions/hashes and retained selection provenance. Each selector must list exactly once before run; old assertions/budgets unchanged.'}
v['environment']['set']['TCL_LSP_REQUIRE_TCLSH']='9.0,8.6'
(out/'selection.json').write_text(json.dumps(v,indent=2)+'\n')
print(json.dumps({'count':len(order),'groups':v['expected_group_counts'],'sha256':hashlib.sha256((out/'selection.json').read_bytes()).hexdigest()}))
