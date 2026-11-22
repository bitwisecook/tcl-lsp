import collections, sys
rel = ['8.4','8.5','8.6','9.0','9.1']
oracle = {}; msgs = {}; subsets = {}
for v in rel:
    for fn in [f'oracle-{v}.tsv', f'xoracle-{v}.tsv']:
        for l in open(fn, errors='replace'):
            f = l.rstrip('\n').split('\t')
            if f[0] == 'P':
                m = f[5] if len(f) > 5 else ''
                c = f[4]
                if c == 'other' and any(x in m for x in ('not enough arguments','too few arguments','too many arguments','Incorrect number of arguments','wrong # args')): c = 'args'
                if c == 'other' and m.startswith('unknown method'): c = 'badsub'
                if c == 'nocmd' and ('"%s"' % f[1]) not in m and ('"%s"' % f[1].lstrip(':')) not in m: c = 'other'
                if c == 'badsub' and f[2] and ('"%s"' % f[2]) not in m: c = 'other'
                oracle[(v, f[1], f[2], int(f[3]))] = c; msgs[(v, f[1], f[2], int(f[3]))] = f[5] if len(f) > 5 else ''
            if f[0] == 'S': subsets[(v, f[1])] = set(f[2].split())
ana = {}
for l in open('analyse-out2.tsv', errors='replace'):
    f = l.rstrip('\n').split('\t')
    ana[(f[0][3:], f[1], f[2], int(f[3]))] = set(x for x in f[4].split(',') if x)
ARITY = {'E002','E003','E005'}
UNK = {'W123','W002'}
cats = collections.defaultdict(lambda: collections.defaultdict(list))
for k, cls in oracle.items():
    v, cmd, sub, n = k
    a = ana.get(k)
    if a is None: continue
    key = (cmd, sub)
    if 'oo::' in cmd and cls == 'nocmd' and sub: continue
    if cls in ('ok','other') and a & ARITY and not (a & UNK):
        cats['false-arity'][key].append(f'{v}:{n}')
    if cls == 'args' and not (a & (ARITY|UNK|{'E001','W001'})):
        cats['missed-arity'][key].append(f'{v}:{n}')
    if cls != 'nocmd' and a & UNK and n == 1:
        cats['false-unknown'][key].append(v)
    if cls == 'nocmd' and not (a & UNK) and n == 1:
        cats['claims-nonexistent'][key].append(v)
    if sub and cls == 'badsub' and n == 1 and not (a & ({'W001'}|UNK)):
        cats['claims-nonexistent-sub'][key].append(v)
    if sub and cls not in ('badsub','nocmd') and 'W001' in a and n == 1:
        cats['false-unknown-sub'][key].append(v)
for c, d in cats.items():
    print(f'== {c}: {len(d)} command/subcommand pairs')
    def m1(cmd, sub):
        for v in rel:
            for n in (0,1,2,3,4,12):
                if (v,cmd,sub,n) in msgs and msgs[(v,cmd,sub,n)]: return msgs[(v,cmd,sub,n)][:70]
        return ''
    for (cmd, sub), where in sorted(d.items())[: int(sys.argv[1]) if len(sys.argv) > 1 else 40]:
        print(f'   {cmd} {sub}: {" ".join(where)}   | {m1(cmd, sub)}')
