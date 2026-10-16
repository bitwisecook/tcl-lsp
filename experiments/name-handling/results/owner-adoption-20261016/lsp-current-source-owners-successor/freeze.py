from pathlib import Path
import hashlib,json,difflib,subprocess,shutil
O=Path('/tmp/lsp-next-rust-union-v2');R=Path('/workspace/tcl-lsp')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
items=json.loads((O/'ingredients.json').read_text());paths=sorted({f['path'] for it in items for f in it['files']});parts=[];files=[]
for p in paths:
 b=O/'base'/p;a=O/'edit'/p;current=R/p
 assert current.exists()==b.exists(),p
 if b.exists():assert sha(b)==sha(current),p
 before=b.read_text().splitlines(True) if b.exists() else [];after=a.read_text().splitlines(True)
 assert before!=after,p
 parts.extend(difflib.unified_diff(before,after,fromfile='a/'+p if b.exists() else '/dev/null',tofile='b/'+p,n=3))
 files.append({'path':p,'before_sha256':sha(b) if b.exists() else None,'after_sha256':sha(a),'new':not b.exists(),'ingredients':[it['name'] for it in items if p in {x['path'] for x in it['files']}]})
patch=O/'next-rust-union.patch';patch.write_text(''.join(parts))
S=O/'fresh-apply';shutil.copytree(O/'base',S,dirs_exist_ok=True)
r=subprocess.run(['patch','--batch','--fuzz=0','-p1','-i',str(patch)],cwd=S,capture_output=True,text=True);assert r.returncode==0,r.stdout+r.stderr
assert 'offset' not in r.stdout and 'fuzz' not in r.stdout
assert all(sha(S/f['path'])==f['after_sha256'] for f in files)
ri=subprocess.run(['patch','--batch','--fuzz=0','--remove-empty-files','-R','-p1','-i',str(patch)],cwd=S,capture_output=True,text=True);assert ri.returncode==0,ri.stdout+ri.stderr
assert all((sha(S/f['path'])==f['before_sha256'] if f['before_sha256'] else not (S/f['path']).exists()) for f in files)
stat={'zero_fuzz_no_offset_forward':True,'all_after_hashes':True,'zero_fuzz_inverse':True,'all_before_hashes_and_new_file_absence_restored':True,'current_touched_beforeimages_equal':True,'full39002_current_index':'not copied or newly issued; Root owns actual full snapshot binding','forward_output':r.stdout,'inverse_output':ri.stdout,'no_build_test_list_trace_gate_or_git':True}
(O/'patch-verification.json').write_text(json.dumps(stat,indent=2)+'\n')
cleanup={'path':'rust/tcl-compiler/src/analyser/types/logical_members.rs','module':'logical_relation_navigation_tests','change':'remove exactly one unused use super::*; existing module only','authorization':'Root explicit bounded cleanup; observed warning in actual build513','other_imports_or_function_bytes_changed':False}
excluded=[{'patch_sha256':'85cedc999afea9039c68a1d6634a71e51deef8d48b3a557022f5228175009253','disposition':'HOLD/superseded by independently reviewed b041ff7a, not admitted','reason':'?-only reporting projection drops genuine own-object method lens identity'}, {'purpose':'TransformV1/V2 prior session-unclear packets','disposition':'HOLD/superseded; reviewed TransformV3 only included'}, {'purpose':'held LensV1/V2 and old CALLV2/29union0731','disposition':'HOLD/superseded; only reviewed CALLV3 and LensV3 admitted as ingredients'}, {'purpose':'Workspacecache387ec910 and incomplete workspace currency successor','disposition':'HOLD/excluded'}]
manifest={'status':'immutable source-only integration proposal, Root final review/admission pending','patch':'next-rust-union.patch','patch_sha256':sha(patch),'patch_bytes':patch.stat().st_size,'files':files,'file_count':len(files),'rust_files':sum(f['path'].endswith('.rs') for f in files),'ingredients':[{'name':it['name'],'patch':it['patch'],'patch_sha256':it['patch_sha256'],'manifest':it['manifest'],'manifest_sha256':it['manifest_sha256'],'independent_review':it['independent_review'],'independent_review_sha256':it['independent_review_sha256']} for it in items],'cleanup':cleanup,'excluded':excluded,'composition':'Exact current-common-base line edit union; all same-position independent insertions retained in ingredient order; no overlapping replacement edits. All reviewed source nodes retained; the only joint function code_lenses equals reviewed LensV3 and Native branch equals NativeV2 except required None field, proven in joint-function-resolution.json. No semantic guard merge.','verification_receipts':{p:sha(O/p) for p in ['exact-composition.json','authorized-cleanups.json','joint-function-resolution.json','retention-verification.json','reviewed-function-hashes.json','new-control-function-hashes.json','all-function-hashes.json','patch-verification.json']},'validation':'Static source/hash/AST/retention and exact temporary patch application only. No Cargo/build/tests/listing/tracing/gates, source acceptance, runtime efficacy, performance, full input/ELF binding, Git/main edits or counts claimed.'}
(O/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps({'patch':str(patch),'patch_sha256':sha(patch),'manifest_sha256':sha(O/'manifest.json'),'files':len(files),'rust_files':manifest['rust_files'],'patch_bytes':patch.stat().st_size},indent=2))
