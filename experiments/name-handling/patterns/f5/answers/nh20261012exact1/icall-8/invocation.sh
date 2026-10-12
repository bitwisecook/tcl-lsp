set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012exact1/evidence/__tcl_lsp_probe_nh20261012exact1_4_8
E=/var/tmp/tcl_lsp_nh20261012exact1/evidence/__tcl_lsp_probe_nh20261012exact1_4_8
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
run before-0 tmsh list sys icall script /Common/__tcl_lsp_probe_nh20261012exact1_4_8
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012exact1_4_8_handler
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012exact1/__tcl_lsp_probe_nh20261012exact1_4_8.conf
if ! grep -q "^d0e6a45ad2a16bffef86f707c4efaff68a3a2a93325fcc979ca72624b36e9c39 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('sys icall script /Common/__tcl_lsp_probe_nh20261012exact1_4_8')
owned+=('sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012exact1_4_8_handler')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012exact1/__tcl_lsp_probe_nh20261012exact1_4_8.conf
run applied-0 tmsh list sys icall script /Common/__tcl_lsp_probe_nh20261012exact1_4_8
run applied-1 tmsh list sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012exact1_4_8_handler
run invoke tmsh generate sys icall event name __tcl_lsp_probe_nh20261012exact1_4_8_event
sleep 5
run runtime-logs grep -F 'NHO|nh20261012exact1|' /var/tmp/scriptd.out
echo "configuration_saved=false" > "$E/save-status.txt"
