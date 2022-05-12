#!/usr/bin/env bash
set -euo pipefail

error() {
    echo "$*" >&2
    exit 1
}

confirm() {
    read -rn1 -p "$1" confirm
    echo
    [[ "$confirm" = y ]]
}

: "${libexec_dir:="$(dirname "$(readlink -f "$0")")/../libexec/cache-gc"}"
[[ -r "$libexec_dir/add-registration-times.jq" ]] || error "Couldn't find registration time adder, are we installed correctly?"

usage() {
    error "Usage: $0 [--delete] <cache-dir>"
}
delete=
cache_dir=
gc_args=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --delete)
            delete=1
            shift
            ;;
        --days)
            gc_args+=("$2")
            shift 2
            ;;
        *)
            if [[ -z "$cache_dir" ]]; then
               cache_dir="$1"
               shift
            else
                usage
            fi
            ;;
    esac
done

[[ -n "$cache_dir" ]] || usage

paths_to_delete=$(
    nix path-info --all --json --store file://"$cache_dir" --option extra-experimental-features "nix-command" |
        jq -f "$libexec_dir/add-registration-times.jq" --slurpfile dates <(cd "$cache_dir"; echo *.narinfo | xargs stat -c '%Y %n' -- | jq -R) |
        gc "${gc_args[@]}"
)

if [[ -z "$paths_to_delete" ]]; then
    echo >&2 "Nothing to delete."
    exit 0
fi

if [[ -n "$delete" ]] || confirm "Perform deletion? "; then
    echo "$paths_to_delete" | (cd "$cache_dir"; xargs -P4 rm)
else
    output=$(mktemp --tmpdir hydra-gc.XXXXXXX)
    echo "$paths_to_delete" > "$output"
    echo >&2 "Wrote paths to delete to $output."
fi
