#!/usr/bin/env python3
"""Read an original captured log, verifying its retained bytes and identity."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("log", help="Original log filename recorded by result.json")
parser.add_argument("--capture", type=Path, default=Path(__file__).resolve().parent)
arguments = parser.parse_args()
capture = arguments.capture.resolve()
if Path(arguments.log).name != arguments.log:
    parser.error("use the original log filename without a directory")
mapping = json.loads((capture / "log-retention-map.json").read_text())
records = [row for row in mapping["logs"] if row["original"] == arguments.log]
if len(records) != 1:
    parser.error("original log must identify one retained record")
record = records[0]
packed = (capture / record["retained"]).read_bytes()
if hashlib.sha256(packed).hexdigest() != record["retained_sha256"]:
    parser.error("retained log digest differs")
raw = gzip.decompress(packed)
if len(raw) != record["original_bytes"] or hashlib.sha256(raw).hexdigest() != record["original_sha256"]:
    parser.error("original log identity differs")
sys.stdout.buffer.write(raw)
