set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012harness2/evidence/__tcl_lsp_probe_nh20261012harness2_2
E=/var/tmp/tcl_lsp_nh20261012harness2/evidence/__tcl_lsp_probe_nh20261012harness2_2
owned=()
run() { label=$1; shift; "$@" > "$E/$label.stdout" 2> "$E/$label.stderr"; echo $? > "$E/$label.exit"; }
cleanup() {
  for (( i=${#owned[@]}-1; i>=0; i-- )); do
    read -r -a words <<< "${owned[$i]}"
    run "delete-$i" tmsh delete "${words[@]}"
    run "after-$i" tmsh list "${words[@]}"
  done
}
trap cleanup EXIT
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module0
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module1
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-2 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module2
if ! grep -q "was not found" "$E/before-2.stderr" "$E/before-2.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-3 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module3
if ! grep -q "was not found" "$E/before-3.stderr" "$E/before-3.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-4 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module4
if ! grep -q "was not found" "$E/before-4.stderr" "$E/before-4.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-5 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module5
if ! grep -q "was not found" "$E/before-5.stderr" "$E/before-5.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-6 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module6
if ! grep -q "was not found" "$E/before-6.stderr" "$E/before-6.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-7 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module7
if ! grep -q "was not found" "$E/before-7.stderr" "$E/before-7.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-8 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module8
if ! grep -q "was not found" "$E/before-8.stderr" "$E/before-8.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-9 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2
if ! grep -q "was not found" "$E/before-9.stderr" "$E/before-9.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-10 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012harness2_2_vs
if ! grep -q "was not found" "$E/before-10.stderr" "$E/before-10.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run destination-inventory tmsh list ltm virtual one-line
if grep -q "destination 192.0.2.10:19912 " "$E/destination-inventory.stdout"; then echo "Destination collision" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012harness2/__tcl_lsp_probe_nh20261012harness2_2.conf
if ! grep -q "^01db2146f660579adf67de620364b68e9cccc281dac07bce52e90de9339e4c49 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module0')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module1')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module2')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module3')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module4')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module5')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module6')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module7')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module8')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2')
owned+=('ltm virtual /Common/__tcl_lsp_probe_nh20261012harness2_2_vs')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012harness2/__tcl_lsp_probe_nh20261012harness2_2.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module0
run applied-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module1
run applied-2 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module2
run applied-3 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module3
run applied-4 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module4
run applied-5 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module5
run applied-6 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module6
run applied-7 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module7
run applied-8 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2_module8
run applied-9 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012harness2_2
run applied-10 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012harness2_2_vs
if [ "$(cat "$E/load.exit")" = 0 ] && [ "$(cat "$E/applied-10.exit")" = 0 ]; then touch "$E/traffic-ready"; for n in {1..90}; do [ -e "$E/traffic-done" ] && break; sleep 1; done; fi
sleep 5
run runtime-logs grep -F 'NHO|nh20261012harness2|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
