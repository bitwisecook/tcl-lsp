from pathlib import Path
import os,shutil,hashlib,json,datetime
src=Path('/tmp/lsp-proposal-diagnostics-owner');dst=Path('/workspace/naming-validation/agent-work/preserved-tmp-packets')/src.name;assert src.is_dir()and not src.is_symlink()and not dst.exists()
def snapshot(p):
 out=[]
 for f in sorted(p.rglob('*')):
  if f.is_symlink():out.append({'path':str(f.relative_to(p)),'symlink':os.readlink(f)})
  elif f.is_file():
   with f.open('rb')as stream:h=hashlib.file_digest(stream,'sha256').hexdigest()
   out.append({'path':str(f.relative_to(p)),'bytes':f.stat().st_size,'sha256':h})
 return out
before=snapshot(src);dst.parent.mkdir(parents=True,exist_ok=True);shutil.copytree(src,dst,symlinks=True);assert snapshot(dst)==before;backup=src.with_name(src.name+'.verified-move-backup');assert not backup.exists();src.rename(backup);src.symlink_to(dst,target_is_directory=True);assert snapshot(src)==before;shutil.rmtree(backup);receipt={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'original_lookup_path':str(src),'payload_destination':str(dst),'original_path_preserved_as_symlink':True,'all_file_bytes_and_symlink_targets_verified_before_deleting_duplicate':True,'bytes_moved':sum(v.get('bytes',0)for v in before),'files':before,'scope':'Relocation of old immutable source packet only; no source/evidence bytes altered or lost; original paths remain valid.'};Path('/workspace/naming-validation/old-tmp-packet-storage-relocation.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({k:v for k,v in receipt.items()if k!='files'}))
