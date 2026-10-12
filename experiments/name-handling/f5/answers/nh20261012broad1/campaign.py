# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Execute retained question variants with the capture owner's isolated jobs."""
import argparse
import datetime
import json
import re
import shlex
import subprocess
from pathlib import Path

import capture
import cases
import broader_cases

CONTEXTS=['stock-8.4','stock-8.5','tmsh-cli-script','iapp-implementation','icall','tmm-RULE_INIT','tmm-procedure','tmm-traffic-event','iapp-apl']


def cleanup_prefix(prefix):
    return f'''foreach nh_child [interp slaves] {{if {{[string match {prefix}* $nh_child]}} {{interp delete $nh_child}}}}
foreach nh_ns [namespace children :: ::{prefix}*] {{namespace delete $nh_ns}}
foreach nh_cmd [info commands ::{prefix}*] {{rename $nh_cmd {{}}}}
foreach nh_var [info vars ::{prefix}*] {{unset -nocomplain $nh_var}}
list [namespace children :: ::{prefix}*] [info commands ::{prefix}*] [info vars ::{prefix}*]'''


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--run',required=True)
    parser.add_argument('--kind',choices=['patterns','broader'],required=True)
    parser.add_argument('--contexts',default=','.join(CONTEXTS))
    args=parser.parse_args()
    tag=args.run
    if not re.fullmatch('[a-z0-9]{6,24}',tag):raise ValueError('Invalid tag')
    capture.TAG=tag
    root=capture.ROOT/'experiments/name-handling'
    dest=root/('patterns/f5/answers' if args.kind=='patterns' else 'f5/answers')/tag
    dest.mkdir(parents=True,exist_ok=False)
    for module in [Path(__file__),Path(capture.__file__),Path(cases.__file__),Path(broader_cases.__file__)]:
        (dest/module.name).write_bytes(module.read_bytes())
    remote='/var/tmp/tcl_lsp_'+tag
    p=capture.rpc('test ! -e '+shlex.quote(remote)+' && mkdir -m 700 '+shlex.quote(remote))
    if p.returncode:raise RuntimeError('Staging collision')
    (dest/'provenance.json').write_text(json.dumps({'run_id':tag,'timestamp_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'input_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'configuration_saved':False,'remote_staging':remote},indent=2)+'\n')
    for label,command in [('version','tmsh show sys version'),('platform','tmsh show sys hardware'),('tmm-roster','tmsh show sys tmm-info'),('failover','tmsh show sys failover')]:
        p=capture.rpc(command)
        (dest/(label+'.stdout')).write_bytes(p.stdout)
        (dest/(label+'.stderr')).write_bytes(p.stderr)
        (dest/(label+'.json')).write_text(json.dumps({'argv':capture.SSH+[command],'exit_code':p.returncode},indent=2)+'\n')
    for context in args.contexts.split(','):
        if context not in CONTEXTS:raise ValueError(context)
        context_tag=tag+str(CONTEXTS.index(context))
        if args.kind=='patterns':
            selected=cases.build_cases(context_tag)
            if context.startswith('stock-'):
                selected=cases.stock_cases()+cases.wiki_cases()
                for item in selected:
                    if item['id'].startswith('wiki-'):
                        observed=capture.emit(context,tag,item['id'],'body-return','$code','$value','none')
                        item['setup']+='proc ::nh_body_return {cmd code value op} {set nh_invocation once; '+observed+'}\n'
                        item['setup']+='proc ::nh_attach {cmd code value op} {if {$code == 0 && [lindex $cmd 1] == "isolated"} {trace add execution ::isolated leave ::nh_body_return}}\ntrace add execution ::proc leave ::nh_attach\n'
                        item['variant_id']='real-tcltest-full-suite-selected-traced-v1'
                        item['adaptation']={'kind':'execution-trace-observer','source_bytes_unchanged':True,'observes':'isolated leave completion/result; child inner error options unavailable'}
        else:
            selected=broader_cases.build_cases(context_tag)
            for item in selected:
                prefix='__tcl_lsp_probe_'+context_tag+'_'+item['id'].replace('-','_')
                if 'fresh interpreter only' in item['scope']:
                    original=item['setup']+item['operation']
                    item['setup']='set nh_fresh [interp create]'
                    item['operation']='interp eval $nh_fresh '+capture.literal(original)
                    item['cleanup']='interp delete $nh_fresh'
                    item['scope']+=' Child-interpreter adaptation.'
                else:item['cleanup']=cleanup_prefix(prefix)
        for item in selected:
            item['context']=context
        (dest/(context+'-cases.json')).write_text(json.dumps(selected,indent=2)+'\n')
        if context.startswith('stock-'):
            groups=[[x] for x in selected]
        elif context in {'tmm-RULE_INIT','tmm-procedure'}:
            groups=[selected[i:i+3] for i in range(0,len(selected),3)]
        else:groups=[selected]
        for index,group in enumerate(groups):
            name='__tcl_lsp_probe_'+tag+'_'+str(CONTEXTS.index(context))+'_'+str(index)
            source=capture.body(group,context,tag)
            capture.run_job(dest/(context+'-'+str(index)),remote,context,name,source,wait=5)
            print('captured',args.kind,context,index,','.join(x['id'] for x in group),flush=True)
    (dest/'capture-complete.json').write_text(json.dumps({'completed_utc':datetime.datetime.now(datetime.timezone.utc).isoformat()},indent=2)+'\n')

if __name__=='__main__':main()
