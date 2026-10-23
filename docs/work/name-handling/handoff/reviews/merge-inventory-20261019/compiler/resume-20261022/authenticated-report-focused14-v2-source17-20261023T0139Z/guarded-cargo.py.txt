import datetime
import json
import os
from pathlib import Path
import re
import selectors
import shutil
import signal
import subprocess
import sys
import time

log = Path(sys.argv[1])
argv = sys.argv[2:]
pattern = re.compile(rb'^(test |running |test result:|error|warning|\s*(?:-->|Running |Finished |Compiling )|thread |failures:|ZZ_|SRC |  (?:IR|FU)|    BINDING|HOSTED_|ORIGINAL_|NATIVE_|G4_|N13_POINT|N13_CHILD_ENTRY|N17G|REPORT_)')
cap = 20 * 1024 * 1024
used = context = 0
pending = b''
minimum_free = shutil.disk_usage('/workspace').free
aborted = False
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
process = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
selector = selectors.DefaultSelector()
selector.register(process.stdout, selectors.EVENT_READ)
next_disk = 0.0
with log.open('wb') as output:
    def write_line(line):
        global used, context
        keep = bool(pattern.search(line)) or context > 0
        context = max(context - 1, 0)
        if line.startswith((b'error', b'thread ')):
            context = 8
        if keep and used < cap:
            data = line[:cap-used]
            output.write(data)
            used += len(data)
    while selector.get_map():
        now = time.monotonic()
        if now >= next_disk:
            free = shutil.disk_usage('/workspace').free
            minimum_free = min(minimum_free, free)
            next_disk = now + 5.0
            if free < 2 * 1024**3 and not aborted:
                aborted = True
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGINT)
                write_line(b'error: compiler run stopped because free disk fell below 2 GiB\n')
        for key, _ in selector.select(1.0):
            data = os.read(key.fd, 65536)
            if not data:
                selector.unregister(key.fileobj)
                continue
            pending += data
            while b'\n' in pending:
                line, pending = pending.split(b'\n', 1)
                write_line(line+b'\n')
            # A malformed diagnostic cannot buffer an unbounded single line.
            if len(pending) > 262144:
                write_line(pending[:262144]+b' [long line truncated]\n')
                pending = b''
    if pending:
        write_line(pending+b'\n')
exit_code = process.wait()
receipt = {
    'argv': argv,
    'started_utc': started,
    'finished_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'cargo_exit': exit_code,
    'minimum_free_bytes': minimum_free,
    'disk_abort': aborted,
    'log_cap_bytes': cap,
    'retained_log_bytes': used,
    'log_at_cap': used == cap,
    'disk_poll_seconds': 5,
    'termination_scope': 'This invocation own Cargo session/process group only',
}
Path(str(log)+'.process.json').write_text(json.dumps(receipt, indent=2)+'\n')
sys.exit(2 if aborted else (exit_code if exit_code >= 0 else 128-exit_code))
