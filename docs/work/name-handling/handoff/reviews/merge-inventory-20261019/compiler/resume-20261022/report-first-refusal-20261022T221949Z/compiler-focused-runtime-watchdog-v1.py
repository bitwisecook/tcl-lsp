"""External, source-preserving 180-second runtime guard for one frozen suite."""
import datetime
import json
import os
from pathlib import Path
import signal
import sys
import time

suite_pid = int(sys.argv[1])
receipt = Path(sys.argv[2]).resolve()
budget = 180.0
hz = os.sysconf('SC_CLK_TCK')
records = []
seen = set()

def utc():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def info(pid):
    try:
        path = Path('/proc') / str(pid)
        stat = (path / 'stat').read_text().rsplit(')', 1)[1].split()
        argv = [x.decode(errors='replace') for x in (path / 'cmdline').read_bytes().split(b'\0') if x]
        executable = Path(argv[0]) if argv else Path('unavailable')
        return {'pid': pid, 'state': stat[0], 'ppid': int(stat[1]),
                'start': int(stat[19]) / hz,
                'argv': argv, 'exe': executable.name, 'exe_path': str(executable)}
    except (OSError, ValueError):
        return None

def children(pid):
    found = []
    for path in Path('/proc').iterdir():
        if path.name.isdigit():
            item = info(int(path.name))
            if item and item['ppid'] == pid:
                found.append(item)
    return found

suite = info(suite_pid)
assert suite and len(suite['argv']) > 1 and suite['argv'][1] == '/tmp/compiler-focused-suite.py'
suite_start = suite['start']
output = receipt / 'runtime-watchdog.json'
started = utc()
attached_uptime = float(Path('/proc/uptime').read_text().split()[0])
while True:
    current = info(suite_pid)
    if not current or current['state'] == 'Z' or current['start'] != suite_start:
        break
    for guard in children(suite_pid):
        args = guard['argv']
        if len(args) < 4 or args[1] != '/tmp/compiler-focused-guarded-cargo.py':
            continue
        log = Path(args[2]).resolve()
        if log.parent != receipt:
            continue
        for cargo in children(guard['pid']):
            if cargo['exe'] != 'cargo' or cargo['argv'][1:3] != ['test', '--locked']:
                continue
            for test in children(cargo['pid']):
                if not test['exe'].startswith('tcl_compiler-') or '/debug/deps/' not in test['exe_path']:
                    continue
                key = (test['pid'], test['start'])
                if key in seen:
                    continue
                seen.add(key)
                row = {'log': str(log), 'test_pid': test['pid'], 'cargo_pid': cargo['pid'],
                       'observed_utc': utc(), 'start_uptime_seconds': test['start'],
                       'exe_path': test['exe_path'], 'exe_device': Path(test['exe_path']).stat().st_dev,
                       'exe_inode': Path(test['exe_path']).stat().st_ino, 'cargo_start_uptime_seconds': cargo['start'],
                       'guard_pid': guard['pid'], 'suite_start_uptime_seconds': suite_start,
                       'budget_seconds': budget, 'budget_scope': 'entire selected test process; compilation excluded',
                       'signals': [], 'runtime_timer_attached_before_test_start':
                       test['start'] >= attached_uptime}
                records.append(row)
                while True:
                    live = info(test['pid'])
                    elapsed = float(Path('/proc/uptime').read_text().split()[0]) - test['start']
                    if not live or live['state'] == 'Z' or live['start'] != test['start']:
                        row['finished_observed_utc'] = utc()
                        row['runtime_observed_seconds'] = elapsed
                        break
                    if elapsed >= budget:
                        # Verify the originally owned Cargo session still exists before signalling it.
                        owner = info(cargo['pid'])
                        if owner and owner['start'] == cargo['start'] and os.getpgid(cargo['pid']) == cargo['pid']:
                            signals = [signal.SIGINT, signal.SIGTERM, signal.SIGKILL]
                            index = len(row['signals'])
                            if index < 3 and elapsed >= budget + index * 10:
                                os.killpg(cargo['pid'], signals[index])
                                row['signals'].append({'signal': signals[index].name, 'utc': utc(), 'elapsed_seconds': elapsed})
                    output.write_text(json.dumps({'started_utc': started, 'suite_pid': suite_pid, 'records': records}, indent=2)+'\n')
                    time.sleep(0.25)
    output.write_text(json.dumps({'started_utc': started, 'suite_pid': suite_pid, 'records': records}, indent=2)+'\n')
    time.sleep(0.25)
output.write_text(json.dumps({'started_utc': started, 'finished_utc': utc(), 'suite_pid': suite_pid, 'records': records}, indent=2)+'\n')
