from pathlib import Path
import argparse
import hashlib
import json

parser = argparse.ArgumentParser()
parser.add_argument("--root", required=True)
parser.add_argument("--apply", action="store_true")
args = parser.parse_args()
root = Path(args.root)
package = Path(__file__).resolve().parent
receipt = json.loads((package / "receipt.json").read_text())
restorations = []
for index, record in enumerate(receipt["records"]):
    target = root / record["path"]
    current = target.read_bytes()
    before = (package / f"{index}-before.rs.txt").read_bytes()
    proposed = (package / f"{index}-proposed.rs.txt").read_bytes()
    assert hashlib.sha256(current).hexdigest() == record["proposed_sha256"], record["path"]
    assert hashlib.sha256(proposed).hexdigest() == record["proposed_sha256"], record["path"]
    assert current == proposed, record["path"]
    assert hashlib.sha256(before).hexdigest() == record["before_sha256"], record["path"]
    restorations.append((target, before))
# Every exact before/proposed/current input is checked before either write.
if args.apply:
    for target, before in restorations:
        target.write_bytes(before)
print(json.dumps({"checked_paths": len(restorations), "applied": args.apply}))
