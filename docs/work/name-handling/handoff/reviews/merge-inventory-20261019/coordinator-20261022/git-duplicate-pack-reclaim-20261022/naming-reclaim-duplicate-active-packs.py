from pathlib import Path
import subprocess,os,json,hashlib,datetime,shutil
active=Path('/workspace/naming-migration-active/tcl-lsp');pub=Path('/workspace/naming-migration-publish/tcl-lsp');old=Path('/workspace/naming-migration/tcl-lsp/.git/objects');receipt=json.loads(Path('/tmp/git-pack-coverage-audit-20261022.json').read_text());env=dict(os.environ,GIT_NO_LAZY_FETCH='1',GIT_OPTIONAL_LOCKS='0');start=datetime.datetime.now(datetime.timezone.utc).isoformat()
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def oids(p):
 with p.open('rb') as f:r=subprocess.run(['git','show-index'],stdin=f,stdout=subprocess.PIPE,check=True,env=env)
 return {line.split()[1].decode() for line in r.stdout.splitlines()}
def capture(root):
 refs=subprocess.check_output(['git','show-ref','--head'],cwd=root,env=env,text=True)
 walk=subprocess.check_output(['git','rev-list','--objects','--all','--missing=print','--no-object-names'],cwd=root,env=env,timeout=45,text=True)
 # Exact before/after object IDs and missing sentinels, no payload download.
 return {'refs':refs,'reachable_or_missing':sorted(set(walk.splitlines())),'worktrees':subprocess.check_output(['git','worktree','list','--porcelain'],cwd=root,env=env,text=True)}
assert (active/'.git/objects/info/alternates').read_text().strip()==str(old)
assert (pub/'.git/objects/info/alternates').read_text().strip()==str(active/'.git/objects')
assert not (old/'info/alternates').exists()
prepack=Path(receipt['preload_pack']['path']);preidx=prepack.with_suffix('.idx');assert prepack.stat().st_size==receipt['preload_pack']['bytes'];assert sha(preidx)==receipt['preload_pack']['index_sha256'];covered=oids(preidx);assert len(covered)==86849
before={str(root):capture(root) for root in [active,pub]};before_free=shutil.disk_usage(active).free;removed=[]
for item in receipt['active_duplicate_candidate_packs']:
 stem=active/'.git/objects/pack'/item['stem'];idx=stem.with_suffix('.idx');pack=stem.with_suffix('.pack');assert sha(idx)==item['idx_sha256'];current=oids(idx);assert len(current)==item['oid_count'] and not current-covered
 assert pack.stat().st_size==item['pack_bytes']
for item in receipt['active_duplicate_candidate_packs']:
 stem=active/'.git/objects/pack'/item['stem']
 for ext in ['.pack','.idx','.promisor']:
  p=stem.with_suffix(ext)
  if p.exists():removed.append({'path':str(p),'bytes':p.stat().st_size});p.unlink()
after={str(root):capture(root) for root in [active,pub]};assert before==after,'Git lookup availability changed; preserve receipt and investigate using retained alternate'
for root in [active,pub]:subprocess.run(['git','status','--porcelain=v1','-uno'],cwd=root,env=env,check=True,stdout=subprocess.PIPE)
result={'started_utc':start,'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'Only four exactly duplicate active-clone Git pack/index/promisor sets; no worktree/source/index/refs/shallow/target/test executable changed. Existing acyclic alternates unchanged.','preload_pack_retained':str(prepack),'coverage_receipt_sha256':sha(Path('/tmp/git-pack-coverage-audit-20261022.json')),'all_candidate_oids_covered_by_retained_preload':True,'before_after_refs_worktrees_and_reachable_missing_sets_equal':True,'reachable_inventory_counts':{root:len(v['reachable_or_missing']) for root,v in before.items()},'removed':removed,'removed_bytes':sum(x['bytes'] for x in removed),'free_before':before_free,'free_after':shutil.disk_usage(active).free,'unique_packs_and_all_publisher_packs_retained':True}
Path('/tmp/naming-duplicate-pack-reclaim-result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
