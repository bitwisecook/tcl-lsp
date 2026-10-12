"""Prepare exact selected Makefile/header inputs; never launch a producer."""
from pathlib import Path
import ast
import hashlib
import json
import re
import shlex
import shutil

BASE = Path(__file__).resolve().parent
PRIOR = Path('/workspace/.proofs/native-explicit-error-code-header-setter614b')
ORIGINAL = Path('/workspace/.proofs/native-explicit-error-code-header-setter614')
SDK = Path('/workspace/tcl-lsp/tmp')
digest = lambda raw: hashlib.sha256(raw).hexdigest()


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def pin(pins, path):
    path = Path(path)
    resolved = path.resolve(strict=True)
    assert resolved.is_file(), str(resolved)
    actual = sha(resolved)
    if str(resolved) in pins:
        assert pins[str(resolved)] == actual, 'retained input changed: ' + str(resolved)
    pins[str(resolved)] = actual
    return {'path': str(path), 'resolved_path': str(resolved), 'sha256': actual,
            'size': resolved.stat().st_size}


def make_values(path):
    raw = path.read_bytes()
    # Horizontal whitespace only: an empty assignment must not consume the
    # following physical line. This parser evaluates no make/shell commands.
    values = dict(re.findall(r'^([A-Za-z_][A-Za-z_0-9]*)[ \t]*=[ \t]*(.*)$',
                            raw.decode().replace('\\\n', ' '), re.M))
    selected, undefined = {}, set()

    def expand(key, stack=()):
        assert key not in stack, 'recursive selected Makefile variable: ' + key
        if key not in values:
            undefined.add(key)
            return ''
        selected[key] = values[key]
        value = re.sub(r'\$\(([A-Za-z_][A-Za-z_0-9]*)\)|\$\{([A-Za-z_][A-Za-z_0-9]*)\}',
                       lambda m: expand(m[1] or m[2], stack + (key,)), values[key])
        assert '$' not in value and '`' not in value, 'unsupported Makefile expansion'
        return value

    expanded = {key: expand(key) for key in ['CC', 'CC_SWITCHES', 'LIBS', 'TCL_LIB_FILE']}
    argv = {key: shlex.split(value) for key, value in expanded.items()}
    return {'makefile': str(path), 'makefile_sha256': digest(raw),
            'literal_selected_assignments': selected,
            'undefined_selected_variables_expanded_empty': sorted(undefined),
            'requested_config_variables_not_defined_here':
                [key for key in ['TCL_INCLUDES', 'DEFS', 'TCL_DEFS'] if key not in values],
            'expanded_values': expanded, 'literal_shell_word_argv': argv,
            'evaluation': 'Read literal assignments, recursively replace simple variable references, then split shell words. No make or shell process is invoked.'}


assert not (BASE / 'request.json').exists() and not (BASE / 'providers').exists()
prior_raw = (PRIOR / 'request.json').read_bytes()
prior = json.loads(prior_raw)
assert digest(prior_raw) == json.loads((PRIOR / 'manifest.json').read_bytes())['request_sha256']
prior_probe = (PRIOR / 'probe.c').read_bytes()
assert digest(prior_probe) == prior['probe_sha256']
old_includes = b'#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include "tclInt.h"'
new_includes = b'#include "tclInt.h"\n#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>'
assert prior_probe.count(old_includes) == 1
probe = prior_probe.replace(old_includes, new_includes)
assert probe.replace(new_includes, old_includes) == prior_probe
(BASE / 'probe.c').write_bytes(probe)

pins = {p: h for p, h in prior['required_sha256'].items()
        if not p.startswith(str(PRIOR) + '/')}
header_trees, builds, configurations = [], [], []
for selected in prior['provider_builds']:
    version = selected['provider']
    root = SDK / ('tcl' + version)
    cwd = root / 'unix'
    config = make_values(cwd / 'Makefile')
    pin(pins, cwd / 'Makefile')
    configurations.append(dict(config, provider=version, compile_cwd=str(cwd)))
    compiler_words = config['literal_shell_word_argv']['CC']
    assert compiler_words[0] == 'gcc'
    switches = config['literal_shell_word_argv']['CC_SWITCHES']
    assert not any(x.endswith('/compat') for x in switches if x.startswith('-I'))
    directory = BASE / 'providers' / version
    library = cwd / config['expanded_values']['TCL_LIB_FILE']
    pin(pins, library)
    command = [prior['compiler'], *compiler_words[1:], *switches,
               '-MD', '-MF', str(directory / 'compile.d'), str(BASE / 'probe.c'),
               str(library), *config['literal_shell_word_argv']['LIBS'],
               '-Wl,-Map,' + str(directory / 'link.map'), '-o', str(directory / 'probe.elf')]
    builds.append({'provider': version, 'command': command, 'compile_cwd': str(cwd),
                   'environment': selected['environment'],
                   'selected_makefile_sha256': config['makefile_sha256']})
    for name in ['generic', 'unix', 'compat', 'libtommath']:
        tree = root / name
        if not tree.exists():
            header_trees.append({'provider': version, 'root': str(tree), 'status': 'absent-in-selected-sdk', 'inputs': []})
            continue
        entries = [pin(pins, p) for p in sorted(tree.rglob('*'))
                   if p.is_file() and (p.suffix in ['.h', '.inc', '.def'] or '.h.' in p.name)]
        header_trees.append({'provider': version, 'root': str(tree),
                             'status': 'all-existing-header-and-include-files-pinned', 'inputs': entries})

