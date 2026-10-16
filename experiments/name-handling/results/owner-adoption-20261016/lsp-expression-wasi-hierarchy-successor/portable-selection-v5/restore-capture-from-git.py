#!/usr/bin/env python3
"""Restore omitted historical source capture from exact committed Git objects."""
from pathlib import Path
import argparse,json,hashlib,subprocess,gzip
p=argparse.ArgumentParser();p.add_argument('--repository',required=True,type=Path);p.add_argument('--package',required=True,type=Path);a=p.parse_args();commit='3d92614629d1691c3ccde73e731ccef06bf578cf';binding=json.loads((a.package/'capture-binding.json').read_text());sources={}
for name,expected in binding['actual_current_sha256'].items():
 data=subprocess.check_output(['git','show',commit+':'+name],cwd=a.repository)
 assert hashlib.sha256(data).hexdigest()==expected,('Committed source mismatch',name)
 sources[name]=data.decode('utf-8')
raw=(json.dumps({'integrated-current-source':sources},separators=(',',':'))+'\n').encode();assert hashlib.sha256(raw).hexdigest()==binding['uncompressed_sha256'];compressed=gzip.compress(raw,mtime=0);assert hashlib.sha256(compressed).hexdigest()==binding['gzip_sha256'];target=a.package/'captured-current-source.json.gz'
if target.exists():assert target.read_bytes()==compressed
else:target.write_bytes(compressed)
manifest=json.loads((a.package/'package-manifest.json').read_text())
for name,v in manifest['files'].items():
 f=a.package/name;assert f.stat().st_size==v['bytes']and hashlib.sha256(f.read_bytes()).hexdigest()==v['sha256'],('Package member mismatch',name)
print(json.dumps({'restored_capture_sha256':binding['gzip_sha256'],'committed_source':commit,'sources':len(sources),'all_original_package_member_hashes_verified':True,'scope':'Historical source restoration only; no current-source/build/test authority'}))
