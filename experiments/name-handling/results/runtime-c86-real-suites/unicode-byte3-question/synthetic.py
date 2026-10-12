import hashlib,json,os,subprocess,tempfile
from pathlib import Path
here=Path(__file__).resolve().parent;root=here.parents[4];tree=root/'tmp/tcl8.6.18';runner=root/'tmp/name-target-runtime/debug/examples/run_script'
out=here/'capture-05';out.mkdir(exist_ok=False)
cases={'scan-nul': 'set b [binary format H* 61626300]; binary scan $b a4 s\n', 'bytearray-nul':'set s [binary format H* 61626300]\n','scan-ascii':'set b [binary format H* 616263]; binary scan $b a3 s\n'}
env=dict(os.environb);env[b'TCL_LIBRARY']=str(tree/'library').encode()
records=[]
for name,head in cases.items():
 script=out/(name+'.tcl');script.write_text(head+'puts [list SPLIT_COUNT [llength [split $s \\0]]]\n')
 for engine,cmd in [('c',[str(tree/'unix/tclsh')]),('native',[str(runner),'--quiet','--tcl-version','8.6','--init'])]:
  dest=out/(engine+'-'+name);dest.mkdir()
  with tempfile.TemporaryDirectory(prefix='unicode-synthetic-') as cwd,(dest/'stdout.txt').open('wb') as stdout,(dest/'stderr.txt').open('wb') as stderr:
   run=subprocess.run(['timeout','--kill-after=5','60',*cmd,str(script)],env=env,cwd=cwd,stdout=stdout,stderr=stderr,timeout=67)
  records.append({'case':name,'engine':engine,'argv':[*cmd,str(script)],'exit_code':run.returncode})
paths=[runner,tree/'unix/tclsh',here/'synthetic.py',root/'rust/tcl-syntax/src/raw_string.rs',root/'runtime/rust/src/value_ops.rs',root/'rust/tcl-cmd-core/src/list.rs',*out.glob('*.tcl')]
(out/'provenance.json').write_text(json.dumps({'records':records,'sha256':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},'scope':'Benign exact 61626300/616263 data only; C86 and retained standalone Native C86 runner. No host values queried or printed.'},indent=2)+'\n')
for row in records:
 dest=out/(row['engine']+'-'+row['case']);print(row['engine'],row['case'],row['exit_code'],(dest/'stdout.txt').read_text().strip(),(dest/'stderr.txt').read_text().strip())
