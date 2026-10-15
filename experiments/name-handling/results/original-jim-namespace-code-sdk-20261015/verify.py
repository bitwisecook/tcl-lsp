from pathlib import Path
import argparse
import hashlib
import json

parser = argparse.ArgumentParser()
parser.add_argument("--repository-root", type=Path, default=Path(__file__).resolve().parents[4])
parser.add_argument("--current-provider", action="store_true")
args = parser.parse_args()
here = Path(__file__).resolve().parent
receipt = json.loads((here / "archive-receipt.json").read_text())
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
for record in receipt["retained_files"]:
    path = here / record["path"]
    assert path.stat().st_size == record["bytes"]
    assert sha(path) == record["sha256"], path
for record in receipt["referenced_provider_snapshots"]:
    path = args.repository_root / record["repository_path"]
    assert path.stat().st_size == record["bytes"]
    assert sha(path) == record["sha256"], path
if args.current_provider:
    manifest = json.loads((here / "manifest.json").read_text())
    assert sha(Path(manifest["library"])) == manifest["library_sha256"]
    for record in manifest["provider_sources"]:
        assert sha(Path(record["original"])) == record["sha256"]
print(f"PASS {len(receipt['retained_files'])} retained files and {len(receipt['referenced_provider_snapshots'])} referenced provider snapshots")
