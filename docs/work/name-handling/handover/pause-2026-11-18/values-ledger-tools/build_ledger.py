import json,sys
d=json.load(open('now_failing.json'))
by_path={}
for k in d:
    cr,t,p=k.split('\t'); by_path[p]=(cr,t)
files=sys.argv[1:]
rows=[]; seen=set()
for f in files:
    for line in open(f):
        line=line.rstrip('\n')
        if not line: continue
        parts=line.split('\t')
        if len(parts)==4: parts.append('')
        assert len(parts)==5,(f,line[:80])
        p,v,c,e,s=parts
        e=e.replace('\\"','"').replace('\\ufeff','U+FEFF')
        assert p in by_path,(f,p)
        assert p not in seen,p
        assert v in {'KEEP-ORACLE','KEEP-F5','KEEP-CONTRACT','FIX-EXPECTATION','FIX-FIXTURE','MERGE','REWRITE','RETIRE','UNDECIDED'},v
        seen.add(p)
        cr,t=by_path[p]
        rows.append([cr,t,p,v,c,e,s])
status={}
import os
if os.path.exists('status.tsv'):
    for l in open('status.tsv'):
        if l.strip():
            a,b=l.rstrip('\n').split('\t'); status[a]=b
for r in rows: r.append(status.get(r[2],'open'))
print('rows',len(rows),'unledgered',len([p for p in by_path if p not in seen]))
with open('../../../../test-scrub/ledger/values.tsv','w') as f:
    f.write('\t'.join(['crate','target','path','verdict','cause','evidence','survivor','status'])+'\n')
    for r in sorted(rows,key=lambda r:(r[4],r[2])): f.write('\t'.join(r)+'\n')
