from pathlib import Path
import hashlib
import json
import os
import subprocess
import time

base = Path('/workspace/.proofs')
folder = base / 'native-child-alias-publication199'
request = json.loads((folder / 'request.json').read_text())
source = folder / 'probe.tcl'
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
assert sha(source) == request['source_sha256']
assert source.read_bytes().isascii() and b'\r' not in source.read_bytes()
for version in request['required_providers']:
    old = json.loads((base / 'native-jim-class-source269' / version / 'receipt.json').read_text())
    executable = Path(old['executable_path'])
    assert sha(executable) == old['executable_sha256']
    pins = dict(old['required_sha256'])
    if version == 'jim':
        pass
    else:
        package_source = Path('/workspace/tcl-lsp/tmp') / ('tcl' + version) / 'generic/tclInterp.c'
        pins[str(package_source)] = sha(package_source)
    for name, digest in pins.items():
        assert sha(Path(name)) == digest, name
    output = folder / version
    assert not output.exists()
    output.mkdir()
    command = [str(executable), str(source)]
    start = time.monotonic()
    result = subprocess.run(command, env={**os.environ, **old['environment']},
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
    (output / 'stdout').write_bytes(result.stdout)
    (output / 'stderr').write_bytes(result.stderr)
    rows = result.stdout.decode().splitlines()
    expected_version = request['jim_expected_version'] if version == 'jim' else version
    assert rows[0] == 'version ' + expected_version or rows[0].startswith('version ' + expected_version + ' '), rows[0]
    assert result.returncode == 0 and not result.stderr
    for name, digest in pins.items():
        assert sha(Path(name)) == digest, name
    assert sha(source) == request['source_sha256']
    record = {'command': command, 'exit': result.returncode,
              'seconds': time.monotonic() - start, 'reported_version_row': rows[0],
              'executable_path': str(executable), 'executable_sha256': sha(executable),
              'source_sha256': sha(source), 'stdout_sha256': sha(output / 'stdout'),
              'stderr_sha256': sha(output / 'stderr'), 'environment': old['environment'],
              'required_sha256': pins, 'rows': rows, 'input_channel': request['channel'],
              'question': request['question']}
    (output / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps({'provider': version, 'exit': result.returncode, 'rows': rows}))
