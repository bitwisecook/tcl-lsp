#!/bin/bash
exec valgrind --tool=massif --massif-out-file=${OUT:?set OUT to an output directory}/massif-8feb2.out --max-snapshots=40 --detailed-freq=1000000 --threshold=0.1 --depth=150 ${SERVER:?set SERVER to a server binary} "$@" 2>>${OUT:?set OUT to an output directory}/massif-8feb-valgrind.log
