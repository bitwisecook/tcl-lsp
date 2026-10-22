"""Apply reviewed integration37 pieces to the main tree, in order, with pinned hashes."""
from pathlib import Path
import hashlib,json,subprocess,datetime

S=Path('/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad');R=Path('/home/user/tcl-lsp');I=S/'i37'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
pieces=[
 ('db042-structural-factory-v2',S/'cp09/pending/sol-composition/db042-structural-factory-v2/change.patch','Luna-2 ADMIT source-level (integration-36-luna/REVIEW.json)'),
 ('roster-setup-v2',I/'roster/roster-setup-v2.patch','Luna-2 ADMIT with conditions; v2 adds the cycle comment and layout text'),
 ('objdefine-inline-v2',I/'objdefine/objdefine-inline-v2.patch','Luna-3 ADMIT after rustfmt (Sol fix-2)'),
 ('my-call-edges',S/'agents/sol-callhier/fix-4.patch','Luna-3 ADMIT (Sol fix-4)'),
 ('unknown-command-debt',S/'agents/sol-callhier/fix-5.patch','Luna-3 ADMIT (Sol fix-5)'),
 ('nextto-c-faithful-v2',I/'nextto/nextto-c-faithful-v2.patch','Sol-4 ADMIT with D1 doc fix and S1 applied'),
 ('nested-body-cut-memo-v2',S/'agents/luna-5/nested-body-cut-memo-v2.patch','Sol-3 ADMIT V1 with conditions; V2 addresses them; V2 delta review by Sol-7 pending'),
]
assert subprocess.run(['git','diff','--quiet','HEAD','--','rust','Cargo.lock','docs/design'],cwd=R).returncode==0,'tree not clean'
record={'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'base':subprocess.check_output(['git','rev-parse','HEAD'],cwd=R,text=True).strip(),'pieces':[]}
for name,patch,review in pieces:
    subprocess.run(['git','apply','--check',str(patch)],cwd=R,check=True)
    subprocess.run(['git','apply',str(patch)],cwd=R,check=True)
    record['pieces'].append({'name':name,'patch':str(patch),'sha256':sha(patch),'review':review})
changed=subprocess.check_output(['git','diff','--name-only'],cwd=R,text=True).split()
record['paths']={p:sha(R/p) for p in changed}
fmt=subprocess.run(['cargo','fmt','--all','--check'],cwd=R,capture_output=True,text=True)
record['cargo_fmt_check']={'exit':fmt.returncode,'stdout_tail':fmt.stdout[-2000:]}
(I/'composition.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({'paths':len(changed),'fmt':fmt.returncode}))
