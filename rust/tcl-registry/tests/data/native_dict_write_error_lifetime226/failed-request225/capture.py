#!/usr/bin/env python3
"""Run the finite separate C85 dictionary release observer; preserve originals."""
from pathlib import Path
import hashlib
import json
import shutil
import subprocess
import time

request_dir = Path(__file__).resolve().parent
request_path = request_dir / 'request.json'
request_bytes = request_path.read_bytes()
request = json.loads(request_bytes)
output = Path(request['output_directory'])
required = request['required_sha256']
sha = lambda data: hashlib.sha256(data).hexdigest()

def verify_inputs():
    for path, expected in required.items():
        actual = sha(Path(path).read_bytes())
        if actual != expected:
            raise RuntimeError(f'Changed required input: {path}: {actual} != {expected}')

verify_inputs()
output.mkdir(exist_ok=False)
sdk = Path('/workspace/tcl-lsp/tmp/tcl8.5.19')
provider = json.loads((request_dir / 'original-provider.json').read_bytes())
archive = Path(provider['library'])
if sha(archive.read_bytes()) != request['original_provider_archive_sha256']:
    raise RuntimeError('Current archive differs from original C85 provider archive')
launcher_sha = sha(Path(__file__).read_bytes())
request_sha = sha(request_bytes)
receipts = []

def run(label, command, cwd=None, timeout=300):
    verify_inputs()
    started = time.monotonic()
    completed = subprocess.run(command, cwd=cwd, capture_output=True, timeout=timeout)
    stdout_path = output / f'{label}.stdout'
    stderr_path = output / f'{label}.stderr'
    stdout_path.write_bytes(completed.stdout)
    stderr_path.write_bytes(completed.stderr)
    receipt = {
        'label': label,
        'command': command,
        'cwd': str(cwd) if cwd else None,
        'exit': completed.returncode,
        'seconds': time.monotonic() - started,
        'request_sha256': request_sha,
        'launcher_sha256': launcher_sha,
        'required_sha256': required,
        'stdout_sha256': sha(completed.stdout),
        'stderr_sha256': sha(completed.stderr),
        'instrumented_build': label.startswith('observed-') or label.startswith('build-'),
        'scope': request['scope'],
    }
    for artifact in [output / 'original-probe', output / 'observed-probe', output / 'libtcl8.5-observed.a', output / 'tclExecute.o', output / 'tclObj.o']:
        if artifact.exists():
            receipt.setdefault('actual_artifact_sha256', {})[str(artifact)] = sha(artifact.read_bytes())
    (output / f'{label}.receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    receipts.append(receipt)
    verify_inputs()
    if completed.returncode:
        raise RuntimeError(f'{label} exited {completed.returncode}; complete streams retained')
    print(f'{label}: exit {completed.returncode}, stdout {len(completed.stdout)} bytes, stderr {len(completed.stderr)} bytes', flush=True)
    return completed

original_command = list(provider['command'])
original_command[original_command.index(provider['command'][3])] = str(request_dir / 'original-probe.c')
original_command[-1] = str(output / 'original-probe')
run('original-compile', original_command)
run('original-case9', [str(output / 'original-probe'), '9'], timeout=60)

makefile = output / 'Makefile.observer'
makefile.write_text(
    f'include {sdk / "unix/Makefile"}\n'
    f'BUILD_DIR := {sdk / "unix"}\n'
    '.PHONY: observed-execute observed-object\n'
    'observed-execute:\n'
    f'\t$(CC) -c $(CC_SWITCHES) {request_dir / "instrumented-source/tclExecute.c"} -o {output / "tclExecute.o"}\n'
    'observed-object:\n'
    f'\t$(CC) -c $(CC_SWITCHES) {request_dir / "instrumented-source/tclObj.c"} -o {output / "tclObj.o"}\n'
)
run('build-execute', ['make', '--no-print-directory', '-f', str(makefile), 'observed-execute'], cwd=sdk / 'unix')
run('build-object', ['make', '--no-print-directory', '-f', str(makefile), 'observed-object'], cwd=sdk / 'unix')
observed_archive = output / 'libtcl8.5-observed.a'
shutil.copyfile(archive, observed_archive)
(output / 'archive-copy.json').write_text(json.dumps({
    'operation': 'copy unchanged archive to independent scratch artifact before replacing two members',
    'source': str(archive), 'destination': str(observed_archive),
    'source_sha256': sha(archive.read_bytes()), 'copied_sha256': sha(observed_archive.read_bytes()),
    'request_sha256': request_sha,
}, indent=2) + '\n')
run('build-archive-replacement', ['ar', 'rcs', str(observed_archive), str(output / 'tclExecute.o'), str(output / 'tclObj.o')])
observed_command = list(original_command)
observed_command[observed_command.index(str(request_dir / 'original-probe.c'))] = str(request_dir / 'instrumented-probe.c')
observed_command[observed_command.index(str(archive))] = str(observed_archive)
observed_command[-1] = str(output / 'observed-probe')
run('observed-compile', observed_command)
for index in [9, 0, 19]:
    run(f'observed-case{index}', [str(output / 'observed-probe'), str(index)], timeout=60)
verify_inputs()
(output / 'capture-summary.json').write_text(json.dumps({
    'id': request['id'], 'request_sha256': request_sha, 'launcher_sha256': launcher_sha,
    'process_count': len(receipts), 'exits': [receipt['exit'] for receipt in receipts],
    'source_question': request['question'], 'scope': request['scope'],
    'source_builds_separate': True,
}, indent=2) + '\n')
