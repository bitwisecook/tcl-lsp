from pathlib import Path
import json,difflib,textwrap,hashlib,copy,re
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-proof-semantic-audit-20261014');rel='docs/design/analysis/name-resolution-proofs/manifest.json';original=(root/rel).read_text();selected={r['id']:r for r in json.loads((out/'selected-proof-review.json').read_text())};changes=[];chunks=[];patch='--- a/'+rel+'\n+++ b/'+rel+'\n';line_delta=0
for ident in ['naming.consumer.original-source-attachment-candidates','naming.source.original-registry-header-advice']:
 old=selected[ident];new=copy.deepcopy(old);doc=(root/old['document']).read_text();new['conclusion']=doc.split('## Conclusion',1)[1].split('\n## ',1)[0].strip()
 if ident=='naming.source.original-registry-header-advice':
  for selector,assertion in [('original_source_schema_keeps_named_selector_and_handler_argument_shape','Original readonly source metadata preserves the named selector and written argument origins independently of private handler argv; a changed whole source image declines. Definition inspected; no test execution result is supplied by this binding.'),('original_source_schema_withholds_dynamic_and_shadowed_selector_claims','Dynamic selection cannot supply wrapping metadata and a source-shadowed selector declines. Definition inspected; no test execution result is supplied by this binding.')]:new['tests'].append({'path':'rust/tcl-compiler/src/registry_invocation/source_structure.rs','selector':'registry_invocation::source_structure::tests::'+selector,'assertion':assertion,'status':'linked'})
 needle=textwrap.indent(json.dumps(old,indent=2,ensure_ascii=False),'    ');replace=textwrap.indent(json.dumps(new,indent=2,ensure_ascii=False),'    ')
 if original.count(needle)!=1:
  needle=textwrap.indent(json.dumps(old,indent=2),'    ');replace=textwrap.indent(json.dumps(new,indent=2),'    ')
 assert original.count(needle)==1,ident
 offset=original[:original.index(needle)].count('\n');a=needle.splitlines(keepends=True);b=replace.splitlines(keepends=True)
 for line in list(difflib.unified_diff(a,b,n=3))[2:]:
  if line.startswith('@@'):
   m=re.match(r'@@ -(\d+)(,\d+)? \+(\d+)(,\d+)? @@(.*)',line);line=f'@@ -{int(m[1])+offset}{m[2] or ""} +{int(m[3])+offset+line_delta}{m[4] or ""} @@{m[5]}\n'
  patch+=line
 line_delta+=len(b)-len(a);changes.append({'id':ident,'fields':['conclusion']+(['tests'] if 'header' in ident else []),'evidence_and_provider_fields_unchanged':old['evidence']==new['evidence'] and old['providers']==new['providers']})
for ident in ['naming.numeric.rust-wasi-libc-environment','naming.numeric.wasi-libc-environment']:
 r=selected[ident];path=r['document'];old=(root/path).read_text();oldscope=old.split('## Scope',1)[1].split('\n## ',1)[0].strip();assert ' '.join(r['scope'].split()).startswith(' '.join(oldscope.split()));new=old.replace(oldscope,r['scope'],1);assert r['scope'] in new;patch+=''.join(difflib.unified_diff(old.splitlines(keepends=True),new.splitlines(keepends=True),fromfile='a/'+path,tofile='b/'+path));(out/Path(path).name).write_text(new)
(out/'bounded-proof-bindings.patch').write_text(patch);(out/'bounded-proof-bindings-manifest.json').write_text(json.dumps({'base_manifest_sha256':hashlib.sha256(original.encode()).hexdigest(),'patch_sha256':hashlib.sha256(patch.encode()).hexdigest(),'record_changes':changes,'scope':'Current authored contract conclusions and real test-definition bindings only; restore2 exact retained redaction boundaries. No provider observation, evidence digest, implementation receipt, assertion, source implementation, oracle seal or census digest renewed. Current Rust assertion acceptance remains pending.'},indent=2)+'\n');print(json.dumps({'patch_bytes':len(patch.encode()),'patch_sha256':hashlib.sha256(patch.encode()).hexdigest(),'record_changes':changes},indent=2))
