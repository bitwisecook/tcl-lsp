"""Expand the normal pack-backed differential in a disposable working copy only."""
from pathlib import Path
import random,json,sys
p=Path(sys.argv[1])
s=p.read_text()
tag='// bd-pack-fuzz-20261119'
assert tag not in s
r=random.Random(20261119)
rows=[]
for trial in range(72):
 x,y,z=[r.randint(-250000,250000) for _ in range(3)]
 key,make,n=r.choice([
 ('vif',lambda:'vif $x',1),('vifarm',lambda:'vifarm $x',1),
 ('vdouble',lambda:'vdouble $x',1),('vdef',lambda:'vdef $x',1),
 ('vformat',lambda:'vformat $x',1),('vapply',lambda:'vapply llength $x',0),
 ('vwhile',lambda:'vwhile 2',0),('vfor',lambda:'vfor 3',0),('veach',lambda:'veach [list $x $y]',0),
 ('vdouble-effect',lambda:'vdouble [incr x]',0),
 ])
 body=make()
 caller=f'proc caller {{x y}} {{{body}}}'
 calls=[f'caller {x} {y}',f'caller {0 if trial%3 else z} {z}']
 if key in ('vdouble','vformat','vif','vifarm') and trial%3==0:
  calls.append('caller notAnInteger 1')
 rows.append((caller,calls,n))
lines=['    '+tag]
for caller,calls,n in rows:
 lines.append('    ('+json.dumps(caller)+', &['+', '.join(json.dumps(x) for x in calls)+f'], {n}),')
idx=s.index('\n',s.index('const CASES:'))+1
p.write_text(s[:idx]+'\n'.join(lines)+'\n'+s[idx:])
Path(sys.argv[2]).write_text(json.dumps(rows,indent=1))
