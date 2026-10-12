import hashlib,json,re
from pathlib import Path
from collections import defaultdict
ROOT=Path('/workspace/tcl-lsp'); SRC=ROOT/'rust/tcl-compiler/src'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def mask(s):
 out=list(s); n=len(s); i=0
 def clear(a,b):
  for k in range(a,b):
   if out[k]!='\n':out[k]=' '
 while i<n:
  start=i
  if s.startswith('//',i):
   j=s.find('\n',i);j=n if j<0 else j;clear(i,j);i=j;continue
  if s.startswith('/*',i):
   j=i+2;depth=1
   while j<n and depth:
    if s.startswith('/*',j):depth+=1;j+=2
    elif s.startswith('*/',j):depth-=1;j+=2
    else:j+=1
   clear(i,j);i=j;continue
  raw=re.match(r'(?:br|cr|r)(#+)?"',s[i:i+64]) if s[i] in 'rbc' and (i==0 or not(s[i-1].isalnum() or s[i-1]=='_')) else None
  if raw:
   ending='"'+(raw.group(1) or '');j=s.find(ending,i+len(raw.group(0)));j=n if j<0 else j+len(ending);clear(i,j);i=j;continue
  if s[i]=='"':
   j=i+1
   while j<n:
    if s[j]=='\\':j+=2
    elif s[j]=='"':j+=1;break
    else:j+=1
   clear(i,min(j,n));i=j;continue
  if s[i]=="'":
   c=re.match(r"'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|[^\n])|[^'\\\n])'",s[i:])
   if c:j=i+len(c.group(0));clear(i,j);i=j;continue
  i+=1
 return ''.join(out)

index=defaultdict(list); module_declarations=[]; missing_module_files=[]; seen=set()
def visit(p,prefix,moddir,lineage):
 key=(str(p),tuple(prefix))
 if key in seen:return
 seen.add(key)
 if not p.exists():missing_module_files.append({'path':str(p),'module':'::'.join(prefix)});return
 s=p.read_text();file_sha=sha(p);m=mask(s); braces={};stack=[]
 for i,c in enumerate(m):
  if c=='{':stack.append(i)
  elif c=='}' and stack:braces[stack.pop()]=i
 mods=[]
 for hit in re.finditer(r'\bmod\s+([A-Za-z_]\w*)\s*([;{])',m):
  name,kind=hit.group(1,2);start=hit.start();br=hit.end()-1
  attrs_start=max(m.rfind(';',0,start),m.rfind('}',0,start),m.rfind('{',0,start))+1
  attrs=s[attrs_start:start]
  path=re.findall(r'#\[\s*path\s*=\s*"([^"\n]+)"\s*\]',attrs)
  mods.append({'name':name,'kind':kind,'start':start,'open':br,'end':braces.get(br,n if False else len(m)),'path':path[-1] if path else None,'line':s.count('\n',0,start)+1})
 def containing(pos):return sorted([a for a in mods if a['kind']=='{' and a['open']<pos<a['end']],key=lambda a:a['open'])
 for hit in re.finditer(r'\bfn\s+([A-Za-z_]\w*)\s*(?:<[^;{}]*?>\s*)?\(',m):
  start=hit.start();a=max(m.rfind(';',0,start),m.rfind('}',0,start),m.rfind('{',0,start))+1
  attrs=s[a:start]
  if not re.search(r'#\[\s*(?:test|tokio::test|crate::rt::test)(?:\s*\([^]]*\))?\s*\]',attrs):continue
  mods_here=containing(start);selector='::'.join(prefix+[x['name'] for x in mods_here]+[hit.group(1)])
  body_open=m.find('{',hit.end());body_end=braces.get(body_open,body_open)
  body=s[body_open:body_end+1]
  index[selector].append({'path':str(p.relative_to(ROOT)),'source_definition_line':s.count('\n',0,start)+1,'source_sha256':file_sha,'ignored':bool(re.search(r'#\[\s*ignore(?:\s*\(|\s*\])',attrs)),'proof_questions':sorted(set(re.findall(r'naming\.[A-Za-z0-9_.-]+',body))),'module_declarations':lineage+[{'path':str(p.relative_to(ROOT)),'line':x['line'],'module':x['name'],'inline':True} for x in mods_here]})
 for a in mods:
  if a['kind']!=';':continue
  enclosing=containing(a['start']); childprefix=prefix+[x['name'] for x in enclosing]+[a['name']]
  childdir=moddir.joinpath(*[x['name'] for x in enclosing])
  if a['path']:child=p.parent/a['path']
  else:
   candidates=[childdir/(a['name']+'.rs'),childdir/a['name']/'mod.rs'];child=next((q for q in candidates if q.exists()),candidates[0])
  nextdir=child.parent if child.name=='mod.rs' else child.parent/child.stem
  declaration={'path':str(p.relative_to(ROOT)),'line':a['line'],'module':a['name'],'inline':False,'child_path':str(child.relative_to(ROOT)) if child.is_relative_to(ROOT) else str(child)}
  visit(child,childprefix,nextdir,lineage+[{'path':str(p.relative_to(ROOT)),'line':x['line'],'module':x['name'],'inline':True} for x in enclosing]+[declaration])

crates={}
for pkg in ['tcl-lsp-core','tcl-lsp-db','tcl-lsp-server','tcl-lexer','tcl-compiler']:
 index.clear(); seen.clear(); missing_module_files.clear()
 SRC=ROOT/'rust'/pkg/'src'
 visit(SRC/'lib.rs',[],SRC,[])
 crates[pkg]={'tests':dict(index),'missing_module_files':list(missing_module_files),'module_files':len(seen)}
 print(pkg, 'source tests',sum(map(len,index.values())), 'missing module files', len(missing_module_files))
integration_targets={}
for pkg in ['tcl-lsp-core','tcl-lsp-db','tcl-lsp-server','tcl-compiler']:
 targets={}
 for root in sorted((ROOT/'rust'/pkg/'tests').glob('*.rs')):
  index.clear();seen.clear();missing_module_files.clear()
  visit(root,[],root.parent/root.stem,[])
  targets[root.stem]={'target_kind':'integration-test','target_name':root.stem,'root_path':str(root.relative_to(ROOT)),'root_sha256':sha(root),'tests':dict(index),'missing_module_files':list(missing_module_files),'module_files':len(seen)}
 integration_targets[pkg]=targets
 print(pkg,'integration targets',len(targets),'source tests',sum(sum(map(len,t['tests'].values())) for t in targets.values()))
Path('/tmp/consumer-current-source-test-index382.json').write_text(json.dumps({'status':'Source only; fresh exact ELF --list admission and assertion execution pending.','crates':crates,'integration_targets':integration_targets},indent=2)+'\n')
