from pathlib import Path
import hashlib,json,sys,collections,re
O=Path('/tmp/lsp-next-rust-union-v2');R=Path('/workspace/tcl-lsp');items=json.loads((O/'ingredients.json').read_text())
sys.path.insert(0,'/tmp/lsp-server-document-path-admission/parser-tools-025')
from tree_sitter import Parser,Language
import tree_sitter_rust
parser=Parser(Language(tree_sitter_rust.language()))
def sha(b):return hashlib.sha256(b).hexdigest()
def inventory(path):
 if not path.exists():return {}
 data=path.read_bytes();t=parser.parse(data);assert not t.root_node.has_error,str(path)
 out={}
 def prior_attrs(n):
  a=[];c=n.prev_named_sibling
  while c and c.type=='attribute_item':a.append(data[c.start_byte:c.end_byte]);c=c.prev_named_sibling
  return b' '.join(a)
 def visit(n,parts=(),test=False):
  attrs=prior_attrs(n)
  if n.type=='mod_item':
   name=n.child_by_field_name('name');name=data[name.start_byte:name.end_byte].decode();parts=parts+('mod:'+name,);test=test or b'cfg(test)' in attrs or name.endswith('tests') or name=='tests'
  if n.type=='impl_item':
   ty=n.child_by_field_name('type');tr=n.child_by_field_name('trait');owner=data[ty.start_byte:ty.end_byte].decode() if ty else '<unknown>';trait=data[tr.start_byte:tr.end_byte].decode() if tr else '';parts=parts+('impl:'+trait+':'+owner,)
  if n.type=='function_item':
   name=n.child_by_field_name('name');key='::'.join(parts+('fn:'+data[name.start_byte:name.end_byte].decode(),))
   assert key not in out,(path,key)
   out[key]={'bytes':data[n.start_byte:n.end_byte],'test':test or bool(re.search(rb'#\[(?:\w+::)*test(?:\]|\()',attrs))}
   parts=parts+('fn:'+data[name.start_byte:name.end_byte].decode(),)
  for c in n.children:visit(c,parts,test)
 visit(t.root_node);return out
paths=sorted({f['path'] for it in items for f in it['files']});cache={}
def inv(path):
 key=str(path)
 if key not in cache:cache[key]=inventory(path)
 return cache[key]
functions=[];modified=[];newcontrols=[];basechangedtests=[];oldtestcount=0;oldtestequal=0;joint=[]
for p in paths:
 if not p.endswith('.rs'):continue
 b=inv(O/'base'/p);c=inv(O/'edit'/p)
 for key,v in b.items():
  if v['test']:
   oldtestcount+=1
   if key in c and c[key]['bytes']==v['bytes']:oldtestequal+=1
   else:basechangedtests.append({'path':p,'key':key,'before_sha256':sha(v['bytes']),'after_sha256':sha(c[key]['bytes']) if key in c else None})
 for it in items:
  if p not in {f['path'] for f in it['files']}:continue
  a=inv(Path(it['directory'])/it.get('layout','edit')/p)
  for key,v in a.items():
   changed=key not in b or b[key]['bytes']!=v['bytes']
   if changed:
    assert key in c,(it['name'],p,key,'missing reviewed changed/new function')
    if c[key]['bytes']!=v['bytes']:joint.append({'ingredient':it['name'],'path':p,'function':key,'ingredient_sha256':sha(v['bytes']),'combined_sha256':sha(c[key]['bytes'])})
    modified.append({'ingredient':it['name'],'path':p,'function':key,'sha256':sha(v['bytes']),'new':key not in b})
    if key not in b and v['test']:newcontrols.append({'ingredient':it['name'],'path':p,'function':key,'sha256':sha(v['bytes'])})
 for key,v in c.items():
  if key in b and b[key]['bytes']!=v['bytes']:
   origins=[x['ingredient'] for x in modified if x['path']==p and x['function']==key]
   assert origins,(p,key,'unreviewed function change')
 functions.extend({'path':p,'function':key,'test_context':v['test'],'sha256':sha(v['bytes'])} for key,v in c.items())
