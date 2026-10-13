from pathlib import Path
import hashlib,json,difflib,re
out=Path('/tmp/name-inline-fixtures-main-20261013');paths=['rust/tcl-compiler/src/interprocedural.rs','rust/tcl-compiler/src/optimiser/propagation.rs','rust/tcl-compiler/src/test_body_identity.rs'];patch=[];records=[];selectors=[]
for rel in paths:
 before=out/'before'/rel;old=before.read_text() if before.exists() else '';new=(out/'after'/rel).read_text()
 if before.exists():
  marker='#[cfg(test)]\nmod tests {';i=old.index(marker);newprefix=new[:new.index(marker)]
  extra='#[cfg(test)]\n#[path = "test_body_identity.rs"]\npub(crate) mod test_body_identity;\n\n' if rel.endswith('/interprocedural.rs') else ''
  assert newprefix==old[:i]+extra,rel+' production prefix changed'
  originalselectors=re.findall(r'#\[test\]\s*(?:#\[[^\n]+\]\s*)*fn\s+(\w+)',old[i:])
  afterselectors=re.findall(r'#\[test\]\s*(?:#\[[^\n]+\]\s*)*fn\s+(\w+)',new[new.index(marker):])
  assert originalselectors==afterselectors,rel+' changed test roster'
  selectors.extend({'file':rel,'selector':s,'executed':False} for s in originalselectors)
 patch.append('diff --git a/'+rel+' b/'+rel+'\n')
 if not before.exists():patch.append('new file mode 100644\n')
 patch.extend(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+rel if before.exists() else '/dev/null',tofile='b/'+rel))
 records.append({'path':rel,'before_sha256':hashlib.sha256(old.encode()).hexdigest() if before.exists() else None,'after_sha256':hashlib.sha256(new.encode()).hexdigest(),'production_prefix_unchanged':True,'new_file':not before.exists()})
text=''.join(patch);(out/'test-only.patch').write_text(text);(out/'manifest.json').write_text(json.dumps({'files':records,'patch_sha256':hashlib.sha256(text.encode()).hexdigest(),'status':'test-only source adaptations; uncompiled and unexecuted','parent_roster':'/tmp/name-body-id-test-fixture-index-counts.json','assertions':'Original programs, expected assertion labels/effects and original selectors retained. Edges followed by exact ID and displayed only in fixture assertions; method/provider negative graph IDs use their original actual owner.'},indent=2));(out/'selectors.json').write_text(json.dumps(selectors,indent=2));print('patch',len(text),hashlib.sha256(text.encode()).hexdigest(),'selectors',len(selectors))
