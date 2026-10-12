set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012broad1/evidence/__tcl_lsp_probe_nh20261012broad1_6_12
E=/var/tmp/tcl_lsp_nh20261012broad1/evidence/__tcl_lsp_probe_nh20261012broad1_6_12
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012broad1_6_12
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012broad1/__tcl_lsp_probe_nh20261012broad1_6_12.conf
if ! grep -q "^610819da54df9d4a1b758e7e8ea784f9121adf7c6d17065d2301f04a1a0afd2d " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012broad1_6_12')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012broad1/__tcl_lsp_probe_nh20261012broad1_6_12.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012broad1_6_12
sleep 5
run runtime-logs grep -F 'NHO|nh20261012broad1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
