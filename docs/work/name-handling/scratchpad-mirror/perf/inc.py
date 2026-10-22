import sys,re,collections
blocks=open(sys.argv[1]).read().split('=== sample')
c=collections.Counter(); n=0
for b in blocks[1:]:
    lines=[l.strip() for l in b.split('\n')[1:] if l.strip()]
    n+=1; seen=set()
    for l in lines:
        if '@tcl-' not in l: continue
        name,loc=l.rsplit(' @',1); key=name[-60:]+' @'+loc
        if key in seen: continue
        seen.add(key); c[key]+=1
print('samples',n)
for k,v in c.most_common(int(sys.argv[2])): print(v,k)
