from pathlib import Path
import json,hashlib,subprocess,shutil,runpy
helpers=runpy.run_path('/tmp/name-comment-attribute-restoration-20261014/lexical_audit.py');rust_tokens=helpers['rust_tokens'];out=Path('/tmp/name-earlier-doc-lexical-audit-20261014');results=[]
for label,path in [('first23','/tmp/name-doc-current-owner-cleanup-20261014'),('followon10','/tmp/name-doc-current-state-followon-20261014')]:
 delivery=Path(path);manifest=json.loads((delivery/'manifest.json').read_text());work=out/label
 if work.exists():shutil.rmtree(work)
 shutil.copytree(delivery/'files',work)
 subprocess.run(['git','apply','--reverse','--unsafe-paths',str(delivery/'implementation.patch')],cwd=work,check=True)
 rows=[]
 for f in manifest['files']:
  p=f['path'];before=(work/p).read_text();after=(delivery/'files'/p).read_text()
  assert hashlib.sha256(before.encode()).hexdigest()==f['before_sha256'],(label,p,'preimage hash mismatch')
  assert hashlib.sha256(after.encode()).hexdigest()==f['after_sha256'],(label,p,'postimage hash mismatch')
  if p.endswith('.rs'):
   a,b=rust_tokens(before),rust_tokens(after);assert a==b,(label,p,'Rust token difference')
   rows.append({'path':p,'meaningful_token_count':len(a),'before_tokens_sha256':hashlib.sha256(repr(a).encode()).hexdigest(),'after_tokens_sha256':hashlib.sha256(repr(b).encode()).hexdigest(),'equal':True})
 results.append({'delivery':label,'patch_sha256':hashlib.sha256((delivery/'implementation.patch').read_bytes()).hexdigest(),'all_manifest_source_hashes_match':True,'rust_files':rows})
 print(label,'Rust files',len(rows),'tokens',sum(r['meaningful_token_count'] for r in rows),'all meaningful tokens unchanged')
(out/'lexical-audit.json').write_text(json.dumps({'all_before_after_bytes_reconstructed_from_exact_reverse_patch':True,'Rust_lexer':'Pygments RustLexer with explicit Rust-attribute/string controls, doc and ordinary comments removed only','deliveries':results},indent=2)+'\n')
