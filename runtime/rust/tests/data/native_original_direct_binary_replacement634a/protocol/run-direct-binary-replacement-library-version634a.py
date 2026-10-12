from pathlib import Path
import concurrent.futures
import hashlib
import json
import shutil
import subprocess
import time

root = Path('/workspace/.proofs/root-native-direct-binary-replacement634a')
source = Path('/workspace/.proofs/native-string-replacement-library-version629.c')
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
shutil.copyfile(source, root / 'library-version.c')
source = root / 'library-version.c'

def provider(name):
    prior = json.loads((root / name / 'receipt.json').read_text())
    folder = root / name / 'same-library-version'
    assert not folder.exists()
    folder.mkdir()
    pins = dict(prior['inputs_before'])
    pins[str(source)] = sha(source)
    assert all(sha(Path(p)) == h for p, h in pins.items())
    argv = list(prior['compile']['argv'])
    argv[argv.index(str(root / 'probe.c'))] = str(source)
    argv[-1] = str(folder / 'version.elf')
    env = prior['compile']['environment']
    records = []
    for label, command in [('compile', argv), ('execute', [argv[-1]])]:
        started = time.monotonic()
        result = subprocess.run(command, cwd=folder, env=env, capture_output=True, timeout=60)
        (folder / (label + '.stdout')).write_bytes(result.stdout)
        (folder / (label + '.stderr')).write_bytes(result.stderr)
        records.append({'argv': command, 'cwd': str(folder), 'environment': env,
                        'exit': result.returncode, 'seconds': time.monotonic() - started,
                        'stdout_sha256': sha(folder / (label + '.stdout')),
                        'stderr_sha256': sha(folder / (label + '.stderr')),
                        'executable_sha256': sha(folder / 'version.elf') if label == 'execute' else None})
        assert result.returncode == 0
    unchanged = all(sha(Path(p)) == h for p, h in pins.items())
    assert unchanged
    receipt = {'provider': name, 'original_question': prior['proof_id'],
               'original_provider_receipt_sha256': sha(root / name / 'receipt.json'),
               'inputs_before': pins, 'all_after_pins_match': unchanged, 'operations': records,
               'scope': 'Independent diagnostic process compiled against the identical actual static library and headers as the original replacement probe. It directly queries that library patchlevel; this diagnostic executes after the original cases and changes none of their original receipts or snapshots. CLI patchlevel has its separate earlier capture.'}
    (folder / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(name, (folder / 'execute.stdout').read_text().strip().replace('\n', ' | '), flush=True)
    return {'provider': name, 'receipt_sha256': sha(folder / 'receipt.json'), 'all_after_pins_match': unchanged}

with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
    rows = list(pool.map(provider, ['9.0.4', '9.1.0']))
(root / 'library-version-results.json').write_text(json.dumps({'providers': rows,
     'scope': 'Same-library independent diagnostic observations, not inserted into the original replacement processes.'}, indent=2) + '\n')