# Complete platform/compiler include trees supplement independently measured
# dependencies. Actual reached dependencies must still rejoin these hashes.
platform = []
for tree in [Path('/usr/include'), Path('/usr/lib/gcc/x86_64-linux-gnu/14/include')]:
    entries = [pin(pins, p) for p in sorted(tree.rglob('*')) if p.is_file()]
    platform.append({'root': str(tree), 'inputs': entries})

attempts = []
maps = []
for attempt in [ORIGINAL, PRIOR]:
    receipts = []
    for path in sorted(attempt.rglob('*.receipt.json')):
        receipts.append(dict(pin(pins, path), purpose='Independent original actual process receipt; no output reuse.'))
    for name in ['request.json', 'manifest.json', 'probe.c', 'launch.py']:
        pin(pins, attempt / name)
    optional = []
    for name in ['closure.json', 'harness-refusal.json']:
        path = attempt / name
        if path.exists():
            optional.append(pin(pins, path))
    log = Path('/tmp/native-explicit-error-code-header-setter' + attempt.name.split('setter')[-1] + '-launch.log')
    assert log.exists(), log
    pin(pins, log)
    attempts.append({'root': str(attempt), 'request_sha256': sha(attempt / 'request.json'),
                     'actual_saved_process_receipts': receipts, 'closure_and_refusal_inputs': optional,
                     'outer_log': pin(pins, log),
                     'scope': 'Independent original bytes and recorded partial/failure boundaries. No process result is reused for this request.'})
    for path in sorted((attempt / 'providers').glob('*/*')):
        if path.name not in ['compile.d', 'link.map']:
            continue
        item = pin(pins, path)
        version = path.parent.name
        cwd = Path(json.loads((attempt / 'request.json').read_bytes())['probe_file']).parent
        raw = path.read_bytes()
        names = (raw.decode().replace('\\\n', '').split(':', 1)[1].split()
                 if path.name == 'compile.d' else
                 [line[5:] for line in raw.decode().splitlines() if line.startswith('LOAD ')])
        real = []
        for name in names:
            input_path = Path(name)
            if not input_path.is_absolute():
                input_path = cwd / input_path
            if input_path.is_file():
                real.append(pin(pins, input_path))
        maps.append(dict(item, actual_existing_inputs=real,
                         ephemeral_limit='Absent compiler temporary objects remain exact map paths, with no fabricated digest.'))

joins = json.loads((PRIOR / 'source-windows.json').read_bytes())
for entry in joins:
    previous = Path(entry['request_excerpt'])
    assert sha(previous) == entry['request_excerpt_sha256']
    selected = BASE / 'source-windows' / previous.name
    selected.parent.mkdir(exist_ok=True)
    shutil.copyfile(previous, selected)
    entry['request_excerpt'] = str(selected)
    pin(pins, selected)
join_raw = (json.dumps(joins, indent=2) + '\n').encode()
(BASE / 'source-windows.json').write_bytes(join_raw)
build_raw = (json.dumps(configurations, indent=2) + '\n').encode()
(BASE / 'selected-makefile-builds.json').write_bytes(build_raw)
request = dict(prior)
request.pop('prior_614_attempt', None)
request.update(probe_file=str(BASE / 'probe.c'), probe_sha256=digest(probe),
               launcher_file=str(BASE / 'launch.py'), launcher_sha256=sha(BASE / 'launch.py'),
               provider_builds=builds, independent_producer_configuration=configurations,
               prior_independent_attempts=attempts,
               status='prepared-not-compiled-or-executed; Root alone launches after independent source/protocol review',
               include_order={
                   'original_probe_sha256': digest(prior_probe), 'selected_probe_sha256': digest(probe),
                   'change': 'tclInt.h precedes explicit system includes, matching selected tclResult.c ingress; all remaining probe bytes are exact.',
                   'selected_sdk_includes': 'Actual expanded CC_SWITCHES in original SDK unix cwd, without an added compat search path.'},
               input_extension={'complete_selected_sdk_header_trees': header_trees,
                                'platform_compiler_include_trees': platform,
                                'prior_actual_maps_and_existing_inputs': maps},
               process_protocol={
                   'producer_receipt': 'Fsync actual streams and actual process receipt before every after-process verification.',
                   'verification': 'Strict required hashes, actual compile cwd, compiler header/link maps, own executable and runtime dependencies. No unpinned admission.',
                   'refusal': 'Separate persisted refusal/verification records preserve actual process outcomes and stop later processes.',
                   'output_reuse': 'Fresh614c output directory and independent own compilation only.'})
for name in ['probe.c', 'launch.py', 'prepare-request.py', 'source-windows.json', 'selected-makefile-builds.json']:
    pin(pins, BASE / name)
request['required_sha256'] = pins
for name in ['launch.py', 'prepare-request.py']:
    ast.parse((BASE / name).read_text(), filename=str(BASE / name))
raw = (json.dumps(request, indent=2) + '\n').encode()
(BASE / 'request.json').write_bytes(raw)
manifest = {'request': str(BASE / 'request.json'), 'request_sha256': digest(raw),
            'probe_sha256': digest(probe), 'launcher_sha256': sha(BASE / 'launch.py'),
            'required_pin_count': len(pins), 'selected_makefile_builds_sha256': digest(build_raw),
            'source_association_sha256': digest(join_raw),
            'source_only_preparation': True, 'compiler_native_or_assertion_launches': 0,
            'independent_review_required_before_Root_launch': True,
            'original_614_and_614b_bytes_unchanged': True}
(BASE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(manifest))
