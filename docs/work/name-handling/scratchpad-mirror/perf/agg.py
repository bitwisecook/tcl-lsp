import sys,re,collections
txt=open(sys.argv[1]).read().split('----')
inc=collections.Counter(); n=0
for s in txt:
    fr=[l for l in s.strip().split('\n') if l.startswith('#')]
    if not fr: continue
    n+=1
    seen=set()
    for l in fr:
        m=re.match(r'#\d+\s+(?:0x\w+ in )?(.+?) \(',l)
        if not m: continue
        f=m.group(1)
        f=re.sub(r'<.*?>','',f) if len(f)>200 else f
        if f in seen: continue
        seen.add(f); inc[f]+=1
print("samples",n)
for f,c in inc.most_common(int(sys.argv[2]) if len(sys.argv)>2 else 80):
    if 'tcl_' in f or 'zz' in f: print(c, f[:180])
