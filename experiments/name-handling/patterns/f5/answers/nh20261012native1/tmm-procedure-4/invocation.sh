set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012native1/evidence/__tcl_lsp_probe_nh20261012native1_observer_detached
E=/var/tmp/tcl_lsp_nh20261012native1/evidence/__tcl_lsp_probe_nh20261012native1_observer_detached
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_observer_detached
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012native1/__tcl_lsp_probe_nh20261012native1_observer_detached.conf
if ! grep -q "^33e046e58ab249805381162ae9cc3482fa5a11dc9cb6fcc7290607f5c8405b8b " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_observer_detached')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012native1/__tcl_lsp_probe_nh20261012native1_observer_detached.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_observer_detached
sleep 5
run runtime-logs grep -F 'NHO|nh20261012native1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
