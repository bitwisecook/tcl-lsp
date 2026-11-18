import re,sys
from collections import defaultdict
NODE = re.compile(r"^(\s*)n(\d+): (\d+) (?:0x[0-9A-F]+: )?(.*)$")
def roots(path):
    text=open(path).read(); tree=text[text.index("heap_tree="):].splitlines()[1:]
    out=defaultdict(int); stack=[]
    for line in tree:
        m=NODE.match(line)
        if not m: continue
        ind,nch,size,name=len(m.group(1)),int(m.group(2)),int(m.group(3)),m.group(4)
        name=re.sub(r" \(in /.*\)$","",name)
        while stack and stack[-1][0]>=ind: stack.pop()
        stack.append((ind,name))
        if nch==0:
            q=[n for _,n in stack if re.search(r"Configuration_>>::execute|_Configuration_ as salsa", n)]
            key=re.sub(r".*IngredientImpl<|.*<tcl_lsp_db::","",q[-1])[:120] if q else "(no salsa query) "+ " < ".join(n[:60] for _,n in stack[-1:-6:-1] if not n.startswith(("alloc","core","std","<alloc","<core","(heap","hashbrown","<hashbrown")))
            out[key]+=size
    return out
a=roots(sys.argv[1]); b=roots(sys.argv[2])
for d,k in sorted(((b.get(k,0)-a.get(k,0),k) for k in set(a)|set(b)),reverse=True)[:20]:
    print(f"{d/1e6:8.2f} MB {b.get(k,0)/1e6:8.1f}  {k[:300]}")
