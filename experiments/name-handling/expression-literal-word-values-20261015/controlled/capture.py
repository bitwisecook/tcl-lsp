from pathlib import Path
import datetime
import hashlib
import json
import os
import subprocess

base = Path(__file__).resolve().parent
prior = json.loads(Path('/tmp/name-expression-literal-word-value-oracles-20261015/manifest.json').read_text())
sha = lambda data: hashlib.sha256(data).hexdigest()
source_driver = Path(__file__).read_bytes()
probe = Path('/tmp/name-expression-literal-word-value-oracles-20261015/probe.tcl').read_bytes()
assert sha(probe) == prior['probe_sha256']
(base / 'probe.tcl').write_bytes(probe)
rows = []
for original in prior['rows']:
    expected = original['original_files_before']
    before = {name: sha(Path(name).read_bytes()) for name in expected}
    assert before == expected
    env = os.environ.copy()
    env.pop('TCL_LIBRARY', None)
    if original.get('selected_original_library'):
        env['TCL_LIBRARY'] = original['selected_original_library']
    argv = [original['argv'][0], str(base / 'probe.tcl')]
    process = subprocess.run(argv, env=env, capture_output=True, timeout=20)
    after = {name: sha(Path(name).read_bytes()) for name in expected}
    assert before == after
    row = {'provider_label': original['provider_label'], 'argv': argv,
           'selected_original_library': original.get('selected_original_library'),
           'original_files_before': before, 'original_files_after': after,
           'exit_code': process.returncode}
    for channel, data in [('stdout', process.stdout), ('stderr', process.stderr)]:
        name = original[channel + '_file']
        (base / name).write_bytes(data)
        row[channel + '_file'] = name
        row[channel + '_sha256'] = sha(data)
    rows.append(row)
assert source_driver == Path(__file__).read_bytes()
manifest = {'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'probe_sha256': sha(probe), 'capture_driver_sha256': sha(source_driver),
            'initial_inline_capture_manifest': '/tmp/name-expression-literal-word-value-oracles-20261015/manifest.json',
            'initial_inline_capture_manifest_sha256': sha(Path('/tmp/name-expression-literal-word-value-oracles-20261015/manifest.json').read_bytes()),
            'scope': 'Same fixed original six-provider word-value cases repeated with this retained actual file driver before any new assertions. Initial inline capture remains untouched and separate. No F5 operator/path/handler/frame/cell/edit or Rust acceptance.',
            'rows': rows}
(base / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(sha((base / 'manifest.json').read_bytes()))
print([(row['provider_label'], row['exit_code']) for row in rows])
