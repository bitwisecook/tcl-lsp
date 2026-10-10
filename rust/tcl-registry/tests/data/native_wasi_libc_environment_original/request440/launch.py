#!/usr/bin/env python3
"""Root-only launcher for fixed guest libc measurements; no Tcl providers."""
from pathlib import Path
import datetime
import hashlib
import json
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def stamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def main():
    request_path = ROOT / 'request.json'
    request = json.loads(request_path.read_text())
    output = Path(sys.argv[1]) if len(sys.argv) == 2 else Path(request['capture_root'])
    if len(sys.argv) > 2:
        raise SystemExit('usage: launch.py [new-capture-directory]')
    if output.exists():
        raise SystemExit(f'refusing existing capture directory: {output}')
    for item in request['input_pins']:
        if digest(item['path']) != item['sha256']:
            raise SystemExit(f'changed input pin: {item["path"]}')
    if digest(__file__) != request['launcher_sha256']:
        raise SystemExit('changed launcher pin')
    output.mkdir(parents=True)
    closure = {
        'request_path': str(request_path), 'request_sha256': digest(request_path),
        'launcher_path': str(Path(__file__).resolve()), 'launcher_sha256': digest(__file__),
        'scope': request['scope'], 'input_pins_before': request['input_pins'],
        'runner_flags': request['runner_flags'], 'original439': request['original439'],
        'started_utc': stamp(), 'steps': [], 'guest_observations': None,
    }
    def save():
        (output / 'receipt.json').write_text(json.dumps(closure, indent=2) + '\n')
    def run(name, argv, timeout):
        stdout = output / (name + '.stdout')
        stderr = output / (name + '.stderr')
        started = time.monotonic()
        record = {'stage': name, 'argv': argv, 'cwd': str(output), 'started_utc': stamp()}
        with stdout.open('wb') as out, stderr.open('wb') as err:
            try:
                process = subprocess.run(argv, cwd=output, stdout=out, stderr=err, timeout=timeout)
                record.update(exit_code=process.returncode, timed_out=False)
            except subprocess.TimeoutExpired:
                record.update(exit_code=None, timed_out=True)
        record.update(
            finished_utc=stamp(), duration_seconds=time.monotonic() - started,
            stdout={'path': str(stdout), 'bytes': stdout.stat().st_size, 'sha256': digest(stdout)},
            stderr={'path': str(stderr), 'bytes': stderr.stat().st_size, 'sha256': digest(stderr)},
        )
        closure['steps'].append(record)
        save()
        return record
    compiler = request['compiler']
    runner = request['runner']
    if request['runner_flags'] != ['run', '-C', 'cache=n']:
        raise SystemExit('unexpected fixed runner flags')
    compile_version = run('compiler-version', [compiler, '--target=wasm32-wasip1', '--version'], 30)
    runner_version = run('runner-version', [runner, '--version'], 30)
    if compile_version['exit_code'] != 0 or runner_version['exit_code'] != 0:
        closure['status'] = 'version-query-failed'
    elif (request['expected_compiler_version_fragment'] not in (output / 'compiler-version.stdout').read_text()
          or request['expected_runner_version_fragment'] not in (output / 'runner-version.stdout').read_text()):
        closure['status'] = 'version-mismatch'
    else:
        module = output / 'probe.wasm'
        argv = [compiler] + request['compile_flags'] + [request['probe_path'], '-o', str(module)]
        compiled = run('compile', argv, 120)
        if compiled['exit_code'] != 0:
            closure['status'] = 'compile-failed'
        else:
            closure['module'] = {'path': str(module), 'bytes': module.stat().st_size, 'sha256': digest(module)}
            executed = run('execute', [runner] + request['runner_flags'] + [str(module)], 30)
            lines = (output / 'execute.stdout').read_text().splitlines()
            observed = {name: sum(line.startswith(name + '\t') for line in lines)
                        for name in ['ABI', 'ERRNO_CELL', 'CASE', 'ROW', 'ADAPTER', 'SUMMARY']}
            closure['guest_observations'] = observed
            closure['status'] = ('complete' if executed['exit_code'] == 0
                and observed == request['expected_line_counts'] else 'guest-or-shape-failed')
    closure['input_pins_after'] = [dict(item, observed_sha256=digest(item['path']))
                                   for item in request['input_pins']]
    closure['all_input_pins_unchanged'] = all(item['sha256'] == item['observed_sha256']
                                             for item in closure['input_pins_after'])
    closure['finished_utc'] = stamp()
    save()
    print(json.dumps({'capture': str(output), 'status': closure['status'],
                      'receipt_sha256': digest(output / 'receipt.json')}, indent=2))
    return 0 if closure['status'] == 'complete' and closure['all_input_pins_unchanged'] else 1

if __name__ == '__main__':
    raise SystemExit(main())
