from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import hashlib, json, os, re, subprocess, sys, time

p = Path('/workspace/.proofs')
a_path = p / 'runtime-focused-consumer-admission349.json'
a = json.loads(a_path.read_text())
own = a['owned_image_record']
image = Path(own['executable'])
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
assert sha(image) == own['sha256']
assert a['count'] == len(a['selectors']) == 454
assert own['target']['name'] == 'tcl_runtime'
assert own['target']['kind'] == ['rlib', 'cdylib', 'staticlib']
assert own['features'] == ['engine']
out = p / 'runtime-focused-isolated349'
assert not out.exists()
out.mkdir()
env = {**os.environ, **json.loads((p / 'native-test-env.json').read_text())}
cwd = '/workspace/.checkpoints/naming-current-validation323'

def run(row):
    ordinal, selector = row
    args = [str(image), '--exact', selector, '--test-threads=1', '--nocapture']
    log_path = out / f'{ordinal:03}.log'
    started = time.monotonic()
    with log_path.open('wb') as log:
        result = subprocess.run(args, cwd=cwd, env=env, stdout=log, stderr=subprocess.STDOUT)
    text = log_path.read_text(errors='replace')
    summaries = re.findall(r'(?m)^test result: (?:ok|FAILED)\.[^\n]*', text)
    passed = result.returncode == 0 and len(summaries) == 1 and '1 passed; 0 failed; 0 ignored' in summaries[0]
    record = {'ordinal': ordinal, 'selector': selector, 'argv': args,
              'exit': result.returncode, 'seconds': time.monotonic() - started,
              'passed_exact_assertion': passed, 'summaries': summaries,
              'log': str(log_path), 'log_sha256': sha(log_path)}
    (out / f'{ordinal:03}.json').write_text(json.dumps(record, indent=2) + '\n')
    if not passed:
        print(json.dumps({'ordinal': ordinal, 'selector': selector, 'exit': result.returncode,
                          'summaries': summaries}), flush=True)
    return record

started = time.monotonic()
with ThreadPoolExecutor(max_workers=4) as pool:
    rows = list(pool.map(run, enumerate(a['selectors'])))
assert sha(image) == own['sha256']
record = {'status': 'Closed actual Runtime349 exact isolated image assertions',
          'admission': str(a_path), 'admission_sha256': sha(a_path),
          'owned_image': str(image), 'owned_image_sha256': own['sha256'],
          'selectors': len(rows), 'passed': sum(r['passed_exact_assertion'] for r in rows),
          'failed_or_aborted': sum(not r['passed_exact_assertion'] for r in rows),
          'seconds': time.monotonic() - started, 'rows': rows,
          'limits': 'Each actual selector executes unchanged in its own process on the actual engine/have_tommath owned image. This diagnostic run does not replace or conceal the grouped run abort, nor admit the three cfg(not(have_tommath)) assertions, nor grant any assertion to another source image. Outer run349 records the exact original immutable349/current compile-root source digest bookends.'}
(out / 'results.json').write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps({k: v for k, v in record.items() if k not in {'rows', 'limits'}}), flush=True)
sys.exit(0 if record['failed_or_aborted'] == 0 else 1)
