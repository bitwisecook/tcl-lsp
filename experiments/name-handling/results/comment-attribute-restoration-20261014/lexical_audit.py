from pathlib import Path
import difflib,json,subprocess,hashlib
from pygments.lexers import RustLexer
from pygments.token import Comment,Whitespace,Text,String
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-comment-attribute-restoration-20261014');prior=Path('/tmp/name-authored-comment-cleanup-20261014'); fixes=json.loads((out/'manifest.json').read_text());fix_by_path={x['path']:x for x in fixes['files']}
lexer=RustLexer(stripnl=False,ensurenl=False)
def rust_tokens(source):
 values=[]
 for index,kind,value in lexer.get_tokens_unprocessed(source):
  if kind in String.Doc:
   assert value.startswith(("///", "//!", "/**", "/*!")),(kind,value[:80])
   continue
  if (kind in Comment and not kind in Comment.Preproc) or kind in Whitespace or (kind in Text and value.isspace()):continue
  values.append((str(kind),value))
 return values
# The lexer must retain attributes and '#'/comment-like text inside original strings.
assert rust_tokens('#[test]\nfn p(){let x="// source\\n# literal";}')==rust_tokens('// note\n#[test]\nfn p(){let x="// source\\n# literal";}')
assert rust_tokens('#[test]\nfn p(){}')!=rust_tokens('fn p(){}')
assert rust_tokens('let x=r#"//data\\n#[test]"#;')!=rust_tokens('let x=r#"//changed\\n#[test]"#;')
rows=[]
for row in json.loads((prior/'manifest.json').read_text())['files']:
 p=row['path']
 if not p.endswith('.rs'):continue
 original=subprocess.check_output(['git','show','51ad3ca41:'+p],cwd=root,text=True)
 corrected=(prior/'files'/p).read_text()
 if p in fix_by_path:
  attrs=fix_by_path[p]['restored_attributes']
  old_code=[l for l in original.splitlines(True) if not l.lstrip().startswith('//')]
  bad_code=[l for l in corrected.splitlines(True) if not l.lstrip().startswith('//')]
  for tag,a,b,c,d in reversed(difflib.SequenceMatcher(None,old_code,bad_code,autojunk=False).get_opcodes()):
   if tag=='equal':continue
   assert tag=='delete' and b-a==1 and old_code[a].lstrip().startswith('#[')
   following=bad_code[c];assert corrected.count(following)==1
   corrected=corrected.replace(following,old_code[a]+following,1)
  dest=out/'corrected-delivery'/p;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(corrected)
 a,b=rust_tokens(original),rust_tokens(corrected)
 assert a==b,(p,[(x,y) for x,y in zip(a,b) if x!=y][:4])
 rows.append({'path':p,'token_count':len(a),'original_meaningful_tokens_sha256':hashlib.sha256(repr(a).encode()).hexdigest(),'corrected_meaningful_tokens_sha256':hashlib.sha256(repr(b).encode()).hexdigest(),'equal':True})
(out/'lexical-audit.json').write_text(json.dumps({'lexer':'Pygments RustLexer; Comment.Preproc (Rust attributes) retained; ordinary comment/whitespace removed; string tokens unchanged','attribute_and_string_controls':True,'files':rows},indent=2)+'\n')
print('Rust files audited',len(rows),'meaningful tokens',sum(x['token_count'] for x in rows),'all equal after exact attribute restoration')
