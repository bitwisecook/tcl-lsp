from pathlib import Path
import hashlib, json, os, re, subprocess, sys, time

p = Path('/workspace/.proofs')
s = Path('/workspace/.checkpoints/naming350')
label = sys.argv[1]
command = sys.argv[2:]
assert command and not (p / (label + '.json')).exists()
source = json.loads((s / 'snapshot.json').read_text())
root = Path(source['compile_source_root'])
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
assert all(sha(s / n) == h and sha(root / n) == h for n, h in source['source'].items())
env = {**os.environ, **json.loads((p / 'native-test-env.json').read_text())}
started = time.monotonic()
log_path = p / (label + '.log')
with log_path.open('wb') as log:
    result = subprocess.run(command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
text = log_path.read_text(errors='replace')
record = {'label': label, 'command': command, 'exit': result.returncode,
          'seconds': time.monotonic() - started, 'source': source,
          'uniform_source': all(sha(s / n) == h and sha(root / n) == h for n, h in source['source'].items()),
          'summaries': re.findall(r'(?m)^test result: (?:ok|FAILED)\.[^\n]*', text),
          'log_sha256': sha(log_path)}
(p / (label + '.json')).write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps({k: v for k, v in record.items() if k not in {'source', 'command'}}))
print('\n'.join(line[:300] for line in text.splitlines()[-12:] if not line.startswith('{"reason":')))
sys.exit(result.returncode or not record['uniform_source'])
