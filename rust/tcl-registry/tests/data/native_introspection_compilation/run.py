import fcntl
import hashlib
import json
import subprocess
from pathlib import Path

root = Path('/workspace/.proofs/2286-adapter-introspection-design')
records = []
with open('/workspace/.proofs/2286-test-slot-0.lock', 'a') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    for version in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0']:
        source = Path('/workspace/tcl-lsp/tmp') / f'tcl{version}'
        archive = source / 'unix' / f'libtcl{version[:3]}.a'
        binary = root / f'probe-{version}'
        argv = ['cc', '-std=c99', '-I' + str(source / 'generic'),
                '-I' + str(source / 'unix'), str(root / 'probe.c'),
                str(archive), '-lm', '-ldl', '-lpthread', '-lz', '-o', str(binary)]
        compile_result = subprocess.run(argv, capture_output=True, timeout=60, check=False)
        item = {'version': version, 'compile_argv': argv,
                'source_sha256': hashlib.sha256((root / 'probe.c').read_bytes()).hexdigest(),
                'archive_sha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
                'header_sha256': hashlib.sha256((source / 'generic/tcl.h').read_bytes()).hexdigest(),
                'compile_returncode': compile_result.returncode,
                'compile_stdout_hex': compile_result.stdout.hex(),
                'compile_stderr_hex': compile_result.stderr.hex()}
        if compile_result.returncode == 0:
            completed = subprocess.run([str(binary)], capture_output=True, timeout=60, check=False)
            item.update(binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                        argv=[str(binary)], returncode=completed.returncode,
                        stdout_hex=completed.stdout.hex(), stderr_hex=completed.stderr.hex())
            (root / f'{version}.tsv').write_bytes(completed.stdout)
            print(version, completed.returncode, completed.stdout.decode(), completed.stderr.decode())
        else:
            print(version, 'compile failure', compile_result.stderr.decode())
        records.append(item)
(root / 'manifest.json').write_text(json.dumps(records, indent=2) + '\n')
