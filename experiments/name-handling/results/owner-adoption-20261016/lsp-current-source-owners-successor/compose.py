from pathlib import Path
import json,hashlib,difflib,collections,shutil
R=Path('/workspace/tcl-lsp');O=Path('/tmp/lsp-next-rust-union-v2')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
items=[
('loader565','/tmp/lsp-loader-notice-original-subject-v2','manifest.json','repair.patch','b18ca854','independent-project-review-CLEAR.txt','8fe3b6d4','edit'),
('namespace','/tmp/lsp-conditional-namespace-advice-v3','manifest.json','conditional-namespace-advice.patch','3c4e6734','independent-sol-compiler-review-CLEAR.txt','b29a458a','edit'),
('call642_v3','/tmp/lsp-logical-call-wire-v3','manifest.json','logical-call-wire.57d5aaf4dd3b97e67dfe02372bd8da3ee2c863047f00312e871e55d0ddd0764d.patch','57d5aaf4','independent-db-owner-review-CLEAR.txt','851b0d17','edit'),
('regression128','/tmp/lsp-original-call-external-source-repair-v1','manifest.json','repair.patch','a38f85e3','independent-sol-compiler-review-CLEAR.txt','83ba4826','edit'),
('native_presentation_v2','/workspace/naming-validation/agent-work/native-lens-presentation-v2','manifest.json','native-lens-presentation-v2.patch','b041ff7a','independent-db-owner-native-v2-review-CLEAR.txt','22a198c4','edit'),
('case725','/workspace/naming-validation/agent-work/case725-fixture-v1','manifest.json','repair.patch','6a843229','independent-db-owner-review-CLEAR.txt','0d7e20e7','after'),
('class_source','/workspace/naming-validation/agent-work/logical-class-definition-source-v1','manifest.json','class-definition-source.patch','9538ce53','independent-root-review-CLEAR.txt','055ea15a','edit'),
('transform_v3','/tmp/lsp-document-transform-currency-v3','manifest.json','repair.patch','d199ae0e','independent-root-review-CLEAR.txt','1968b93d','edit'),
('signature','/tmp/lsp-signature-original-query-v1','manifest.json','signature-original-query.1a0d86672b130dfa0fe460668dae4006d56e0fcd6b9c9b4d55cf73470b563a33.patch','1a0d8667','independent-sol-compiler-review-CLEAR.txt','6ba9e77b','edit'),
('math_document_scope','/workspace/naming-validation/agent-work/logical-math-document-scope-v1','manifest.json','logical-math-document-scope-v1.patch','6bf25c82','independent-sol-compiler-review-CLEAR.txt','958af8d4','edit'),
]
metadata=[];byfile=collections.defaultdict(list)
for name,d,m,p,expected,review,reviewsha,layout in items:
 d=Path(d);j=json.loads((d/m).read_text());assert sha(d/p)==j['patch_sha256'] and sha(d/p).startswith(expected);assert sha(d/review).startswith(reviewsha),(name,sha(d/review))
 it={'name':name,'directory':str(d),'layout':layout,'manifest':str(d/m),'manifest_sha256':sha(d/m),'patch':str(d/p),'patch_sha256':sha(d/p),'independent_review':str(d/review),'independent_review_sha256':sha(d/review),'files':j['files'],'layer':'actual-current-main'}
 if name=='transform_v3':it['host_review']=str(d/'independent-host-review-CLEAR.txt');it['host_review_sha256']=sha(d/'independent-host-review-CLEAR.txt');assert it['host_review_sha256'].startswith('9226b4ec')
 for f in j['files']:
  path=f['path'];q=R/path;assert (sha(q) if q.exists() else None)==f['before_sha256'],(name,path,'current basis differs');assert sha(d/layout/path)==f['after_sha256']
  b=d/'base'/path
  if b.exists():assert sha(b)==f['before_sha256']
  byfile[path].append((it,f))
 metadata.append(it)
records=[]
for path,variants in sorted(byfile.items()):
 for part in ['base','edit']:
  q=O/part/path;q.parent.mkdir(parents=True,exist_ok=True)
  if (R/path).exists():shutil.copyfile(R/path,q)
 before=O/'base'/path;target=O/'edit'/path
 if not before.exists():
  assert len(variants)==1;(target).write_bytes((Path(variants[0][0]['directory'])/variants[0][0]['layout']/path).read_bytes());continue
 base=before.read_text().splitlines(True);changes=[]
 for number,(it,f) in enumerate(variants):
  after=(Path(it['directory'])/it['layout']/path).read_text().splitlines(True)
  for tag,a,b,c,d in difflib.SequenceMatcher(None,base,after,autojunk=False).get_opcodes():
   if tag!='equal':changes.append((a,b,number,after[c:d],it['name']))
 changes.sort(key=lambda v:(v[0],v[1],v[2]));result=[];cur=0;prev=None
 for a,b,order,text,name in changes:
  if prev and (a,b,text)==prev:continue
  assert a>=cur,(path,'overlapping replacements',a,b,cur,name)
  result+=base[cur:a]+text;cur=b;prev=(a,b,text)
 result+=base[cur:];target.write_text(''.join(result));records.append({'path':path,'layer':'independent current-main','changes':[{'start':a,'end':b,'new_lines':len(text),'new_text_sha256':hashlib.sha256(''.join(text).encode()).hexdigest(),'ingredient':name} for a,b,_,text,name in changes]})
