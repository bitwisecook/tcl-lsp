import re,collections,sys
t=open(sys.argv[1]).read()
threads=re.split(r'\n(?=Thread \d+ )',t)
names=collections.Counter(); sig=collections.Counter()
skip=('std::','core::','tokio::','__','syscall','futex','<','alloc::','parking_lot','start_thread','clone')
for th in threads:
    m=re.match(r'Thread \d+ \(Thread 0x\w+ \(LWP \d+\) "([^"]*)"\)',th)
    if not m: continue
    names[m.group(1)]+=1
    frames=re.findall(r'^#\d+\s+(?:0x\w+ in )?(\S+)',th,re.M)
    keep=[f for f in frames if not f.startswith(skip)]
    key=" < ".join(f[-70:] for f in keep[:4])
    sig[(m.group(1),key)]+=1
print(names.most_common())
for (n,k),c in sig.most_common(int(sys.argv[2]) if len(sys.argv)>2 else 25): print(c,n,'|',k)
