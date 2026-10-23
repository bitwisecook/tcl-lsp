from pathlib import Path
import json,re,hashlib,difflib,datetime
r=Path('/workspace/naming-lsp-release/tcl-lsp');o=Path('/tmp/lsp-candidate-doc-contract-cleanup-v1');w=Path('/workspace/naming-migration-active/tcl-lsp')
p=r/'docs/design/compiler/value-transfer-consumers.md';s=p.read_text().replace('The pure-ish third-party commands are rows too.','Third-party commands are included under their declared routes.').replace('(#2164).','.')
s=s.replace('the same debt as the W230–W232 family','the same role boundary as the W230–W232 family').replace('the walker still calls the file, so it is reviewed, not deleted','the walker consumes these facts in this file').replace('a file waiver at review','a file-level CLI waiver')
s=s.replace('product drift limits, not a validation history.','enforced drift limits.')
p.write_text(s)
p=r/'rust/xtask/src/value_transfers.rs';s=p.read_text().replace('the ratchet\'s ledger','the ratchet contract').replace('expect("the ledger page")','expect("the consumer contract page")');p.write_text(s)
m=json.loads((o/'changes.json').read_text());diff=[];pin={}
for rel in m['changes']:
 p=r/rel;b=w/rel
 before=b.read_bytes() if b.exists() else b'';after=p.read_bytes() if p.exists() else b''
 # WORK comparison is labelled as such; candidate carried fixture path literals can coexist.
 if before!=after:
  diff.extend(difflib.unified_diff(before.decode().splitlines(True),after.decode().splitlines(True),fromfile='WORK/'+rel,tofile='candidate/'+rel))
 pin[rel]={'current_work_sha256':hashlib.sha256(before).hexdigest() if b.exists() else None,'candidate_sha256':hashlib.sha256(after).hexdigest() if p.exists() else None}
(o/'candidate-vs-current-work.patch').write_text(''.join(diff))
# Exact gate table retained independently of prose changes.
old=(o/'value-transfers-migration.md.txt').read_text() if (o/'value-transfers-migration.md.txt').exists() else (w/'docs/design/compiler/value-transfers-migration.md').read_text()
rows=lambda x:dict((a,int(b)) for a,b in re.findall(r'^\| `(rust/[^`]+\.rs)` \| (\d+) \|',x,re.M))
assert rows(old)==rows((r/'docs/design/compiler/value-transfer-consumers.md').read_text())
(o/'final-pins.json').write_text(json.dumps({'actual_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'comparison':'Current WORK versus candidate, not a claim that WORK is pristine candidate pre-edit baseline','ratchet_rows':rows(old),'paths':pin},indent=2)+'\n')
print('final paths',len(pin),'ratchet rows',len(rows(old)))
