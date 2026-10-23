#!/usr/bin/env python3
"""Reapply reviewed include-only cleanup after root functional carry; default check only."""
import pathlib,json,re,hashlib,argparse,datetime
BASE=pathlib.Path('/tmp/lsp-release-fixture-closure-audit-v1');CANDIDATE=pathlib.Path('/workspace/naming-lsp-release/tcl-lsp')
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--apply',action='store_true');parser.add_argument('--receipt',required=True,type=pathlib.Path);a=parser.parse_args();assert a.receipt.is_relative_to('/tmp'),'receipt must be outside candidate'
token=re.compile(r"r(?P<h>\#{0,8})\".*?\"(?P=h)|'(?:\\u\{[0-9a-fA-F]+\}|\\x[0-9a-fA-F]{2}|\\.|[^'\\])'|\"(?:\\.|[^\"\\])*\"|//[^\n]*|/\*.*?\*/|[A-Za-z_]\w*|\S",re.S)
def toks(s):return [(m.group(),m.start()) for m in token.finditer(s) if not m.group().startswith(('//','/*'))]
def digest(s):return hashlib.sha256(s.encode()).hexdigest()
prepared=[]
for row in json.loads((BASE/'reapply-literal-map.json').read_text())['consumers']:
 p=CANDIDATE/row['consumer'];s=p.read_text();ts=toks(s);regions=[]
 for i,(v,pos) in enumerate(ts):
  if v not in {'include','include_str','include_bytes'} or [x[0] for x in ts[i+1:i+3]]!=['!','(']:continue
  j=i+3;depth=1
  while depth:depth+=ts[j][0]=='(';depth-=ts[j][0]==')';j+=1
  regions.extend(ts[i+3:j-1])
 edits=[];sites=[]
 for pair in row['literal_pairs']:
  old=[pos for v,pos in regions if v==pair['old']];new=[pos for v,pos in regions if v==pair['new']];assert len(old)+len(new)==pair['expected_sites'],(row['consumer'],pair,len(old),len(new))
  edits.extend((pos,pos+len(pair['old']),pair['new']) for pos in old);sites.append({'old':pair['old'],'new':pair['new'],'reapply_sites':len(old),'already_clean_sites':len(new)})
 final=s
 for start,end,new in sorted(set(edits),reverse=True):final=final[:start]+new+final[end:]
 prepared.append((p,final,{'consumer':row['consumer'],'before_sha256':digest(s),'after_sha256':digest(final),'sites':sites}))
# Validate all files before any optional mutation; never copy functional source.
if a.apply:
 for p,final,receipt in prepared:
  assert digest(p.read_text())==receipt['before_sha256'],'candidate changed during validation'
 for p,final,receipt in prepared:
  if receipt['before_sha256']!=receipt['after_sha256']:p.write_text(final)
receipt={'actual_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'Root-reviewed cleanup reapply only; no functional source copied/overwritten','applied':a.apply,'consumers':[x[2] for x in prepared]};a.receipt.write_text(json.dumps(receipt,indent=2)+'\n');print('APPLIED' if a.apply else 'CHECK ONLY',len(prepared),'files',sum(x['reapply_sites'] for _,_,r in prepared for x in r['sites']),'pending literal edits')
