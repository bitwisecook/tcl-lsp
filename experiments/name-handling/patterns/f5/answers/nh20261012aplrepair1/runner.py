#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Prepare/remove a real APL form measuring native capability boundaries."""
import argparse,datetime,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[6]
sys.path.insert(0,str(ROOT/'scripts/dev/bigip-probes/name-patterns'))
import capture
RUN='nh20261012aplrepair1'; DEST=Path(__file__).resolve().parent; JOB=DEST/'iapp-apl-0'
NAME='/Common/__tcl_lsp_probe_'+RUN; REMOTE='/var/tmp/tcl_lsp_'+RUN
capture.TAG=RUN
orig_rpc=capture.rpc

def rpc(cmd,data=None,timeout=180):
 r=orig_rpc(cmd,data,timeout); r.stderr=b'\n'.join(x for x in r.stderr.split(b'\n') if not (x.startswith(b'** ') or b'openssh.com/pq.html' in x));return r
capture.rpc=rpc

def receipt(directory,label,cmd):
 r=rpc(cmd);(directory/(label+'.stdout')).write_bytes(b'\n'.join(x for x in r.stdout.split(b'\n') if b'FIPS Module' not in x));(directory/(label+'.stderr')).write_bytes(r.stderr);(directory/(label+'.exit')).write_text(str(r.returncode)+'\n');return r

args=argparse.ArgumentParser();args.add_argument('--phase',choices=['prepare','finish'],required=True);phase=args.parse_args().phase
if phase=='finish':
 assert json.loads((DEST/'ownership.json').read_text())=={'object':NAME,'absent_before':True}
 receipt(JOB,'delete-0','tmsh delete sys application template '+NAME)
 after=receipt(JOB,'after-0','tmsh list sys application template '+NAME)
 assert b'was not found' in after.stderr
 (DEST/'capture-complete.json').write_text(json.dumps({'completed':True,'browser_render_required':True})+'\n')
 sys.exit(0)
JOB.mkdir(exist_ok=False)
for label,cmd in [('version','tmsh show sys version'),('platform','tmsh show sys hardware'),('tmm-roster','tmsh show sys tmm-info')]:receipt(DEST,label,cmd)
assert b'was not found' in receipt(JOB,'before-0','tmsh list sys application template '+NAME).stderr
selected=[{'id':'version','setup':'','operation':'list [info patchlevel] [info tclversion] [namespace current]'}, {'id':'native-real-tcltest','setup':'','operation':'package require tcltest'}, {'id':'native-call-command-surface','setup':'','operation':'list commands [info commands call] procs [info procs call]'}, {'id':'native-call-runtime','setup':'set nh_call call','operation':'$nh_call /Common/__tcl_lsp_probe_'+RUN+'_absent::target {a b} {c d}'}, {'id':'native-ensemble-subcommand','setup':'set nh_namespace namespace','operation':'$nh_namespace ensemble create -command ::__tcl_lsp_probe_'+RUN+'_ensemble -map {}','cleanup':'if {[llength [info commands ::__tcl_lsp_probe_'+RUN+'_ensemble]]} {rename ::__tcl_lsp_probe_'+RUN+'_ensemble {}}'}, {'id':'native-oo-class','setup':'set nh_oo oo::class','operation':'$nh_oo create ::__tcl_lsp_probe_'+RUN+'_class {}','cleanup':'if {[llength [info commands ::__tcl_lsp_probe_'+RUN+'_class]]} {::__tcl_lsp_probe_'+RUN+'_class destroy}'}]
sources=[];presentation='section probe {\n'
for i,item in enumerate(selected):
 item.update(context='iapp-apl',variant_id='native-apl-capability-control-v1',exact_source=False,scope='Independent native APL callback capability control. No substitute harness and no portable original-input lookup claim.')
 source='set nh_apl_rows {}\n'+capture.body([item],'iapp-apl',RUN).replace('puts "NHO|','lappend nh_apl_rows "NHO|')+'\nreturn [join $nh_apl_rows "\\n"]\n'
 sources.append(source);presentation+='choice result'+str(i)+' tcl {\n'+source+'\n}\n'
presentation+='}\n'
config='sys application template '+NAME+' {\n actions { definition { implementation {} presentation {\n'+presentation+'}\n} }\n}\n'
(DEST/'iapp-apl-cases.json').write_text(json.dumps(selected,indent=2)+'\n');(JOB/'input.tcl').write_text('\n'.join(sources));(JOB/'applied.conf').write_text(config)
assert rpc('test ! -e '+REMOTE+' && mkdir -m 700 '+REMOTE+' && cat > '+REMOTE+'/template.conf',config.encode()).returncode==0
(DEST/'ownership.json').write_text(json.dumps({'object':NAME,'absent_before':True})+'\n')
(DEST/'provenance.json').write_text(json.dumps({'run_id':RUN,'configuration_saved':False,'timestamp_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'transport':'Authentication details omitted','invoking_action':'Chrome Application Services Create form template selection'},indent=2)+'\n')
r=receipt(JOB,'load','tmsh load sys config merge file '+REMOTE+'/template.conf');receipt(JOB,'applied-0','tmsh list sys application template '+NAME)
print(json.dumps({'load_exit':r.returncode,'template':NAME,'cases':len(selected)}))
