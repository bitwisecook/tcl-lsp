import re, collections
L='../../../../test-scrub/ledger/values.tsv'
M='../../../../test-scrub/values.md'
rows=[l.rstrip('\n').split('\t') for l in open(L)][1:]
v=collections.Counter(r[3] for r in rows); st=collections.Counter(r[7] for r in rows)
s=open(M).read()
for k in ['KEEP-ORACLE','KEEP-CONTRACT','KEEP-F5','FIX-FIXTURE','FIX-EXPECTATION','REWRITE','RETIRE','MERGE','UNDECIDED']:
    s=re.sub(r'\| '+re.escape(k)+r' \| \d+ \|', f'| {k} | {v.get(k,0)} |', s)
routed={k[8:-1]:n for k,n in st.items() if k.startswith('routed (')}
order=sorted(routed,key=lambda k:-routed[k])
rt=', '.join(f'{k} {routed[k]}' for k in order)
para=(f"Of {len(rows)} rows: {st['open']} open with values, {st.get('exposed',0)} exposed (the test was corrected and fails on a product cause, most of them routed), {sum(routed.values())} routed to the owner of their cause ({rt}),\n"
      f"{st.get('repaired',0)} repaired (corrected and passing), {st['fixed']} fixed, {st['deleted']} deleted. The status column names each owner.")
s=re.sub(r'Of \d+ rows: .*?The status column names each owner\.', para, s, flags=re.S)
open(M,'w').write(s)
print(para)
