set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012exact1/evidence/__tcl_lsp_probe_nh20261012exact1_4_29
E=/var/tmp/tcl_lsp_nh20261012exact1/evidence/__tcl_lsp_probe_nh20261012exact1_4_29
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
run before-0 tmsh list sys icall script /Common/__tcl_lsp_probe_nh20261012exact1_4_29
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012exact1_4_29_handler
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012exact1/__tcl_lsp_probe_nh20261012exact1_4_29.conf
if ! grep -q "^d2dc02c93f14fc7fc6e2c7f5a143bb714fd9cefc9fe3cd3d937c1ddce1d3b3b8 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('sys icall script /Common/__tcl_lsp_probe_nh20261012exact1_4_29')
owned+=('sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012exact1_4_29_handler')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012exact1/__tcl_lsp_probe_nh20261012exact1_4_29.conf
run applied-0 tmsh list sys icall script /Common/__tcl_lsp_probe_nh20261012exact1_4_29
run applied-1 tmsh list sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012exact1_4_29_handler
run invoke tmsh generate sys icall event name __tcl_lsp_probe_nh20261012exact1_4_29_event
sleep 5
run runtime-logs grep -F 'NHO|nh20261012exact1|' /var/tmp/scriptd.out
echo "configuration_saved=false" > "$E/save-status.txt"
