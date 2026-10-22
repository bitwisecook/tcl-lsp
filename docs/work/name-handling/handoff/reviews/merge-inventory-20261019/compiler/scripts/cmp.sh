#!/bin/bash
# cmp.sh base.txt after.txt : prints regressions (ok->FAILED) and gains
b=$1; a=$2
join <(sed -E 's/^test (.*) \.\.\. (.*)$/\1 \2/' $b | sort) <(sed -E 's/^test (.*) \.\.\. (.*)$/\1 \2/' $a | sort) > ${SOL49_SCRATCH:-/tmp/sol49}/.j
echo "REGRESSIONS:"; awk '$2=="ok" && $3=="FAILED"{print "  "$1}' ${SOL49_SCRATCH:-/tmp/sol49}/.j
echo "GAINS: $(awk '$2=="FAILED" && $3=="ok"' ${SOL49_SCRATCH:-/tmp/sol49}/.j | wc -l)"
