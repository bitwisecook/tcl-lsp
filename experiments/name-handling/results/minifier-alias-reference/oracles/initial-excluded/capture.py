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
    ('BRACED_NAMESPACE_TAIL', 'set a ALIAS; namespace eval ::a {}; set ::a::tail QUALIFIED; set observed "${a}::tail"'),
    ('BARE_NAMESPACE_TAIL', 'set a ALIAS; namespace eval ::a {}; set ::a::tail QUALIFIED; set observed "$a::tail"'),
    ('BRACED_SINGLE_COLON_TAIL', 'set a ALIAS; set observed "${a}:tail"'),
    ('BARE_SINGLE_COLON_TAIL', 'set a ALIAS; set observed "$a:tail"'),
    ('BRACED_IDENTIFIER_TAIL', 'set a ALIAS; set ax OTHER; set observed "${a}x"'),
    ('BARE_IDENTIFIER_TAIL', 'set a ALIAS; set ax OTHER; set observed "$ax"'),
    ('BRACED_UNICODE_TAIL', 'set a ALIAS; set aé UNICODE; set observed "${a}é"'),
    ('BARE_UNICODE_TAIL', 'set a ALIAS; set aé UNICODE; set observed "$aé"'),
    ('BRACED_ARRAY_LOOKING_TAIL', 'set a ALIAS; set observed "${a}(k)"'),
    ('BARE_ARRAY_READ', 'set a(k) ELEMENT; set observed "$a(k)"'),
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
            'scope': 'Original six-provider alias-reference suffix spelling/value observations BEFORE new minifier constructor conformance assertions. Whole namespace/identifier/Unicode/array-looking suffixes and single-colon behavior retain provider differences. No alias freshness, movement, whole transform equivalence, F5, Native, frame/cell/edit or Rust acceptance.',
            'rows': rows}
(base / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(sha((base / 'manifest.json').read_bytes()))
print([(r['provider_label'], r['exit_code']) for r in rows])
