import datetime, hashlib, json, os, signal, subprocess, sys, time
from pathlib import Path
runner=Path('/tmp/compiler-rbs-existing-trace-v1/run.sh')
watcher=Path('/tmp/compiler-rbs-existing-trace-v1/runtime-watchdog.py')
plan=Path('/tmp/compiler-rbs-existing-trace-v1/plan.json')
pins={str(runner):'00908167cf415e1864842db85cf8207598dbc6c0e3d32fb9697b055935480ee0',str(watcher):'46d6f2dc2abd95a6286caa748b411ae4c2b7783b71f190deefba1878abfb12b1',str(plan):'63b08ace8660f7fd233c7e20f0d8e6b5d461d0d9cbd2241146a6335cb6c25dbc'}
def hashes():return {p:hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in pins}
assert hashes()==pins
base=Path('/tmp/compiler-rbs-existing-trace'); base.mkdir(exist_ok=True); existing=set(base.iterdir())
started=datetime.datetime.now(datetime.timezone.utc).isoformat()
process=subprocess.Popen(['bash',str(runner),str(plan)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,bufsize=1)
receipt=None
until=time.monotonic()+15
while time.monotonic()<until:
    new=set(base.iterdir())-existing
    if len(new)==1:
        receipt=new.pop();break
    if len(new)>1 or process.poll() is not None:break
    time.sleep(0.025)
if receipt is None:
    process.send_signal(signal.SIGINT)
    raise SystemExit('STOP: could not identify this suite new receipt before startup')
observer=subprocess.Popen([sys.executable,str(watcher),str(process.pid),str(receipt)])
launch={'started_utc':started,'suite_pid':process.pid,'observer_pid':observer.pid,'pins_before':pins,'receipt':str(receipt),'timer_seconds':180,'runtime_scope':'whole selected executable, excluding compilation','root_authorization':'Prepared exactRBS existingtrace only; explicit root GO required'}
(receipt/'runtime-watchdog-launch.json').write_text(json.dumps(launch,indent=2)+'\n')
print('WATCHDOG_ATTACHED',json.dumps(launch),flush=True)
for line in process.stdout:print(line,end='',flush=True)
exit_code=process.wait()
try:observer_exit=observer.wait(timeout=5)
except subprocess.TimeoutExpired:
    observer.send_signal(signal.SIGINT)
    observer_exit=observer.wait(timeout=5)
watch=json.loads((receipt/'runtime-watchdog.json').read_text()) if (receipt/'runtime-watchdog.json').exists() else {}
invalid=(observer_exit!=0 or not watch or watch.get('watchdog_failure') or any(r.get('timed_out') or r.get('acceptance_invalid') for r in watch.get('records',[])))
launch.update({'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'suite_exit':exit_code,'observer_exit':observer_exit,'pins_after':hashes(),'runtime_acceptance_invalid':bool(invalid)})
(receipt/'runtime-watchdog-launch.json').write_text(json.dumps(launch,indent=2)+'\n')
print('WATCHDOG_CLOSED',json.dumps(launch),flush=True)
raise SystemExit(exit_code or (2 if invalid or launch['pins_after']!=pins else 0))
