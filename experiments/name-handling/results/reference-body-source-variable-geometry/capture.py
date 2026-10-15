from pathlib import Path
import argparse
import datetime
import hashlib
import json
import os
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--repository-root', type=Path, required=True)
parser.add_argument('--out', type=Path, required=True)
args = parser.parse_args()
root = args.repository_root.resolve()
out = args.out.resolve()
out.mkdir(exist_ok=False)
probe = Path(__file__).with_name('probe.tcl')
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
manifest = root / 'rust/tcl-dialect/data/reference-toolchains.tsv'
providers = []
for line in manifest.read_text().splitlines():
    if line and not line.startswith('#'):
        release, version, tag = line.split('\t')
        tree = root / 'tmp' / ('tcl' + version)
        providers.append((version, tree / 'unix/tclsh', tree, tag))
jim = root / 'tmp/jimtcl-oracle'
revision = subprocess.check_output(['git', '-C', str(jim), 'rev-parse', 'HEAD'], text=True).strip()
providers.append(('Jim', jim / 'jimsh', jim, revision))
runs = []
for label, binary, tree, revision in providers:
    env = dict(os.environ)
    env.pop('TCL_LIBRARY', None)
    if label != 'Jim':
        env['TCL_LIBRARY'] = str(tree / 'library')
    inputs = [binary]
    for name in ['generic/tclParse.c', 'generic/tcl.h', 'library/init.tcl',
                 'library/tcltest/tcltest.tcl', 'unix/config.log', 'unix/Makefile',
                 'jim.c', 'jim-subcmd.c', 'jimautoconf.h', 'config.log', 'libjim.a']:
        if (tree / name).is_file():
            inputs.append(tree / name)
    before = {str(p): sha(p) for p in inputs}
    command = [str(binary), str(probe)]
    result = subprocess.run(command, env=env, capture_output=True)
    (out / (label + '.stdout')).write_bytes(result.stdout)
    (out / (label + '.stderr')).write_bytes(result.stderr)
    after = {str(p): sha(p) for p in inputs}
    assert before == after
    runs.append({'provider': label, 'revision': revision, 'argv': command,
                 'exit_code': result.returncode, 'inputs': before,
                 'stdout_sha256': hashlib.sha256(result.stdout).hexdigest(),
                 'stderr_sha256': hashlib.sha256(result.stderr).hexdigest(),
                 'library_override': env.get('TCL_LIBRARY'), 'input_drift': {}})
(out / 'manifest.json').write_text(json.dumps({
    'question_ids': ['reference-body-braced-qualified-name', 'reference-body-index-brace-substitution'],
    'captured_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'probe_sha256': sha(probe), 'toolchain_manifest_sha256': sha(manifest),
    'runs': runs, 'harness': 'Direct original interpreters; no Tcltest invoked',
    'build_owner': 'scripts/dev/ensure-test-deps.sh; scripts/dev/ensure-jim-oracle.sh',
}, indent=2) + '\n')
