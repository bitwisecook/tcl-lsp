import datetime,hashlib,json,shlex,sys
from pathlib import Path
sys.path.insert(0,str(Path('scripts/dev/bigip-probes/name-patterns').resolve()))
import capture
root=Path('.').resolve()
dest=root/'experiments/name-handling/patterns/f5/answers/nh20261012cleanup2'
assert dest.is_dir()
(dest/'executed-helper-retry2.py').write_bytes(Path(__file__).read_bytes())
receipts=[]
for scope,tag in [('patterns/','nh20261012exact1'),('patterns/','nh20261012aplexact1'),('patterns/','nh20261012exact2'),('','nh20261012persist1'),('','nh20261012nsowner1'),('','nh20261012aplnsowner1')]:
 run=root/('experiments/name-handling/'+scope+'f5/answers/'+tag)
 seal=run/'MANIFEST.sha256'
 for line in seal.read_text().splitlines():
  digest,name=line.split('  ',1)
  assert hashlib.sha256((run/name).read_bytes()).hexdigest()==digest
 prov=run/'provenance.json'
 remote=json.loads(prov.read_text())['remote_staging']
 assert remote=='/var/tmp/tcl_lsp_'+tag
 q=shlex.quote(remote)
 command=f'if test -L {q}; then exit 9; fi; if test -d {q}; then find {q} -maxdepth 2 -type f; fi; rm -rf -- {q}; test ! -e {q}'
 result=capture.rpc(command)
 (dest/(tag+'.retry2.stdout')).write_bytes(result.stdout)
 (dest/(tag+'.retry2.stderr')).write_bytes(result.stderr)
 receipts.append({'run_id':tag,'provenance':str(prov.relative_to(root)),'provenance_sha256':hashlib.sha256(prov.read_bytes()).hexdigest(),'seal':str(seal.relative_to(root)),'seal_sha256':hashlib.sha256(seal.read_bytes()).hexdigest(),'remote_staging':remote,'argv':capture.SSH+[command],'exit_code':result.returncode,'staging_absent_after':result.returncode==0})
 assert result.returncode==0
inventory=[]
for kind in ['ltm rule','ltm virtual','ltm pool','sys application template','sys application service','cli script','sys icall script','sys icall handler periodic','sys folder','auth partition']:
 # Filtering runs remotely; the transcript includes the actual listing status.
 command='nh_listing=$(tmsh list '+kind+'); nh_exit=$?; printf "listing-exit=%s\\n" "$nh_exit"; if [ "$nh_exit" = 0 ]; then printf "%s\\n" "$nh_listing" | awk \'/__tcl_lsp_probe_nh20261012/ {print}\'; fi; exit "$nh_exit"'
 r=capture.rpc(command)
 label=kind.replace(' ','-')
 (dest/(label+'.retry2.stdout')).write_bytes(r.stdout)
 (dest/(label+'.retry2.stderr')).write_bytes(r.stderr)
 inventory.append({'kind':kind,'argv':capture.SSH+[command],'exit_code':r.returncode,'owned_tag_absent':r.returncode==0 and b'__tcl_lsp_probe_nh20261012' not in r.stdout})
 assert r.returncode==0 and b'__tcl_lsp_probe_nh20261012' not in r.stdout,(kind,r.stdout,r.stderr)
(dest/'receipts.json').write_text(json.dumps({'captured_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'configuration_saved':False,'receipts':receipts,'final_configuration_inventory':inventory,'retry_attempt':{'helper':'executed-helper-retry.py','exit_code':1,'failure':'The intended syntax substitution was not applied; same listing failure retained separately.'},'prior_attempt':{'helper':'executed-helper.py','exit_code':1,'failure':'Final ltm rule listing interpreted one-line as an object name; all six staging cleanup assertions had passed before that listing. Initial staging inventories and failed listing stdout/stderr are retained unchanged.'}},indent=2)+'\n')
(dest/'README.md').write_text('# Final owned staging and configuration checks\n\nAll six listed captures were retrieved, independently reviewed and seal-verified before their exact owned remote staging paths were removed. Every removal and post-removal check returned zero. The final separate listings contain no configuration object with this task\'s unique prefix across the ten listed object types. No probe configuration was saved. Runtime cleanup is bounded by the individual captures; earlier original-name interpreter state remains subject to the separate hosted cleanup review.\n')
print(json.dumps({'removed_owned_staging':len(receipts),'configuration_types_checked':len(inventory)}))
