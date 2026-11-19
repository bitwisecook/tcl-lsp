from pathlib import Path
from subprocess import run
import json
v=['8.4','8.5','8.6','9.0','9.1'];cases=['','.', '/', 'alpha','/dev/null','*','.*',r'\.*',r'alp\ha','{alpha,beta}','{alpha,alpha}','{}','{alpha,}','{dir/inner,beta}','nope/{','nope*/{','dir/{','d*/{','{','}','a[lp]*','dir/inner',r'dir\/inner','./alpha','dir//inner','alpha/','dir/','{{alpha,beta}}','foo{,x}']
rows=[]
tcl=Path('/tmp/bd-fs-case-command.tcl');tcl.write_text('set p [lindex $argv 0]\nset s [catch {glob -nocomplain -- $p} r]; if {$s} {puts "ERR $r"} else {foreach item [lsort $r] {binary scan $item H* hex; puts -nonewline " $hex"}; puts ""}\n')
for i,release in enumerate(v):
 for p in cases:
  proc=lambda cmd: run(cmd,cwd='/tmp/bd-fs-check',capture_output=True,text=True).stdout.rstrip('\n')
  orig=proc(['tclsh'+release,str(tcl),p]);actual=proc(['/tmp/bd-fs-c-walk',str(i),p]);rows.append([release,p,orig,actual])
for row in rows:
 if row[-2]!=row[-1]:print(*row,sep=' | ')
print('equal',sum(r[-2]==r[-1] for r in rows),'of',len(rows))
Path('/tmp/bd-fs-c-matrix.json').write_text(json.dumps(rows))
