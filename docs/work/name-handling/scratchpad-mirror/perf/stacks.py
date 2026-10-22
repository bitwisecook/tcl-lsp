import sys,re
txt=open(sys.argv[1]).read().split('----')
for i,s in enumerate(txt):
    fr=[l for l in s.strip().split('\n') if l.startswith('#')]
    if not fr: continue
    out=[]
    for l in fr:
        m=re.search(r' in (.*?) \(',l) or re.match(r'#\d+\s+(.*?) \(',l)
        f=m.group(1) if m else l
        loc=re.search(r' at (\S+)$',l)
        if 'tcl_' in f or (loc and 'rust/tcl' in loc.group(1)):
            name=re.sub(r'<[^<>]*>','',f); name=re.sub(r'<[^<>]*>','',name); name=re.sub(r'<[^<>]*>','',name)
            out.append(name[-90:]+ (' @'+loc.group(1).split('rust/')[-1] if loc else ''))
    print("=== sample",i)
    # dedupe consecutive
    prev=None
    for o in out:
        if o!=prev: print("  ",o)
        prev=o
