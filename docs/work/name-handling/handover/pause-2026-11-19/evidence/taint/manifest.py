from pathlib import Path
import sys
workspace=Path('/workspace/tcl-lsp')
trees=['tcl8.4.20/library','tcl8.5.19/library','tcl8.6.18/library','tcl9.0.4/library','tcl9.1.0/library','tcllib-2.0/modules']
entries=[]
for root in trees:
 for p in sorted((workspace/'tmp'/root).rglob('*.tcl')):
  entries.append(('tcl8.6',str(p)))
for root in ('samples','tests','rust','experiments/corpus/adversarial'):
 for p in sorted((workspace/root).rglob('*')):
  if p.suffix.lower() not in {'.tcl','.test','.tm','.irul','.irule','.bpftcl','.apl','.sslictcl','.f5q','.conf'}:continue
  if root=='rust' and '/tests/' not in str(p) and '/testdata/' not in str(p):continue
  d=({'.irul':'f5-irules','.irule':'f5-irules','.apl':'apl-regions','.bpftcl':'bpf','.sslictcl':'declarative-sslic','.f5q':'query-dsl','.conf':'scf-regions'}).get(p.suffix,'tcl8.6')
  entries.append((d,str(p)))
out=Path('/tmp/bd-taint-ab/inputs.tsv')
out.write_text(''.join(d+'\t'+p+'\n' for d,p in entries))
print(len(entries),out)