# LensV3 is a separately checked dependent layer over actual CALLV3+NativeV2.
d=Path('/workspace/naming-validation/agent-work/lsp-logical-lens-wire-v3');j=json.loads((d/'manifest.json').read_text());assert sha(d/'logical-lens-wire-v3.patch')==j['patch_sha256']=='52a0650dfe1e63d10726c0499d12f50fdbd903ce05dcfbbc7786d107e2476462';review=d/'independent-db-owner-review-CLEAR.txt';assert sha(review)=='6d691999f98354aba6a1ddc39644426485b119538a5d60bfcf665540b078cbfd'
it={'name':'lens_v3','directory':str(d),'layout':'edit','manifest':str(d/'manifest.json'),'manifest_sha256':sha(d/'manifest.json'),'patch':str(d/'logical-lens-wire-v3.patch'),'patch_sha256':j['patch_sha256'],'independent_review':str(review),'independent_review_sha256':sha(review),'files':j['files'],'layer':'exact CALLV3+NativeV2 dependency; retain all other already-composed nodes'}
for f in j['files']:
 path=f['path'];target=O/'edit'/path;author_before=d/'base'/path;author_after=d/'edit'/path;assert sha(author_after)==f['after_sha256']
 if f['before_sha256'] is None:
  assert not author_before.exists() and not (R/path).exists() and not target.exists();target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(author_after.read_bytes());continue
 assert sha(author_before)==f['before_sha256'];assert target.exists()
 base=author_before.read_text().splitlines(True);changes=[]
 for number,(variant,name) in enumerate([(target,'approved_current_union'),(author_after,'lens_v3')]):
  lines=variant.read_text().splitlines(True)
  for tag,a,b,c,e in difflib.SequenceMatcher(None,base,lines,autojunk=False).get_opcodes():
   if tag!='equal':changes.append((a,b,number,lines[c:e],name))
 changes.sort(key=lambda v:(v[0],v[1],v[2]));result=[];cur=0;prev=None
 for a,b,order,text,name in changes:
  if prev and (a,b,text)==prev:continue
  assert a>=cur,(path,'dependent overlapping replacements',a,b,cur,name)
  result+=base[cur:a]+text;cur=b;prev=(a,b,text)
 result+=base[cur:];target.write_text(''.join(result));records.append({'path':path,'layer':'lens dependent common-before','changes':[{'start':a,'end':b,'new_lines':len(text),'new_text_sha256':hashlib.sha256(''.join(text).encode()).hexdigest(),'ingredient':name} for a,b,_,text,name in changes]})
metadata.append(it)
# Exactly Root-authorized source cleanup.
p='rust/tcl-compiler/src/analyser/types/logical_members.rs';q=O/'edit'/p;s=q.read_text();a='mod logical_relation_navigation_tests {\n    use super::*;\n';b='mod logical_relation_navigation_tests {\n';assert s.count(a)==1;q.write_text(s.replace(a,b))
# Exactly Root-authorized grammar and one extra blank-line cleanup, no source contract change.
p='docs/design/contracts/name-handling.md';q=O/'edit'/p;s=q.read_text();a='''The shared sealed `wire_session::EditorWireSession::allocate` owner uses the
existing locked getrandom 0.2.17
native/WASI/browser issuer is used directly, with no fallback to fabricated
time, source hashes, process-local reset counters or absent-owner equality.''';b='''The shared sealed `wire_session::EditorWireSession::allocate` owner uses the
existing locked getrandom 0.2.17 Native/WASI/browser issuer directly, without a
fallback to time, source hashes, process-local reset counters or absent-owner
equality.''';assert s.count(a)==1;s=s.replace(a,b);a='\n\n\nCALL schema 3';b='\n\nCALL schema 3';assert s.count(a)==1;q.write_text(s.replace(a,b))
(O/'ingredients.json').write_text(json.dumps(metadata,indent=2)+'\n');(O/'exact-composition.json').write_text(json.dumps(records,indent=2)+'\n');(O/'authorized-cleanups.json').write_text(json.dumps({'source':'one unused super::* in logical_relation_navigation_tests only','documentation':'Root authorized final CALLV3 getrandom sentence grammar plus one extra blank line; exact old/new text in compose.py','other_unreviewed_changes':False},indent=2)+'\n')
print(json.dumps({'ingredients':len(metadata),'paths':len(set(byfile)|{f['path'] for f in j['files']}),'overlapping_replacements':0},indent=2))
