from pathlib import Path
import difflib,json,hashlib
out=Path('/tmp/name-variable-protection-owner-20261014');root=Path('/workspace/tcl-lsp');patch='';rows=[]
for proposed in sorted((out/'proposal').rglob('*')):
 if not proposed.is_file():continue
 rel=str(proposed.relative_to(out/'proposal'));base=out/'base'/rel;before=base.read_text() if base.exists() else '';after=proposed.read_text()
 if before==after:continue
 patch+=''.join(difflib.unified_diff(before.splitlines(keepends=True),after.splitlines(keepends=True),fromfile='a/'+rel if base.exists() else '/dev/null',tofile='b/'+rel));rows.append({'path':rel,'before_sha256':hashlib.sha256(before.encode()).hexdigest() if base.exists() else None,'after_sha256':hashlib.sha256(after.encode()).hexdigest(),'before_matches_current_Main':base.exists() and (root/rel).is_file() and base.read_bytes()==(root/rel).read_bytes()})
(out/'implementation.patch').write_text(patch);(out/'implementation-manifest.json').write_text(json.dumps({'scope':'Isolated source/contract migration. No Cargo/Rust tests executed. Actual C5/Jim literal-name observations retained separately. Unknown and entered-Native guards unchanged; no oracle or proof-index digests renewed. Empty literal observer/write names now retain conservative protection.','patch_sha256':hashlib.sha256(patch.encode()).hexdigest(),'files':rows},indent=2)+'\n');print(json.dumps({'patch_bytes':len(patch.encode()),'files':len(rows),'patch_sha256':hashlib.sha256(patch.encode()).hexdigest(),'Main_drift':[x['path'] for x in rows if x['before_sha256'] is not None and not x['before_matches_current_Main']]},indent=2))
