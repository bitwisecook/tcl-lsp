from pathlib import Path
from subprocess import run
import json,os
v=['8.4','8.5','8.6','9.0','9.1'];cases=['','.', '/', 'alpha','/dev/null','*','.*',r'\.*',r'alp\ha','{alpha,beta}','{alpha,alpha}','{}','{alpha,}','{dir/inner,beta}','nope/{','nope*/{','dir/{','d*/{','{','}','a[lp]*','dir/inner',r'dir\/inner','./alpha','dir//inner','alpha/','dir/','{{alpha,beta}}','foo{,x}']
rows=[]
tcl=Path('/tmp/bd-fs-case-command.tcl');tcl.write_text('set p [lindex $argv 0]\nset s [catch {glob -nocomplain -- $p} r]; if {$s} {puts "ERR $r"} else {foreach item [lsort $r] {binary scan $item H* hex; puts -nonewline " $hex"}; puts ""}\n')
cases+=['{x{a,b},beta}', '{a{l,p}pha,beta}', r'{a\,b,beta}', 'dir//inner', 'dir/',r'\.*','{','{{a}','a,b','a}b','{}','{{}}',r'\{alpha,beta\}','./alpha','./.','{alpha,alpha}{,}', '\x01a','\x02a','a[b]']
for i,release in [(0,'jim')]:
 for p in cases:
  proc=lambda cmd: run(cmd,cwd='/tmp/bd-fs-check',capture_output=True,text=True).stdout.rstrip('\n')
  orig=proc([os.environ['TCL_LSP_JIMSH'],str(tcl),p]);actual=proc(['/tmp/bd-fs-c-walk','jim',p]);rows.append([release,p,orig,actual])
for row in rows:
 if row[-2]!=row[-1]:print(*row,sep=' | ')
print('equal',sum(r[-2]==r[-1] for r in rows),'of',len(rows))
Path('/tmp/bd-fs-jim-matrix.json').write_text(json.dumps(rows))
