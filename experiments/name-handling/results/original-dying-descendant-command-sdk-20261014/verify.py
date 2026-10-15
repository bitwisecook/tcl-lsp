from pathlib import Path
import hashlib
import json
import sys
here = Path(__file__).resolve().parent
repo = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else here.parents[3]
receipt = json.loads((here / "archive-receipt.json").read_text())
for row in receipt["files"]:
    path = here / row["path"]
    assert hashlib.sha256(path.read_bytes()).hexdigest() == row["sha256"], str(path)
for row in receipt["referenced_provider_snapshots"]:
    path = repo / row["path"]
    assert hashlib.sha256(path.read_bytes()).hexdigest() == row["sha256"], str(path)
print(f"verified {len(receipt['files'])} retained and {len(receipt['referenced_provider_snapshots'])} referenced files")
