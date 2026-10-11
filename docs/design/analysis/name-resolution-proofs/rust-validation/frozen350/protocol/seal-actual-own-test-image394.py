from pathlib import Path
import hashlib, json, os, stat, sys

snapshot, label, package, target, short = sys.argv[1:]
base = Path('/workspace/.proofs')
receipt = base / (label + '.json')
r = json.loads(receipt.read_text())
assert r['exit'] == 0 and r['uniform_source'] and not r['summaries']
artifacts = []
for line in (base / (label + '.log')).read_text().splitlines():
    if not line.startswith('{'):
        continue
    m = json.loads(line)
    if m.get('reason') == 'compiler-artifact' and m['target']['name'] == target and m['profile']['test'] and m.get('executable'):
        assert package in m['package_id']
        artifacts.append(m)
assert len(artifacts) == 1, len(artifacts)
artifact = artifacts[0]
original = Path(artifact['executable'])
assert original.is_file()
original_stat = original.stat()

def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()

original_sha = digest(original)
owned = Path('/workspace/.pins/naming-owned' + snapshot)
owned.mkdir(exist_ok=True)
sealed = owned / ('pinned-' + short + snapshot + '.elf')
assert not sealed.exists() and original_stat.st_dev == owned.stat().st_dev
os.rename(original, sealed)
os.chmod(sealed, 0o700)
assert digest(sealed) == original_sha and sealed.stat().st_ino == original_stat.st_ino
snapshot_path = Path('/workspace/.checkpoints/naming' + snapshot + '/snapshot.json')
record = {'executable': str(sealed), 'sha256': original_sha, 'size': sealed.stat().st_size,
          'package': package, 'target': artifact['target'], 'features': artifact['features'],
          'cargo_artifact': artifact, 'original_path': str(original),
          'original_stat': {k: getattr(original_stat, k) for k in ['st_dev','st_ino','st_mode','st_size','st_nlink','st_mtime_ns','st_ctime_ns']},
          'sealed_stat': {k: getattr(sealed.stat(), k) for k in ['st_dev','st_ino','st_mode','st_size','st_nlink','st_mtime_ns','st_ctime_ns']},
          'build_receipt': str(receipt), 'build_receipt_sha256': digest(receipt),
          'source_snapshot': str(snapshot_path), 'source_snapshot_sha256': digest(snapshot_path),
          'storage_rule': 'The exact own-target Cargo-emitted ELF is sealed by same-filesystem rename retaining its inode and all full bytes. Permission mode is set to 0700. No dependency executable or stale/list-only target supplies ownership or assertion credit.'}
out = base / ('pinned-' + short + snapshot + '.json')
assert not out.exists()
out.write_text(json.dumps(record, indent=2) + '\n')
print('Sealed actual own-target image', sealed, original_sha, record['size'])
