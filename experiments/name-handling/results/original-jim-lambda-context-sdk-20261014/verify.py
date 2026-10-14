from pathlib import Path
import hashlib
import json

root = Path(__file__).resolve().parent
receipt = json.loads((root / "archive-receipt.json").read_text())
count = 0
for capture in receipt["captures"]:
    for artifact in capture["archived_files"]:
        actual = hashlib.sha256((root / artifact["path"]).read_bytes()).hexdigest()
        assert actual == artifact["sha256"], artifact["path"]
        count += 1
print(f"Verified {count} exact archived original artifacts")
