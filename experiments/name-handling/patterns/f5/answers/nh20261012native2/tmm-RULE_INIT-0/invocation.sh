set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012native2/evidence/__tcl_lsp_probe_nh20261012native2_preflight
E=/var/tmp/tcl_lsp_nh20261012native2/evidence/__tcl_lsp_probe_nh20261012native2_preflight
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native2_preflight
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012native2/__tcl_lsp_probe_nh20261012native2_preflight.conf
if ! grep -q "^0bc918515d380913b0099299e9c54f4535bbce83d602ec2167e15d96aacd3951 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012native2_preflight')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012native2/__tcl_lsp_probe_nh20261012native2_preflight.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native2_preflight
sleep 5
run runtime-logs grep -F 'NHO|nh20261012native2|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
