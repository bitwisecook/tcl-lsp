from pathlib import Path
import hashlib
import json
import os
import subprocess
import time

base = Path('/workspace/.proofs')
folder = base / 'native-variable-container-envelope188'
request = json.loads((folder / 'request.json').read_text())
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
for version in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0', 'jim']:
    old = json.loads((base / 'native-variable-container-runtime187' / version / 'receipt.json').read_text())
    executable = Path(old['executable_path'])
    assert sha(executable) == old['executable_sha256']
    pins = old['required_sha256']
    for path, digest in pins.items():
        assert sha(Path(path)) == digest, path
    for case in request['cases']:
        source = Path(case['path'])
        assert sha(source) == case['source_sha256']
        assert source.read_bytes().isascii() and b'\r' not in source.read_bytes()
        output = folder / version / case['label']
        assert not output.exists()
        output.mkdir(parents=True)
        command = [str(executable), str(source)]
        start = time.monotonic()
        result = subprocess.run(command, env={**os.environ, **old['environment']},
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
        (output / 'stdout').write_bytes(result.stdout)
        (output / 'stderr').write_bytes(result.stderr)
        assert sha(source) == case['source_sha256']
        record = {'command': command, 'exit': result.returncode,
                  'seconds': time.monotonic() - start,
                  'version_pin_receipt': str(base / 'native-variable-container-runtime187' / version / 'receipt.json'),
                  'reported_version_row': old['reported_version_row'],
                  'executable_path': str(executable), 'executable_sha256': sha(executable),
                  'source_sha256': sha(source), 'stdout_sha256': sha(output / 'stdout'),
                  'stderr_sha256': sha(output / 'stderr'), 'environment': old['environment'],
                  'required_sha256': pins, 'rows': result.stdout.decode().splitlines(),
                  'input_channel': request['purpose'], 'control': case['label']}
        (output / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
        print(json.dumps({'provider': version, 'control': case['label'],
                          'exit': result.returncode, 'rows': record['rows'],
                          'stderr_bytes': len(result.stderr)}))
    for path, digest in pins.items():
        assert sha(Path(path)) == digest, path
