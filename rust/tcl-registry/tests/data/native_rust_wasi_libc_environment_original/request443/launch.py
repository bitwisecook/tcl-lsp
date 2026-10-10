#!/usr/bin/env python3
"""Root-only Rust-sysroot WASIp1 libc capture; separate from the SDK C probe."""
from pathlib import Path
import datetime
import hashlib
import json
import os
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def pin(path):
    p = Path(path)
    return {'path': str(p), 'resolved_path': str(p.resolve()), 'bytes': p.stat().st_size, 'sha256': digest(p)}

def stamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def main():
    request_path = ROOT / 'request.json'
    request = json.loads(request_path.read_text())
    output = Path(sys.argv[1]) if len(sys.argv) == 2 else Path(request['capture_root'])
    if len(sys.argv) > 2:
        raise SystemExit('usage: launch.py [new capture directory]')
    if output.exists():
        raise SystemExit(f'refusing existing capture directory: {output}')
    if digest(__file__) != request['launcher_sha256']:
        raise SystemExit('changed launcher')
    if request['runner_flags'] != ['run', '-C', 'cache=n']:
        raise SystemExit('unexpected fixed runner flags')
    for item in request['input_pins']:
        if pin(item['path']) != item:
            raise SystemExit(f'changed input pin: {item["path"]}')
    library_dir = Path(request['target_libdir'])
    actual_inventory = [str(p) for p in sorted(library_dir.rglob('*')) if p.is_file()]
    if actual_inventory != [item['path'] for item in request['target_library_pins']]:
        raise SystemExit('changed full target library inventory')
    output.mkdir(parents=True)
    environment = os.environ.copy()
    environment.update(request['environment'])
    environment['GRAMMAR_WASI_PROBE_LINK_CAPTURE_DIR'] = str(output / 'linker')
    closure = {
        'request_path': str(request_path), 'request_sha256': digest(request_path),
        'launcher': pin(__file__), 'scope': request['scope'],
        'input_pins_before': request['input_pins'], 'started_utc': stamp(),
        'environment_delta': request['environment'], 'steps': [],
        'guest_observations': None, 'prior439_440': request['prior439_440'],
        'prior442': request['prior442'],
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
                process = subprocess.run(argv, cwd=output, env=environment, stdout=out, stderr=err, timeout=timeout)
                record.update(exit_code=process.returncode, timed_out=False)
            except subprocess.TimeoutExpired:
                record.update(exit_code=None, timed_out=True)
        record.update(finished_utc=stamp(), duration_seconds=time.monotonic()-started,
                      stdout=pin(stdout), stderr=pin(stderr))
        closure['steps'].append(record)
        save()
        return record
    compiler = request['compiler']
    runner = request['runner']
    preflights = [
        ('compiler-version', [compiler, '-vV']),
        ('compiler-sysroot', [compiler, '--print', 'sysroot']),
        ('compiler-target-libdir', [compiler, '--target', request['target'], '--print', 'target-libdir']),
        ('compiler-target-cfg', [compiler, '--target', request['target'], '--print', 'cfg']),
        ('linker-version', [request['linker'], '-flavor', 'wasm', '--version']),
        ('runner-version', [runner, '--version']),
    ]
    queried = [run(name, argv, 30) for name, argv in preflights]
    if any(item['exit_code'] != 0 for item in queried):
        closure['status'] = 'preflight-failed'
    elif ((output / 'compiler-sysroot.stdout').read_text().strip() != request['sysroot']
          or (output / 'compiler-target-libdir.stdout').read_text().strip() != request['target_libdir']):
        closure['status'] = 'queried-sysroot-mismatch'
    elif any(line not in (output / 'compiler-target-cfg.stdout').read_text().splitlines()
             for line in ['target_arch="wasm32"', 'target_os="wasi"', 'target_env="p1"', 'target_pointer_width="32"']):
        closure['status'] = 'queried-target-mismatch'
    elif request['expected_runner_version_fragment'] not in (output / 'runner-version.stdout').read_text():
        closure['status'] = 'runner-version-mismatch'
    else:
        module = output / 'probe.wasm'
        argv = [compiler] + request['compile_flags'] + [request['probe_path'], '-o', str(module)]
        compiled = run('compile', argv, 120)
        link_receipt = output / 'linker' / 'receipt.json'
        if link_receipt.exists():
            closure['linker_capture'] = {'receipt': pin(link_receipt),
                'original': json.loads(link_receipt.read_text())}
        if compiled['exit_code'] != 0:
            closure['status'] = 'compile-failed'
        elif not link_receipt.exists() or not closure['linker_capture']['original']['target_libraries_unchanged']:
            closure['status'] = 'link-closure-missing-or-changed'
        else:
            closure['module'] = pin(module)
            executed = run('execute', [runner] + request['runner_flags'] + [str(module)], 30)
            lines = (output / 'execute.stdout').read_text().splitlines()
            observed = {name: sum(line.startswith(name + '\t') for line in lines)
                        for name in request['expected_line_counts']}
            closure['guest_observations'] = observed
            closure['status'] = ('complete' if executed['exit_code'] == 0
                and observed == request['expected_line_counts'] else 'guest-or-shape-failed')
    closure['input_pins_after'] = [pin(item['path']) for item in request['input_pins']]
    closure['all_input_pins_unchanged'] = closure['input_pins_before'] == closure['input_pins_after']
    closure['target_library_inventory_after'] = [str(p) for p in sorted(library_dir.rglob('*')) if p.is_file()]
    closure['target_library_inventory_unchanged'] = actual_inventory == closure['target_library_inventory_after']
    closure['finished_utc'] = stamp()
    save()
    print(json.dumps({'capture': str(output), 'status': closure['status'],
                      'receipt_sha256': digest(output / 'receipt.json')}, indent=2))
    return 0 if (closure['status'] == 'complete' and closure['all_input_pins_unchanged']
                 and closure['target_library_inventory_unchanged']) else 1

if __name__ == '__main__':
    raise SystemExit(main())
