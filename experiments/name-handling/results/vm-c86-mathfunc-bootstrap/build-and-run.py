#!/usr/bin/env python3
"""Link a VM probe from the retained fresh Cargo artifact receipt and run it."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path('/workspace/tcl-lsp')
OUT = Path(__file__).resolve().parent


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


artifacts = {}
for line in (OUT / 'integration-build.jsonl').read_text().splitlines():
    if not line.startswith('{'):
        continue
    row = json.loads(line)
    if row.get('reason') != 'compiler-artifact':
        continue
    name = row['target']['name']
    if name in ['tcl_vm', 'tcl_compiler', 'tcl_registry']:
        for filename in row['filenames']:
            if filename.endswith('.rlib'):
                artifacts[name] = filename

assert len(artifacts) == 3, artifacts
binary = Path('/tmp/name-vm-c86-mathfunc-bootstrap-runner')
argv = ['rustc', '--edition=2024', str(OUT / 'real-init-runner.rs'),
        '-L', f'dependency={ROOT}/tmp/name-target-issues/debug/deps',
        '-o', str(binary)]
for name, filename in sorted(artifacts.items()):
    argv.extend(['--extern', f'{name}={filename}'])
compiled = subprocess.run(argv, cwd=ROOT, capture_output=True, timeout=120)
(OUT / 'runner-build.stdout.txt').write_bytes(compiled.stdout)
(OUT / 'runner-build.stderr.txt').write_bytes(compiled.stderr)
record = {'build_argv': argv, 'build_exit': compiled.returncode,
          'dependencies': {name: {'path': filename, 'sha256': digest(filename)}
                           for name, filename in artifacts.items()},
          'runner_source_sha256': digest(OUT / 'real-init-runner.rs')}
assert compiled.returncode == 0, compiled.stderr.decode(errors='replace')
env = os.environ.copy()
env['TCL_LIBRARY'] = str(ROOT / 'tmp/tcl8.6.18/library')
record['library'] = {'TCL_LIBRARY': env['TCL_LIBRARY'],
                     'init_sha256': digest(Path(env['TCL_LIBRARY']) / 'init.tcl')}
record['binary'] = {'path': str(binary), 'sha256': digest(binary)}
try:
    reached = subprocess.run([str(binary)], cwd=ROOT, env=env,
                             capture_output=True, timeout=45)
    stdout, stderr, exit_code, timed_out = reached.stdout, reached.stderr, reached.returncode, False
except subprocess.TimeoutExpired as error:
    stdout, stderr, exit_code, timed_out = error.stdout or b'', error.stderr or b'', None, True
(OUT / 'real-init.stdout.txt').write_bytes(stdout)
(OUT / 'real-init.stderr.txt').write_bytes(stderr)
record['run'] = {'argv': [str(binary)], 'cwd': str(ROOT), 'exit': exit_code,
                 'timeout': timed_out, 'stdout_sha256': hashlib.sha256(stdout).hexdigest(),
                 'stderr_sha256': hashlib.sha256(stderr).hexdigest(),
                 'startup_reached': exit_code == 0 and stdout == b'REACHED\n' and not stderr}
(OUT / 'real-init-record.json').write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps(record['run']))
