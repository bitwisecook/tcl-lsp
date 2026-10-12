import hashlib,json,os,subprocess,tempfile
from pathlib import Path
here=Path(__file__).resolve().parent
root=here.parents[4]
tree=root/'tmp/tcl8.6.18'
runner=root/'tmp/name-target-runtime/debug/examples/run_script'
out=here/'capture-02';out.mkdir(exist_ok=False)
env=dict(os.environb)
for k in [b'NH_CLOCK_GETENV_MISSING',b'TCL_BACKEND_CONSTRAINTS',b'TCLLIBPATH',b'TCL_TEST_VERBOSE',b'TCL_TEST_CONSTRAINTS']: env.pop(k,None)
env.update({b'TCL_LIBRARY':str(tree/'library').encode(),b'NH_CLOCK_GETENV_TEST':b'host-value',b'NH_CLOCK_GETENV_\xff':b'raw-name-value',b'NH_CLOCK_GETENV_RAW_VALUE':b'\xff'})
base=f'source {{{tree}/library/tcltest/tcltest.tcl}}\nsource {{{tree}/library/clock.tcl}}\n'
commands=['::tcl::clock::format','::tcl::clock::GetSystemTimeZone','::tcl::clock::ReadZoneinfoFile','::tcl::clock::SetupTimeZone','binary','string','read']
for i,cmd in enumerate(commands):
 for event in ['enter','leave']:
  base+='trace add execution '+cmd+' '+event+' {apply {{args} {puts stderr Q'+str(i)+'_'+event+'}}}\n'
base+='set argv [list -singleproc 1 -file var.test -relateddir {} -verbose {body error start skip}]\n'
base+=f'source {{{tree}/tests/all.tcl}}\n'
script=out/'fixed-label-traces.tcl';script.write_text(base)
records=[]
for name,cmd in [('c',[str(tree/'unix/tclsh')]),('native',[str(runner),'--quiet','--tcl-version','8.6','--init'])]:
 with tempfile.TemporaryDirectory(prefix='unicode-byte3-') as cwd:
  dest=out/name;dest.mkdir()
  with (dest/'stdout.txt').open('wb') as stdout,(dest/'stderr.txt').open('wb') as stderr:
   run=subprocess.run(['timeout','--kill-after=5','60',*cmd,str(script)],cwd=cwd,env=env,stdout=stdout,stderr=stderr,timeout=67)
  records.append({'engine':name,'argv':[*cmd,str(script)],'exit_code':run.returncode})
paths=[runner,tree/'unix/tclsh',tree/'library/clock.tcl',tree/'library/tcltest/tcltest.tcl',tree/'tests/all.tcl',here/'probe.py',script]
(out/'provenance.json').write_text(json.dumps({'records':records,'sha256':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},'trace_labels':dict(enumerate(commands)),'environment':'Same named benign overrides/removals as getter capture; inherited host values are neither enumerated nor logged.'},indent=2)+'\n')
print('retained',out)
print('native tail', (out/'native/stderr.txt').read_text().splitlines()[-30:])
