"""Prepare immutable614b inputs only; never compile or execute a provider."""
from pathlib import Path
import ast
import hashlib
import json
import re
import shutil

BASE = Path(__file__).resolve().parent
ORIGINAL = Path('/workspace/.proofs/native-explicit-error-code-header-setter614')
SDK = Path('/workspace/tcl-lsp/tmp')
digest = lambda raw: hashlib.sha256(raw).hexdigest()


def pin(pins, path):
    resolved = Path(path).resolve(strict=True)
    assert resolved.is_file(), str(resolved)
    actual = digest(resolved.read_bytes())
    if str(resolved) in pins:
        assert pins[str(resolved)] == actual, 'retained original input changed: ' + str(resolved)
    pins[str(resolved)] = actual
    return {'path': str(path), 'resolved_path': str(resolved), 'sha256': actual}


manifest = json.loads((ORIGINAL / 'manifest.json').read_bytes())
original_raw = (ORIGINAL / 'request.json').read_bytes()
assert digest(original_raw) == manifest['request_sha256']
original = json.loads(original_raw)
assert digest((ORIGINAL / 'probe.c').read_bytes()) == original['probe_sha256']
assert not (BASE / 'request.json').exists() and not (BASE / 'providers').exists()
shutil.copyfile(ORIGINAL / 'probe.c', BASE / 'probe.c')
pins = {p: h for p, h in original['required_sha256'].items()
        if not p.startswith(str(ORIGINAL) + '/')}
compat_pins = []
for version in original['providers']:
    directory = SDK / ('tcl' + version) / 'compat'
    for path in sorted(directory.rglob('*')):
        if path.is_file():
            compat_pins.append(dict(pin(pins, path), provider=version,
                                    purpose='Selected original SDK compatibility file; actual compile dependency use is captured separately.'))

# Retain every observed real original dependency from the exact closed614 maps.
# Ephemeral compiler .o paths absent after compilation remain map-only facts.
map_pins, map_provenance = [], []
for path in sorted((ORIGINAL / 'providers').glob('*/*')):
    if path.name not in ['compile.d', 'link.map']:
        continue
    raw = path.read_bytes()
    map_provenance.append({'path': str(path), 'sha256': digest(raw), 'size': len(raw)})
    if path.name == 'compile.d':
        names = raw.decode().replace('\\\n', '').split(':', 1)[1].split()
    else:
        names = [line[5:] for line in raw.decode().splitlines() if line.startswith('LOAD ')]
    for name in names:
        path_input = Path(name)
        if path_input.is_file() and not str(path_input.resolve()).startswith(str(ORIGINAL) + '/'):
            map_pins.append(pin(pins, path_input))

# The original selected internal headers can include platform headers beyond
# the public-header request. Pin these finite compiler/platform include trees;
# exact actual dependency files continue to be checked before each execution.
platform_pins = []
for directory in [Path('/usr/include/linux'), Path('/usr/include/asm-generic'),
                  Path('/usr/include/x86_64-linux-gnu'),
                  Path('/usr/lib/gcc/x86_64-linux-gnu/14/include')]:
    assert directory.is_dir(), directory
    for path in sorted(directory.rglob('*')):
        if path.is_file():
            platform_pins.append(pin(pins, path))

joins = json.loads((ORIGINAL / 'source-windows.json').read_bytes())
for entry in joins:
    previous = Path(entry['request_excerpt'])
    assert digest(previous.read_bytes()) == entry['request_excerpt_sha256']
    selected = BASE / 'source-windows' / previous.name
    selected.parent.mkdir(exist_ok=True)
    shutil.copyfile(previous, selected)
    entry['request_excerpt'] = str(selected)
    pin(pins, selected)
join_raw = (json.dumps(joins, indent=2) + '\n').encode()
(BASE / 'source-windows.json').write_bytes(join_raw)
producer_builds = []
for old in original['provider_builds']:
    value = dict(old)
    value['command'] = [arg.replace(str(ORIGINAL) + '/', str(BASE) + '/') for arg in old['command']]
    producer_builds.append(value)

request = dict(original)
request.update(probe_file=str(BASE / 'probe.c'), launcher_file=str(BASE / 'launch.py'),
    launcher_sha256=digest((BASE / 'launch.py').read_bytes()), provider_builds=producer_builds,
    status='prepared-not-compiled-or-executed; Root alone launches after independent source/protocol review',
    prior_614_attempt={
        'request': str(ORIGINAL / 'request.json'), 'request_sha256': digest(original_raw),
        'probe_sha256': original['probe_sha256'], 'launcher_sha256': original['launcher_sha256'],
        'scope': 'Independent closed header-admission refusal before any provider execution. Original bytes are retained; no original output is input to614b.',
        'compiler_version_receipt': str(ORIGINAL / 'compiler-version.receipt.json'),
        'compiler_version_receipt_sha256': digest((ORIGINAL / 'compiler-version.receipt.json').read_bytes()),
        'outer_launch_log': '/tmp/native-explicit-error-code-header-setter614-launch.log',
        'outer_launch_log_sha256': digest(Path('/tmp/native-explicit-error-code-header-setter614-launch.log').read_bytes()),
        'actual_maps': map_provenance,
        'compile_receipt_limit': 'The original compiler streams/ELF/header map/link map exist; no compile receipt was saved. No retrospective process receipt or provider execute result is fabricated.'},
    input_extension={
        'all_selected_sdk_compat_regular_file_count': len(compat_pins),
        'compat_inputs': compat_pins,
        'actual_closed_614_map_real_input_associations': map_pins,
        'platform_compiler_header_pin_count': len(platform_pins),
        'platform_compiler_header_associations': platform_pins},
    process_protocol={
        'producer_receipt': 'Fsync original stdout/stderr and the actual process receipt immediately after process completion, before after-process input/header/link/runtime verification.',
        'verification': 'Strict actual dependencies must be present with exact hashes. Separate verification records never rewrite actual process receipts.',
        'refusal': 'Persist exact harness refusal, retained process receipt associations, and final closure verification status. A refusal does not authorize later processes.',
        'output_reuse': 'Fresh614b provider output directory only; compile independently from the unchanged probe and selected original SDK inputs.'})
for name in ['probe.c', 'launch.py', 'prepare-request.py', 'source-windows.json']:
    pin(pins, BASE / name)
request['required_sha256'] = pins
assert request['probe_sha256'] == digest((BASE / 'probe.c').read_bytes())
for name in ['launch.py', 'prepare-request.py']:
    ast.parse((BASE / name).read_text(), filename=str(BASE / name))
raw = (json.dumps(request, indent=2) + '\n').encode()
(BASE / 'request.json').write_bytes(raw)
final = {'request': str(BASE / 'request.json'), 'request_sha256': digest(raw),
    'probe_sha256': request['probe_sha256'], 'launcher_sha256': request['launcher_sha256'],
    'required_pin_count': len(pins), 'source_association_sha256': digest(join_raw),
    'compat_regular_input_count': len(compat_pins),
    'source_only_preparation': True, 'compiler_native_or_assertion_launches': 0,
    'independent_review_required_before_Root_launch': True,
    'original_614_bytes_unchanged': True}
(BASE / 'manifest.json').write_text(json.dumps(final, indent=2) + '\n')
print(json.dumps(final))
