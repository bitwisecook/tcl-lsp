from pathlib import Path
import re,json
out=Path('/tmp/name-inline-fixtures-main-20261013');root=Path('/workspace/tcl-lsp');rows=json.load(open(out/'remaining-literal-body-accesses.json'))['rows']
exclude=['/analyser/','/command_binding/logical_purpose_tests.rs','/interprocedural.rs','/optimiser/propagation.rs']
paths=sorted({r['path'] for r in rows if r['path'].startswith('rust/tcl-compiler/src/') and not any(x in r['path'] for x in exclude)})
records=[]
for rel in paths:
 src=root/rel;s=src.read_text();orig=s
 # Only standalone all-test modules or the suffix after the first cfg(test) module.
 testwhole=src.name.endswith('_tests.rs') or src.name=='tests.rs'
 marker=re.search(r'#\[cfg\(test\)\]\s*(?:(?:pub(?:\(crate\))?\s+)?mod\s+\w+\s*\{|mod\s+\w+\s*;)',s)
 if testwhole:prefix='';t=s
 elif marker:prefix=s[:marker.start()];t=s[marker.start():]
 else:print('NO TEST MODULE',rel);continue
 # Insert the shared private cfg(test) helper imports once inside every test module.
 imports='    use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};\n'
 if testwhole:
  offset=t.index('\nuse ')+1 if '\nuse ' in t else 0
  t=t[:offset]+imports.strip()+'\n'+t[offset:]
 else:
  t=re.sub(r'(#\[cfg\(test\)\]\s*(?:pub(?:\(crate\))?\s+)?mod\s+\w+\s*\{)',lambda m:m[1]+'\n'+imports,t)
 # Literal membership retains absence; display tests cannot manufacture an ID.
 pattern=r'([A-Za-z_]\w*(?:\s*\.\s*[A-Za-z_]\w*)*)\s*\.(procedures|methods)\s*\.contains_key\(("[^"\n]*")\)'
 t=re.sub(pattern,lambda m:f'fixture_id_if_present(&{m[1]}.{m[2]}, {m[3]}).is_some()',t)
 # A mutable operation must compute the ID before borrowing the owner mutably.
 pattern=r'([A-Za-z_]\w*(?:\s*\.\s*[A-Za-z_]\w*)*)\s*\.(procedures|methods)\s*\.(get_mut|remove)\(("[^"\n]*")\)'
 t=re.sub(pattern,lambda m:f'{{ let id = fixture_id(&{m[1]}.{m[2]}, {m[4]}); {m[1]}.{m[2]}.{m[3]}(&id) }}',t)
 pattern=r'([A-Za-z_]\w*(?:\s*\.\s*[A-Za-z_]\w*)*)\s*\.(procedures|methods)\s*\.get\(("[^"\n]*")\)'
 t=re.sub(pattern,lambda m:f'fixture_id_if_present(&{m[1]}.{m[2]}, {m[3]}).and_then(|id| {m[1]}.{m[2]}.get(&id))',t)
 pattern=r'([A-Za-z_]\w*(?:\s*\.\s*[A-Za-z_]\w*)*)\s*\.(procedures|methods)\s*\[("[^"\n]*")\]'
 t=re.sub(pattern,lambda m:f'{m[1]}.{m[2]}.get(&fixture_id(&{m[1]}.{m[2]}, {m[3]})).expect("authored fixture body")',t)
 # Preserve old Index's borrowed result without an extra &&T after replacement.
 t=re.sub(r'&([A-Za-z_]\w*(?:\s*\.\s*[A-Za-z_]\w*)*\.(?:procedures|methods)\.get\(&fixture_id\(&[^\n]+?\)\)\.expect\("authored fixture body"\))',r'\1',t)
 # Remove unused imports independently in each test module, using source ranges.
 modules=list(re.finditer(r'#\[cfg\(test\)\]\s*(?:pub(?:\(crate\))?\s+)?mod\s+\w+\s*\{',t))
 if testwhole:segments=[(0,len(t))]
 else:segments=[(m.start(),modules[i+1].start() if i+1<len(modules) else len(t)) for i,m in enumerate(modules)]
 for start,end in reversed(segments):
  segment=t[start:end];match=re.search(r'use crate::interprocedural::test_body_identity::[^;]+;',segment)
  if not match:continue
  body=segment[:match.start()]+segment[match.end():]
  used=[name for name in ['fixture_id','fixture_id_if_present'] if re.search(r'\b'+name+r'\(',body)]
  replacement='use crate::interprocedural::test_body_identity::{'+', '.join(used)+'};' if len(used)>1 else ('use crate::interprocedural::test_body_identity::'+used[0]+';' if used else '')
  segment=segment[:match.start()]+replacement+segment[match.end():];t=t[:start]+segment+t[end:]
 final=prefix+t
 if final==orig:continue
 pre=out/'broader-before'/rel;pre.parent.mkdir(parents=True,exist_ok=True);pre.write_text(orig)
 after=out/'broader-after'/rel;after.parent.mkdir(parents=True,exist_ok=True);after.write_text(final)
 records.append({'path':rel,'testwhole':testwhole,'prefix_length':len(prefix),'prefix':prefix})
(out/'broader-paths.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='prefix'} for r in records],indent=2))
print('files',len(records))
