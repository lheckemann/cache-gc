# nix path-info --all --json --store file:/$cache | jq --slurpfile dates <(echo $cache/*.narinfo | xargs stat -c '%Y %n' -- | jq -R)
def path_to_hash: match("/?([0-9a-z]{32})-?") | .captures[0].string;

($dates | map(. / " " | { name: .[1][:32], value: (.[0] | tonumber) }) | from_entries) as $registrationTimes
| map(. + {
  registrationTime: ($registrationTimes[(.path | path_to_hash)])
})

