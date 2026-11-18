import re,os,sys,collections
roots=["tcl-bytecode/src","tcl-runtime-api/src","tcl-cmd-core/src","tcl-regex/src","tcl-compiler/src","tcl-core-types/src","tcl-lsp-db/src","tcl-registry/src","tcl-dialect/src","tcl-syntax/src","tcl-lexer/src"]
defs=collections.defaultdict(list)
for r in roots:
  for dp,dn,fn in os.walk(r):
    if "test" in dp: continue
    for f in fn:
      if not f.endswith(".rs") or "test" in f: continue
      p=os.path.join(dp,f); s=re.sub(r"//[^\n]*","",open(p).read()); s=re.sub(r"'(?:\\.|[^\\'])'","'x'",s)
      for m in re.finditer(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(struct|enum)\s+([A-Z]\w*)[^;{(]*([{(;])",s,re.M):
        kind,name,op=m.groups()
        if op==";": continue
        i=m.end()-1; depth=0; close={"{":"}","(":")"}[op]; j=i
        while j<len(s):
          if s[j]==op: depth+=1
          elif s[j]==close:
            depth-=1
            if depth==0: break
          j+=1
        body=re.sub(r"//.*","",s[i:j+1])
        body=re.sub(r"#\[[^\]]*\]","",body)
        if kind=="enum": body=re.sub(r"(?m)^(\s*)[A-Z]\w*",r"\1",body)
        # qualified paths: keep last segment but tag with module

        defs[name].append((p,s.count("\n",0,m.start())+1,body))
alias=collections.defaultdict(set)
for r in roots:
  for dp,dn,fn in os.walk(r):
    for f in fn:
      if f.endswith(".rs"):
        for a,b in re.findall(r"\b([A-Z]\w*) as ([A-Z]\w*)\b",open(os.path.join(dp,f)).read()):
          if a!=b: alias[b].add(a)
ident={"ProcedureId","ProcedureModuleId","BodyFrameId","ProcedureIdIssuer"}
def tok(b):
  t=set(re.findall(r"\b([A-Z]\w*)\b",b))
  for x in list(t): t|=alias.get(x,set())
  return t
carr=set(ident); changed=True
while changed:
  changed=False
  for n,ds in defs.items():
    if n in carr: continue
    for p,l,b in ds:
      if tok(b)&carr: carr.add(n); changed=True; break
# shortest witness via BFS from identity outward: dist
dist={n:0 for n in ident}; nxt={}
changed=True
while changed:
  changed=False
  for n,ds in defs.items():
    for p,l,b in ds:
      for t in tok(b):
        if t in dist and t!=n and (n not in dist or dist[t]+1<dist[n]):
          dist[n]=dist[t]+1; nxt[n]=(t,p,l); changed=True
mode=sys.argv[1]
direct={n for n,ds in defs.items() for (p,l,b) in ds if tok(b)&ident}
if mode=="reach":
  from collections import deque
  for r in sys.argv[2:]:
    prev={r:None}; q=deque([r])
    while q:
      c=q.popleft()
      for p,l,b in defs.get(c,[]):
        for t in tok(b):
          if t in defs and t not in prev and t in carr:
            prev[t]=c; q.append(t)
    print("##",r)
    for d in sorted(direct & set(prev)):
      ch=[];c=d
      while c: ch.append(c); c=prev[c]
      print("  ",d,":"," <- ".join(ch[1:]))

if mode=="direct":
  for n,ds in sorted(defs.items()):
    for p,l,b in ds:
      if tok(b)&ident:
        print(f"{n} {p}:{l} ::", ";".join(x.strip()[:110] for x in b.split("\n") if tok(x)&ident))

if mode=="w":
  for r in sys.argv[2:]:
    if r not in dist: print(r,": NO"); continue
    chain=[];c=r
    while c in nxt:
      t,p,l=nxt[c]; chain.append(f"{c}({p}:{l})"); c=t
    print(r,":"," -> ".join(chain+[c]))
elif mode=="f":
  # list fields of type r that carry
  for r in sys.argv[2:]:
    for p,l,b in defs.get(r,[]):
      print("==",r,p,l)
      for line in b.split("\n"):
        ts=tok(line)&set(dist)
        if ts: print("   ",line.strip()[:160],"  <<",sorted(ts))
