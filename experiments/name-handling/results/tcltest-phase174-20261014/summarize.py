"""Extract bounded primitive phase lines without materialising huge Debug lines."""
import argparse
from collections import Counter, deque
import gzip
import hashlib
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('log', type=Path)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
if args.output.exists():
    raise SystemExit('Refuse to overwrite an existing summary')
prefixes = (
    b'ANALYSER_PHASE ', b'DIAGNOSTIC_PHASE ', b'DIAGNOSTIC_CFG_EMISSION_PHASE ',
    b'COMPILER_PIPELINE_PHASE ', b'COMPILER_FUNCTION_PHASE_TOTALS ', b'SOURCE_PHASE ',
    b'SOURCE_DEFERRED_PHASE ', b'SOURCE_LEXICAL_PROJECTION_PHASE ',
    b'SOURCE_DEFERRED_INDEX_PHASE ', b'SOURCE_LOOKUP_PHASE ',
    b'SOURCE_PROVIDER_VALIDATION_PHASE ', b'SOURCE_ORIGINAL_GEOMETRY_PHASE ',
    b'SOURCE_DEFERRED_PROGRESS ', b'LOWER_SOURCE_PHASE ', b'LOWER_SCRIPT_PHASE ',
    b'LOWER_ORIGINAL_BODY_PHASE ', b'SOURCE_INVOCATION_PROJECT ',
    b'ANALYSER_TOKEN_PREPARE ', b'SOURCE_BODY_PROJECTION ', b'SOURCE_ROOT_OWNER_WORK ',
    b'SOURCE_LAYOUT_PHASE ', b'SOURCE_LAYOUT_SYNTAX_SUMMARY ', b'test result: ',
)
counts = Counter()
latest = deque(maxlen=256)
line = bytearray()
truncated = False
long_lines = 0
raw_bytes = 0
raw_hash = hashlib.sha256()

def finish():
    global long_lines
    value = bytes(line)
    if truncated:
        long_lines += 1
    # Rust's terse harness can prefix the first trace with `test NAME ... `.
    if value.startswith(b'test ') and b' ... ' in value:
        value = value.split(b' ... ', 1)[1]
    if value.startswith(prefixes):
        kind = value.split(b' ', 1)[0].decode('ascii', 'replace')
        counts[kind] += 1
        latest.append({'line': value.decode('utf-8', 'replace'), 'truncated': truncated})

with gzip.open(args.log, 'rb') as source:
    while chunk := source.read(64 * 1024):
        raw_bytes += len(chunk)
        raw_hash.update(chunk)
        for piece_index, piece in enumerate(chunk.split(b'\n')):
            if piece_index:
                finish()
                line.clear()
                truncated = False
            room = 4096 - len(line)
            line.extend(piece[:room])
            truncated = truncated or len(piece) > room
if line or truncated:
    finish()
args.output.write_text(json.dumps({
    'log': str(args.log.resolve()), 'decoded_bytes': raw_bytes,
    'decoded_sha256': raw_hash.hexdigest(), 'overlong_lines': long_lines,
    'primitive_line_counts': dict(counts), 'latest_primitive_lines': list(latest),
    'scope': 'Bounded primitive line extraction only; cumulative nested phase values overlap. No exclusive cost totals or semantic acceptance inferred.'
}, indent=2) + '\n')
print(json.dumps({'summary': str(args.output), 'primitive_lines': sum(counts.values()),
                  'decoded_bytes': raw_bytes, 'overlong_lines': long_lines}))
