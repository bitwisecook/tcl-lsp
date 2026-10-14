import datetime
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path('/workspace/tcl-lsp')
OUT = Path('/tmp/name-issues-refresh-20261014/oracles-04')
OUT.mkdir(exist_ok=True)
SCRIPTS = {'provider-identity': 'puts [info patchlevel]\n', '2296-nested-array-components': 'set k x\nset b(x) key\nset a(key) VALUE\nputs $a($b($k))\n', '2255-braced-index-is-literal': 'set {a($k)} LITERAL\nputs ${a($k)}\n', '2260-empty-variable-name': 'set {} EMPTY\nputs ${}\nputs [set {}]\n', 'written-index-head-is-not-builtin-role': 'set a(key) VALUE\nrename ::set ::original_set\nproc ::set {args} {return key}\nputs $a([set k])\n'}

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def tree(path):
    return {str(p.relative_to(path)): sha(p) for p in sorted(path.rglob('*')) if p.is_file() and '.git' not in p.parts}

TCLLIB = Path('/tmp/name-issues-refresh-20261014/tcllib-tcllib-2-0')

def inputs(name, binary, library):
    return {'binary': sha(binary), 'library': tree(library), 'shared_libraries': {str(p): sha(p) for p in sorted(binary.parent.glob('libtcl*.so*')) if p.is_file()}, 'tcllib': tree(TCLLIB)}

providers = [(f'tcl{v}', ROOT / f'tmp/tcl{v}/unix/tclsh', ROOT / f'tmp/tcl{v}/library') for v in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0']]
providers.append(('jim', ROOT / 'tmp/jimtcl-oracle/jimsh', ROOT / 'tmp/jimtcl-oracle'))
records = []
before = {}
for name, binary, library in providers:
    before[name] = inputs(name, binary, library)
    env = os.environ.copy()
    if name != 'jim':
        env['TCL_LIBRARY'] = str(library)
        env['LD_LIBRARY_PATH'] = str(binary.parent)
        env['TCLLIBPATH'] = ' '.join(str(TCLLIB / 'modules' / component) for component in ['struct', 'cmdline'])
    provider = OUT / name
    provider.mkdir(exist_ok=True)
    for case, source in SCRIPTS.items():
        case_dir = provider / case
        case_dir.mkdir(exist_ok=True)
        script = case_dir / 'source.tcl'
        script.write_text(source)
        started = datetime.datetime.now(datetime.timezone.utc).isoformat()
        try:
            answer = subprocess.run([str(binary), str(script)], env=env, capture_output=True, timeout=20)
            code, stdout, stderr = answer.returncode, answer.stdout, answer.stderr
        except subprocess.TimeoutExpired as e:
            code, stdout, stderr = None, e.stdout or b'', e.stderr or b''
        (case_dir / 'stdout').write_bytes(stdout)
        (case_dir / 'stderr').write_bytes(stderr)
        row = {'provider': name, 'case': case, 'argv': [str(binary), str(script)], 'started_at': started, 'exit_code': code, 'source_sha256': sha(script), 'binary_sha256': before[name]['binary'], 'stdout_sha256': sha(case_dir / 'stdout'), 'stderr_sha256': sha(case_dir / 'stderr')}
        (case_dir / 'receipt.json').write_text(json.dumps(row, indent=2) + '\n')
        records.append(row)
        print(name, case, code, repr(stdout.decode(errors='replace')[:150]), repr(stderr.decode(errors='replace')[:100]), flush=True)
after = {name: inputs(name, binary, library) for name, binary, library in providers}
seal = {'recorded_at': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'runner_sha256': sha(Path(__file__)), 'jim_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT / 'tmp/jimtcl-oracle', text=True).strip(), 'changed_provider_inputs': [name for name in before if before[name] != after[name]], 'records': records, 'authority': 'Exact direct original interpreter executions only. No tcltest, Rust acceptance, native receipt admission, F5 or successful unavailable package execution is claimed.'}
seal['tcllib_tag'] = 'tcllib-2-0'
seal['tcllib_revision'] = '2a63bf212cc6423efb7a39958596ae2a2333c266'
seal['tcllib_archive_sha256'] = sha(TCLLIB.parent / 'tcllib-2-0.tar.gz')
seal['provider_environment'] = {'C Tcl': {'TCL_LIBRARY': 'matching provider library', 'LD_LIBRARY_PATH': 'matching provider unix directory', 'TCLLIBPATH': [str(TCLLIB / 'modules' / p) for p in ['struct', 'cmdline']]}, 'Jim': 'inherited environment, no C Tcl library or package donation'}
for name, snapshot in [('provider-inputs-before.json.gz', before), ('provider-inputs-after.json.gz', after)]:
    with (OUT / name).open('wb') as stream:
        with gzip.GzipFile(fileobj=stream, mode='wb', mtime=0) as compressed:
            compressed.write((json.dumps(snapshot, indent=2) + '\n').encode())
(OUT / 'outcomes.json').write_text(json.dumps(seal, indent=2) + '\n')
