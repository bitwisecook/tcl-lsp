from pathlib import Path
import argparse, hashlib, json
parser=argparse.ArgumentParser()
parser.add_argument("--repository-root",type=Path,default=Path(__file__).resolve().parents[4])
parser.add_argument("--current-provider",action="store_true")
args=parser.parse_args(); here=Path(__file__).resolve().parent
receipt=json.loads((here/"archive-receipt.json").read_text())
sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
for record in receipt["retained_files"]:
    path=here/record["path"]
    assert path.stat().st_size==record["bytes"] and sha(path)==record["sha256"],path
for record in receipt["referenced_provider_snapshots"]:
    path=args.repository_root/record["repository_path"]
    assert path.stat().st_size==record["bytes"] and sha(path)==record["sha256"],path
manifest=json.loads((here/"manifest.json").read_text())
assert manifest["provider_before_sha256"]==manifest["provider_after_sha256"]
if args.current_provider:
    for original, expected in manifest["provider_after_sha256"].items():
        assert sha(Path(original))==expected,original
print(f"PASS {len(receipt['retained_files'])} retained files and {len(receipt['referenced_provider_snapshots'])} referenced provider snapshots")
