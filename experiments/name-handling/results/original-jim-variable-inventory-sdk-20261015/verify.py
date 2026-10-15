from pathlib import Path
import argparse, hashlib, json
parser=argparse.ArgumentParser()
parser.add_argument('--repository-root',type=Path,default=Path(__file__).resolve().parents[4])
parser.add_argument('--current-provider',action='store_true')
args=parser.parse_args();here=Path(__file__).resolve().parent
receipt=json.loads((here/'archive-receipt.json').read_text());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
for row in receipt['retained_files']:
 f=here/row['path'];assert f.stat().st_size==row['bytes'];assert sha(f)==row['sha256'],f
for row in receipt['referenced_provider_snapshots']:
 f=args.repository_root/row['repository_path'];assert f.stat().st_size==row['bytes'];assert sha(f)==row['sha256'],f
for row in receipt['capture_manifests']:
 f=here/row['path'];assert sha(f)==row['sha256'];m=json.loads(f.read_text());assert len((f.parent/'stdout.tsv').read_bytes().splitlines())==row['rows']
 if args.current_provider:
  assert sha(Path(m['library']))==m['library_sha256']
  for source in m['provider_sources']:assert sha(Path(source['original']))==source['sha256']
print(f"PASS {len(receipt['retained_files'])} retained files and {len(receipt['referenced_provider_snapshots'])} referenced provider snapshots")
