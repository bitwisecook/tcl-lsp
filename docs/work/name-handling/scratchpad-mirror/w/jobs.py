import json,sys
from collections import Counter
d=json.load(open(sys.argv[1]));j=d['jobs']
j=j['jobs'] if isinstance(j,dict) else j
print(Counter((x.get('status'),x.get('conclusion')) for x in j))
for x in j:
    if x.get('conclusion') not in ('success','skipped'):
        bad=[s['name'] for s in x.get('steps',[]) if s.get('conclusion')=='failure']
        print(x['id'],x.get('status'),x.get('conclusion'),x['name'][:60],bad[:2])
