"""Disk-bounded CI-equivalent inventory: build, run and delete one test target at a time.

usage: inventory.py OUTDIR [PACKAGE ...]
Writes OUTDIR/<pkg>__<kind>__<name>.xml (JUnit) and OUTDIR/summary.jsonl.
"""
from pathlib import Path
import json, os, shutil, subprocess, sys, threading, time

R = Path('/home/user/tcl-lsp')
OUT = Path(sys.argv[1]).resolve(); OUT.mkdir(parents=True, exist_ok=True)
ONLY = set(sys.argv[2:])
DEPS = R / 'target/debug/deps'
CFG = OUT / 'nextest.toml'
CFG.write_text((R / '.config/nextest.toml').read_text() + '''
[profile.inventory]
fail-fast = false
slow-timeout = { period = "60s", terminate-after = 3 }
leak-timeout = "1s"
status-level = "fail"
final-status-level = "fail"
success-output = "never"
failure-output = "immediate"
[profile.inventory.junit]
path = "junit.xml"
store-success-output = false
store-failure-output = false
''')
ENV = dict(os.environ, CARGO_INCREMENTAL='0', TCL_LSP_REQUIRE_TCLSH='9.0,8.6',
           TCL_LSP_JIMSH=str(R / 'tmp/jimtcl-oracle/jimsh'))
DONE = {json.loads(l)['id'] for l in (OUT / 'summary.jsonl').open()} if (OUT / 'summary.jsonl').exists() else set()


def descendants(pid):
    out, todo = [], [pid]
    while todo:
        p = todo.pop()
        try:
            kids = Path(f'/proc/{p}/task/{p}/children').read_text().split()
        except OSError:
            continue
        out += [int(k) for k in kids]; todo += [int(k) for k in kids]
    return out


def rss_kb(pid):
    try:
        for l in Path(f'/proc/{pid}/status').read_text().splitlines():
            if l.startswith('VmRSS:'):
                return int(l.split()[1])
    except OSError:
        pass
    return 0


def watchdog(proc, kills, stop):
    while not stop.is_set():
        time.sleep(2)
        pids = descendants(proc.pid)
        nx = [p for p in pids if 'nextest' in Path(f'/proc/{p}/cmdline').read_bytes().decode(errors='replace')] if pids else []
        if not nx or max(rss_kb(p) for p in nx) < 3 * 2**20:
            continue
        tests = []
        for p in pids:
            try:
                cmd = Path(f'/proc/{p}/cmdline').read_bytes().split(b'\0')
                io = Path(f'/proc/{p}/io').read_text()
            except OSError:
                continue
            if b'--exact' in cmd:
                w = int(next(l.split()[1] for l in io.splitlines() if l.startswith('wchar')))
                tests.append((w, p, cmd[cmd.index(b'--exact') + 1].decode()))
        if tests:
            w, p, name = max(tests)
            kills.append({'test': name, 'wchar': w})
            try:
                os.kill(p, 9)
            except OSError:
                pass
            time.sleep(5)


def free_gb():
    s = os.statvfs(R); return s.f_bavail * s.f_frsize / 2**30


def executables():
    return {p for p in DEPS.iterdir() if p.is_file() and os.access(p, os.X_OK) and '.' not in p.name} if DEPS.exists() else set()


meta = json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version', '1'], cwd=R))
jobs = []
for p in sorted(meta['packages'], key=lambda p: p['name']):
    if ONLY and p['name'] not in ONLY:
        continue
    for t in p['targets']:
        k = t['kind'][0]
        sel = {'lib': ['--lib'], 'proc-macro': ['--lib'], 'bin': ['--bin', t['name']], 'test': ['--test', t['name']]}.get(k)
        if sel and t.get('test', True):
            jobs.append((p['name'], k, t['name'], sel))

for pkg, kind, name, sel in jobs:
    jid = f'{pkg}__{kind}__{name}'
    if jid in DONE:
        continue
    if free_gb() < 2.5:
        subprocess.run(['cargo', 'clean', '--profile', 'test'], cwd=R, capture_output=True)
        subprocess.run(['cargo', 'clean'], cwd=R, capture_output=True)
    before = executables()
    t0 = time.time()
    cmd = ['cargo', 'nextest', 'run', '--config-file', str(CFG), '--profile', 'inventory', '--no-fail-fast',
           '-p', pkg, '--all-features', *sel]
    log = OUT / f'{jid}.log'
    with log.open('w') as lf:
        proc = subprocess.Popen(cmd, cwd=R, env=ENV, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                                text=True, errors='replace')
        written = 0
        kills, stop = [], threading.Event()
        threading.Thread(target=watchdog, args=(proc, kills, stop), daemon=True).start()
        for line in proc.stderr:
            if written < 200 * 2**20:
                line = line if len(line) <= 2000 else line[:2000] + ' ...[truncated]\n'
                lf.write(line); written += len(line)
        rc = proc.wait()
        stop.set()
    junit = R / 'target/nextest/inventory/junit.xml'
    if junit.exists():
        shutil.move(junit, OUT / f'{jid}.xml')
    for e in executables() - before:
        e.unlink(missing_ok=True)
    rec = {'id': jid, 'pkg': pkg, 'kind': kind, 'name': name, 'rc': rc,
           'secs': round(time.time() - t0, 1), 'free_gb': round(free_gb(), 2),
           'junit': (OUT / f'{jid}.xml').exists(), 'watchdog_kills': kills}
    with (OUT / 'summary.jsonl').open('a') as f:
        f.write(json.dumps(rec) + '\n')
    print(json.dumps(rec), flush=True)
