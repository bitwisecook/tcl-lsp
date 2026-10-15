from pathlib import Path
import datetime
import hashlib
import json
import os
import subprocess

base = Path(__file__).resolve().parent
prior = json.loads(Path('/tmp/name-expression-literal-word-value-oracles-controlled-20261015/manifest.json').read_text())
sha = lambda data: hashlib.sha256(data).hexdigest()
driver = Path(__file__).read_bytes()
cases = [
    ('SCALAR_BARE', 'set x SCALAR; set observed $x'),
    ('SCALAR_BRACED', 'set x SCALAR; set observed ${x}'),
    ('STATIC_ARRAY', 'set a(k) ELEMENT; set observed $a(k)'),
    ('COMPUTED_ARRAY', 'set i k; set a(k) ELEMENT; set {a($i)} LITERAL; set observed $a($i)'),
    ('BRACED_LITERAL_ARRAY_NAME', 'set i k; set a(k) ELEMENT; set {a($i)} LITERAL; set observed ${a($i)}'),
    ('QUOTED_COMPUTED_ARRAY', 'set i k; set a(k) ELEMENT; set {a($i)} LITERAL; set observed "v:$a($i)"'),
    ('QUOTED_BRACED_LITERAL_ARRAY_NAME', 'set i k; set a(k) ELEMENT; set {a($i)} LITERAL; set observed "v:${a($i)}"'),
    ('LITERAL_DOLLAR_SCALAR_NAME', 'set {$n} LITERAL; set observed ${$n}'),
    ('EMPTY_SCALAR_NAME', 'set {} EMPTY; set observed ${}'),
    ('CLOSER_NAME', 'set {a{b}c} NESTED; set observed ${a{b}c}'),
]
probe = 'puts [list PATCHLEVEL [info patchlevel]]\n'
for label, script in cases:
    probe += 'set script [binary format H* ' + script.encode().hex() + ']\n'
    probe += 'set complete [info complete $script]\nset code [catch {uplevel #0 $script} result]\n'
    probe += 'puts [list ' + label + ' COMPLETE $complete CODE $code RESULT $result]\n'
(base / 'probe.tcl').write_text(probe)
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
assert driver == Path(__file__).read_bytes()
manifest = {'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'probe_sha256': sha((base / 'probe.tcl').read_bytes()),
            'capture_driver_sha256': sha(driver), 'cases': cases,
            'scope': 'Original six-provider variable-reference spelling/value observations BEFORE formatter proposal conformance assertions. Scalar versus separate computed array index versus literal braced combined name and selected closer outcomes stay distinct. No F5/Native/runtime binding, frame, cell, edit, formatting or whole transformation equivalence acceptance.',
            'rows': rows}
(base / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(sha((base / 'manifest.json').read_bytes()))
print([(r['provider_label'], r['exit_code']) for r in rows])
