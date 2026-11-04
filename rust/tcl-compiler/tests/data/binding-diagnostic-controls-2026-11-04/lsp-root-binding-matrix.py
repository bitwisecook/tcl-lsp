from pathlib import Path
import subprocess,json,time,hashlib
cases={
 '2296-exact':'array set a {x 1}\nset k x\nputs $a($k)\n',
 '2296-unused-key':'array set a {x 1}\nset k x\nputs $a(x)\n',
 '2297-exact':'proc p {} {\nupvar 1 go go\nwhile {$go} {set go 0}\n}\nset go 1\np\n',
 '2297-unread-local':'proc p {} {set spare 1; puts ok}\np\n',
 '2247-qualified':'set d {a 1}\n::tcl::dict::with d {puts $a}\n',
 '2247-public':'set d {a 1}\ndict with d {puts $a}\n',
 '2247-unknown-head':'set d {a 1}\n::tcl::dict::missing d {puts $a}\n',
 '2247-unread-local':'proc p {} {set spare 1; set d {a 1}; ::tcl::dict::with d {puts $a}}\np\n'
}
root=Path('/tmp/lsp-root-binding-matrix');root.mkdir(exist_ok=True)
rows=[]
for label,source in cases.items():
 (root/(label+'.tcl')).write_text(source)
 for profile in ['tcl8.4','tcl8.5','tcl8.6','tcl9.0','tcl9.1','jim']:
  if label.startswith('2247-') and profile in ['tcl8.4','tcl8.5','jim']:continue
  start=time.monotonic()
  try:
   p=subprocess.run(['/tmp/lsp-root-binding-probe',profile],input=source,text=True,capture_output=True,timeout=30)
   row={'case':label,'profile':profile,'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr,'seconds':time.monotonic()-start}
  except subprocess.TimeoutExpired as e:row={'case':label,'profile':profile,'timeout':True,'stdout':str(e.stdout),'stderr':str(e.stderr)}
  rows.append(row)
  (root/'rows.json').write_text(json.dumps(rows,indent=2)+'\n')
  print(json.dumps({'case':label,'profile':profile,'exit':row.get('exit'),'warnings':[line for line in row['stdout'].splitlines() if line.startswith(('A W','C W','A I230','C I230'))]}),flush=True)
