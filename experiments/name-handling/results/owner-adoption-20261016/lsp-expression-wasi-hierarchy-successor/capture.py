#!/usr/bin/env python3
"""Freeze compact receipts only; never run source tools, Cargo, tests or Git."""
from pathlib import Path
import argparse,hashlib,io,json,subprocess,tarfile,shutil
ROOT=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def write_json(p,x):p.write_text(json.dumps(x,indent=2,sort_keys=True)+'\n')
def verify():
 m=json.loads((ROOT/'manifest.json').read_text());a=ROOT/m['archive']['name'];assert sha(a.read_bytes())==m['archive']['sha256']
 raw=subprocess.run(['zstd','--decompress','--stdout',str(a)],check=True,capture_output=True).stdout
 assert sha(raw)==m['archive']['uncompressed_tar_sha256']
 with tarfile.open(fileobj=io.BytesIO(raw),mode='r:') as t:
  names=t.getnames();assert names==sorted(m['members']) and len(names)==len(set(names))
  for f in t:
   assert f.isfile() and not f.name.startswith('/') and '..' not in Path(f.name).parts
   data=t.extractfile(f).read();expected=m['members'][f.name];assert len(data)==expected['bytes'] and sha(data)==expected['sha256'],f.name
 print(json.dumps({'verified_members':len(m['members']),'archive':str(a),'bytes':a.stat().st_size,'sha256':sha(a.read_bytes())},indent=2))
def capture():
 assert not (ROOT/'manifest.json').exists(),'Immutable output exists; use --verify or a new destination'
 spec=json.loads((ROOT/'capture-inputs.json').read_text());stage=ROOT/'capsule';stage.mkdir(exist_ok=False);members={}
 def put(name,data,source=None):
  assert not name.startswith('/') and '..' not in Path(name).parts and name not in members
  assert not data.startswith(b'\x7fELF'),name
  f=stage/name;f.parent.mkdir(parents=True,exist_ok=True);f.write_bytes(data);members[name]={'bytes':len(data),'sha256':sha(data),'source':source}
 for row in spec['files']:
  p=Path(row['source']);assert p.is_file() and not p.is_symlink();data=p.read_bytes();assert sha(data)==row['sha256'],str(p);put(row['member'],data,str(p))
 refs=[]
 for row in spec['references']:
  p=Path(row['path']);data=p.read_bytes();assert sha(data)==row['sha256'],str(p)
  if row.get('json_entry_count') is not None:assert len(json.loads(data))==row['json_entry_count'],str(p)
  refs.append(row)
 put('references/excluded-large-inputs.json',(json.dumps(refs,indent=2,sort_keys=True)+'\n').encode())
 put('preservation-scope.json',(json.dumps(spec['scope'],indent=2,sort_keys=True)+'\n').encode())
 put('README.md',spec['readme'].encode())
 put('tools/capture.py',Path(__file__).read_bytes(),str(Path(__file__)))
 put('tools/capture-inputs.json',(ROOT/'capture-inputs.json').read_bytes(),str(ROOT/'capture-inputs.json'))
 tar=ROOT/'expression-batch-preservation.tar'
 with tarfile.open(tar,'w',format=tarfile.PAX_FORMAT) as t:
  for name in sorted(members):
   data=(stage/name).read_bytes();info=tarfile.TarInfo(name);info.size=len(data);info.mode=0o644;info.mtime=0;info.uid=info.gid=0;info.uname=info.gname='';t.addfile(info,io.BytesIO(data))
 uncompressed_sha=sha(tar.read_bytes());uncompressed_bytes=tar.stat().st_size
 archive=ROOT/'expression-batch-preservation.tar.zst';subprocess.run(['zstd','--threads=1','-10','--quiet',str(tar),'-o',str(archive)],check=True)
 m={'status':'COMPACT_PRESERVATION_ONLY_NO_NEW_EXECUTION_OR_ACCEPTANCE','members':members,'member_count':len(members),'member_bytes':sum(x['bytes'] for x in members.values()),'archive':{'name':archive.name,'bytes':archive.stat().st_size,'sha256':sha(archive.read_bytes()),'uncompressed_tar_sha256':uncompressed_sha,'uncompressed_tar_bytes':uncompressed_bytes},'excluded_reference_count':len(refs),'missing_or_unrecovered':spec['scope']['missing_or_unrecovered']};write_json(ROOT/'manifest.json',m);verify();tar.unlink()
 print(json.dumps({'manifest_sha256':sha((ROOT/'manifest.json').read_bytes()),'total_raw_member_bytes':m['member_bytes'],'missing_or_unrecovered':m['missing_or_unrecovered']},indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--capture',action='store_true');p.add_argument('--verify',action='store_true');a=p.parse_args();assert a.capture!=a.verify
 capture() if a.capture else verify()
