import pathlib,re,json,hashlib,datetime
root=pathlib.Path('/workspace/naming-lsp-release/tcl-lsp'); out=pathlib.Path('/tmp/lsp-candidate-doc-contract-cleanup-v1')
m=json.loads((out/'changes.json').read_text()); errors=[];checked=0
for rel in m['changes']:
 p=root/rel
 if not p.exists() or p.suffix!='.md':continue
 s=p.read_text()
 for label,url in re.findall(r'\[([^\]]*)\]\(([^\s)]+)(?:\s+[^)]*)?\)',s):
  url=url.strip('<>')
  if '://' in url or url.startswith(('mailto:','app:','plugin:')):continue
  path,_,frag=url.partition('#')
  target=(p.parent/path).resolve() if path else p
  # Only newly retargeted links/new contract outgoing links are this change's responsibility.
  if p.name not in ('value-transfer-consumers.md','embedded-tcl-contract.md') and target.name not in ('value-transfer-consumers.md','embedded-tcl-contract.md'):continue
  checked+=1
  if not target.exists():errors.append([rel,url,'missing path']);continue
  if frag and target.suffix=='.md':
   headings=[]
   for h in re.findall(r'^#+\s+(.+)$',target.read_text(),re.M):
    h=re.sub(r'[`*_]','',h).lower();h=re.sub(r'[^\w\- ]','',h);headings.append(h.replace(' ','-'))
   if frag not in headings:errors.append([rel,url,'missing heading'])
for rel,v in m['changes'].items():
 p=root/rel
 v['final_sha256']=hashlib.sha256(p.read_bytes()).hexdigest() if p.exists() else None
res={'actual_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'checked_changed_contract_links':checked,'errors':errors,'scope':'New contracts and all links retargeted to them; existing unrelated broken links excluded','final_paths':m['changes']}
(out/'linkcheck.json').write_text(json.dumps(res,indent=2)+'\n');print(json.dumps({'checked':checked,'errors':errors},indent=2))
