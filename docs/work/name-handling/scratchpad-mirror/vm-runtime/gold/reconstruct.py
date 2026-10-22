"""Rebuild each differing command's spec rendering as the golden commit's
CommandSpec Debug would have printed it (fields added since dropped, fields
reordered to the old declaration), hash it as golden.rs does, and compare
with the golden digest."""
import json, sys, re, os
import xxhash
G = os.path.dirname(os.path.abspath(__file__))
old = json.load(open(f'{G}/old.json')); new = json.load(open(f'{G}/new.json'))

class P:
    def __init__(s, t): s.t=t; s.i=0
    def ws(s):
        while s.i < len(s.t) and s.t[s.i]==' ': s.i+=1
    def peek(s): s.ws(); return s.t[s.i] if s.i < len(s.t) else ''
    def string(s):
        q=s.t[s.i]; j=s.i+1
        while s.t[j]!=q:
            j+= 2 if s.t[j]=='\\' else 1
        r=s.t[s.i:j+1]; s.i=j+1; return ('atom', r)
    def value(s):
        c=s.peek()
        if c in '"\'': return s.string()
        if c=='[':
            s.i+=1; items=s.seq(']'); return ('list', items)
        if c=='(':
            s.i+=1; items=s.seq(')'); return ('tuple', None, items)
        if c=='{':
            s.i+=1; return ('map', s.mapitems())
        m=re.compile(r'[^,\s\(\)\[\]\{\}:]+(?:::[^,\s\(\)\[\]\{\}:]+)*').match(s.t, s.i)
        word=m.group(0); s.i=m.end()
        if s.i < len(s.t) and s.t[s.i]==' ' and s.t[s.i+1:s.i+2]=='{':
            s.i+=2; return ('struct', word, s.fields())
        if s.i < len(s.t) and s.t[s.i]=='(':
            s.i+=1; return ('tuple', word, s.seq(')'))
        return ('atom', word)
    def seq(s, close):
        items=[]
        while True:
            if s.peek()==close: s.i+=1; return items
            items.append(s.value())
            if s.peek()==',': s.i+=1
    def fields(s):
        fs=[]
        while True:
            if s.peek()=='}': s.i+=1; return fs
            m=re.compile(r'(\w+): ').match(s.t, s.i); s.i=m.end()
            fs.append((m.group(1), s.value()))
            if s.peek()==',': s.i+=1
    def mapitems(s):
        items=[]
        while True:
            if s.peek()=='}': s.i+=1; return items
            k=s.value(); assert s.t[s.i]==':'; s.i+=1; v=s.value(); items.append((k,v))
            if s.peek()==',': s.i+=1

problems=set()
def pick(defs, names):
    best=max(defs, key=lambda d: len(set(d)&set(names)))
    return best
def render(v):
    k=v[0]
    if k=='atom': return v[1]
    if k=='list': return '['+', '.join(render(x) for x in v[1])+']'
    if k=='tuple': return (v[1] or '')+'('+', '.join(render(x) for x in v[2])+')'
    if k=='map': return '{'+', '.join(render(a)+': '+render(b) for a,b in v[1])+'}'
    name, fs = v[1], v[2]
    names=[f for f,_ in fs]
    if name in old and name in new:
        o=pick(old[name], names)
        if o!=names:
            missing=[f for f in o if f not in names]
            if missing: problems.add(f'{name} lacks old fields {missing}')
            d=dict(fs); fs=[(f,d[f]) for f in o if f in d]
    elif name not in old:
        problems.add(f'{name} not in old tree')
    if not fs: return name
    return name+' { '+', '.join(f+': '+render(x) for f,x in fs)+' }'

out=sys.argv[1]
golden_dir='/home/user/tcl-lsp/rust/tcl-spectcl/tests/golden'
total=ok=0; rows=[]
for fn in sorted(os.listdir(out)):
    if not fn.endswith('.renders'): continue
    snap=fn[:-len('.renders')]+'.snap'
    gold={}
    for line in open(f'{golden_dir}/{snap}'):
        m=re.match(r'command (\S+) line (\d+) overrides (\S+) degraded (\S+) spec (\w+) hooks (\w+) grammar (\w+)', line)
        if m: gold[m.group(1)]=m.groups()
    fresh={}
    for line in open(f'{out}/{snap}'):
        m=re.match(r'command (\S+) line (\d+) overrides (\S+) degraded (\S+) spec (\w+) hooks (\w+) grammar (\w+)', line)
        if m: fresh[m.group(1)]=m.groups()
    for line in open(f'{out}/{fn}'):
        cmd, rendering = line.rstrip('\n').split('\t',1)
        problems.clear()
        p=P(rendering); tree=p.value(); assert p.i==len(rendering), (cmd, p.i, len(rendering))
        rebuilt=render(tree)
        d=xxhash.xxh3_64_hexdigest(rebuilt.encode())
        g=gold.get(cmd); f=fresh.get(cmd)
        same_rest = g and f and g[:4]==f[:4] and g[5:]==f[5:]
        total+=1; match = g is not None and d==g[4]
        ok+= match and same_rest
        rows.append((snap, cmd, match, same_rest, sorted(problems)))
print(f'{ok}/{total} differing commands reproduce the golden digest after dropping post-golden fields, all other columns equal')
for r in rows:
    if not (r[2] and r[3]): print('UNEXPLAINED', r)
