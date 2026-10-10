#!/usr/bin/env python3
"""Root-only transparent rust-lld observer for the fixed Rust WASI probe."""
from pathlib import Path
import datetime
import hashlib
import json
import os
import shlex
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def pin(path):
    p = Path(path)
    return {'path': str(p), 'resolved_path': str(p.resolve()), 'bytes': p.stat().st_size, 'sha256': digest(p)}

def stamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def main():
    request = json.loads((ROOT / 'request.json').read_text())
    if digest(__file__) != request['link_wrapper_sha256']:
        raise SystemExit('changed link wrapper')
    linker = request['linker']
    expected = next(item for item in request['input_pins'] if item['path'] == linker)
    if digest(linker) != expected['sha256']:
        raise SystemExit('changed actual rust-lld')
    capture = Path(os.environ['GRAMMAR_WASI_PROBE_LINK_CAPTURE_DIR'])
    capture.mkdir(parents=True, exist_ok=False)
    argv = [linker] + sys.argv[1:]
    responses = []
    expanded_args = []
    for index, arg in enumerate(sys.argv[1:]):
        if arg.startswith('@'):
            p = Path(arg[1:])
            copied = capture / ('response-' + str(index) + '.bytes')
            copied.write_bytes(p.read_bytes())
            responses.append(dict(pin(p), captured_path=str(copied)))
            expanded_args.extend(shlex.split(p.read_text()))
        else:
            expanded_args.append(arg)
    original_file_args = []
    for index, arg in enumerate(expanded_args):
        p = Path(arg)
        if p.is_file():
            item = pin(p)
            if p.suffix == '.o':
                copied = capture / ('input-' + str(index) + '-' + p.name)
                copied.write_bytes(p.read_bytes())
                item['captured_path'] = str(copied)
            original_file_args.append(item)
    before = [pin(item['path']) for item in request['target_library_pins']]
    started = time.monotonic()
    record = {'argv': argv, 'cwd': str(Path.cwd()), 'started_utc': stamp(),
              'linker': pin(linker), 'response_files': responses,
              'expanded_args': expanded_args, 'original_file_args': original_file_args,
              'target_library_pins_before': before}
    process = subprocess.run(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    (capture / 'stdout.bytes').write_bytes(process.stdout)
    (capture / 'stderr.bytes').write_bytes(process.stderr)
    record.update(exit_code=process.returncode, duration_seconds=time.monotonic()-started,
                  finished_utc=stamp(), stdout=pin(capture / 'stdout.bytes'),
                  stderr=pin(capture / 'stderr.bytes'))
    after = [pin(item['path']) for item in request['target_library_pins']]
    record['target_library_pins_after'] = after
    record['target_libraries_unchanged'] = before == after == request['target_library_pins']
    (capture / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
    sys.stdout.buffer.write(process.stdout)
    sys.stderr.buffer.write(process.stderr)
    return process.returncode

if __name__ == '__main__':
    raise SystemExit(main())
