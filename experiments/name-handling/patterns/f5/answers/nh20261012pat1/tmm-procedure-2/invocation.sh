set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012pat1/evidence/__tcl_lsp_probe_nh20261012pat1_6_2
E=/var/tmp/tcl_lsp_nh20261012pat1/evidence/__tcl_lsp_probe_nh20261012pat1_6_2
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012pat1_6_2
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012pat1/__tcl_lsp_probe_nh20261012pat1_6_2.conf
if ! grep -q "^12653465968e6840a1af38af05668b483b69df5bb7424a1ba088c9ecaf56b390 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012pat1_6_2')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012pat1/__tcl_lsp_probe_nh20261012pat1_6_2.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012pat1_6_2
sleep 5
run runtime-logs grep -F 'NHO|nh20261012pat1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