# Only existing synthetic Word constructors acquire required None fields, exactly
# as reviewed in 565V2. Every such changed function must match that ingredient.
# Any intentional existing test-helper/fixture edit must preserve every
# original assertion macro byte, independently of the ingredient's claim.
def assertions(data):
 tree=parser.parse(data);out=[];stack=[tree.root_node]
 while stack:
  n=stack.pop()
  if n.type=='macro_invocation':
   text=data[n.start_byte:n.end_byte]
   if re.match(rb'(?:[A-Za-z_][A-Za-z0-9_]*::)*assert(?:_eq|_ne)?!',text):out.append(text)
  stack.extend(reversed(n.children))
 return out
for x in basechangedtests:
 before=inv(O/'base'/x['path'])[x['key']]['bytes'];after=inv(O/'edit'/x['path'])[x['key']]['bytes']
 assert assertions(before)==assertions(after),(x,'original assertion changed')
# Removing one unused import changes no function bytes.
p='rust/tcl-compiler/src/analyser/types/logical_members.rs';expected=Path('/tmp/lsp-logical-call-wire-v2/edit')/p
needle=b'mod logical_relation_navigation_tests {\n    use super::*;\n'
assert expected.read_bytes().count(needle)==1
assert (O/'edit'/p).read_bytes()==expected.read_bytes().replace(needle,b'mod logical_relation_navigation_tests {\n')
print(json.dumps({'function_joint_compositions':joint,'old_test_context_functions':oldtestcount,'old_test_changes':basechangedtests},indent=2),flush=True)
# Reviewed docs additions/replacements survive exactly, except documented Root grammar cleanup.
for it in items:
 for f in it['files']:
  p=f['path']
  if not p.endswith('.md'):continue
  before=((O/'base'/p).read_text().splitlines(True) if (O/'base'/p).exists() else []);after=(Path(it['directory'])/it.get('layout','edit')/p).read_text().splitlines(True);combined=(O/'edit'/p).read_text()
  import difflib
  for tag,a,b,c,d in difflib.SequenceMatcher(None,before,after,autojunk=False).get_opcodes():
   if tag!='equal' and c<d:
    permitted_grammar={
     'existing locked getrandom 0.2.17\n',
     'native/WASI/browser issuer is used directly, with no fallback to fabricated\n',
     'time, source hashes, process-local reset counters or absent-owner equality.\n',
    }
    for line in after[c:d]:
     if line.strip() and line not in combined:
      assert p=='docs/design/contracts/name-handling.md' and line in permitted_grammar,(it['name'],p,'lost reviewed documentation line',line)
report={'all_rust_files_parse':True,'rust_files':sum(p.endswith('.rs') for p in paths),'every_reviewed_changed_or_new_function_byte_exact':not joint,'joint_function_compositions':joint,'reviewed_modified_new_function_count':len(modified),'old_test_context_function_count':oldtestcount,'old_test_context_functions_byte_equal':oldtestequal,'existing_test_function_changes':basechangedtests,'existing_test_changes_scope':'Only exact reviewed fixture/helper changes; original assertion macro bytes independently identical','new_test_context_function_count':len(newcontrols),'unused_import_cleanup_exact':'logical_members::logical_relation_navigation_tests only; all function bytes equal CALLV2','all_reviewed_documentation_changes_present':True,'execution':'none: static AST/source retention only'}
(O/'retention-verification.json').write_text(json.dumps(report,indent=2)+'\n');(O/'reviewed-function-hashes.json').write_text(json.dumps(modified,indent=2)+'\n');(O/'new-control-function-hashes.json').write_text(json.dumps(newcontrols,indent=2)+'\n');(O/'all-function-hashes.json').write_text(json.dumps(functions,indent=2)+'\n');print(json.dumps(report,indent=2))
