import sys,collections
keys=sys.argv[2].split(',')
blocks=open(sys.argv[1]).read().split('=== sample')[1:]
c=collections.Counter()
for b in blocks:
    k=next((k for k in keys if k in b),'OTHER')
    c[k]+=1
    if k=='OTHER' and len(sys.argv)>3: print(b[:3000])
for k,v in c.most_common(): print(v,k)
