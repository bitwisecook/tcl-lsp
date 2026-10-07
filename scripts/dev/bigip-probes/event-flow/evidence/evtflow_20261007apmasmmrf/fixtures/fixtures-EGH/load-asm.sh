#!/bin/bash
set -u
for mode in transparent blocking; do
    policy=/Common/__tcl_lsp_evtflow_egh_asm_$mode
    blocking=disabled
    if [ "$mode" = blocking ]; then blocking=enabled; fi
    tmsh create asm policy "$policy" active blocking-mode "$blocking" encoding utf-8 policy-builder disabled policy-template POLICY_TEMPLATE_RAPID_DEPLOYMENT
    tmsh publish asm policy "$policy"
done
