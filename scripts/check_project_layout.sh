#!/usr/bin/env bash
# The project layout gate: docs/PROJECT_LAYOUT.md's rules over one project's asset root
# (`<project>/assets/`) — a game, the engine's `examples/`, or a bundled playground project.
#
# The kind folders and the extension each one's New writes are read from `CONVENTIONS` in
# crates/editor/src/asset_browser/tree.rs, the table the asset browser routes by, and from the
# doc's folder table; the two must agree row for row and in order, or the gate fails before it
# reads a file (a count alone would pass a renamed root). The doc's `ai/` and `LICENSE` rows are
# its own additions and are left out of that comparison by name; so are the companions and the
# other recorded-audio extensions listed in COMPANIONS below, which the router has no row for.
#
# What it checks, each file named once with the first rule it breaks: every entry at the root is
# a kind folder (or `ai/`, which mirrors them); every file sits in its own kind's folder; no kind
# folder sits inside another; every sprite has its sidecar, and a synced `sprites/` (one holding a
# `sync.list`) is exactly what its list names; every name is ASCII snake_case with one dot before
# its extension chain, but for a font's distributed filename (reported, never passed silently),
# `LICENSE`, `.keep` and the names a song's Save generates. `music/imports/` holds the recordings
# the playground's import wrote, as `music/samples/` holds a saved song's. The LICENSE rule is
# scripts/check_audio_provenance.sh, run over the project rather than repeated here.
#
# Usage: scripts/check_project_layout.sh <project>
# One line per violation, exit 1 on any; exit 2 when the project or a table cannot be read.
set -uo pipefail

if [ $# -ne 1 ]; then
    echo "usage: scripts/check_project_layout.sh <project>" >&2
    exit 2
fi
engine_root=$(cd "$(dirname "$0")/.." && pwd)
project=${1%/}
asset_root="$project/assets"
if [ ! -d "$asset_root" ]; then
    echo "project layout gate: $project has no assets/ folder" >&2
    exit 2
fi
router_file="$engine_root/crates/editor/src/asset_browser/tree.rs"
doc_file="$engine_root/docs/PROJECT_LAYOUT.md"

# A suffix the router has no row for, and the kind folder whose files carry it.
COMPANIONS=".sheet.ron:sprites .jpg:sprites .jpeg:sprites .bmp:sprites .mid.ron:music .midi.ron:music .midi:music .ogg:sounds .mp3:sounds .flac:sounds .otf:fonts"
# The folders a LICENSE may sit in, at any depth below them.
LICENSED_FOLDERS="sounds music soundfonts fonts"

# Prints `<root> <extension>` for each `Convention { … }` row of CONVENTIONS, in order.
router_rows() {
    awk '/^const CONVENTIONS/ { inside = 1; next } inside && /^\];/ { inside = 0 } inside' "$router_file" |
        sed -n 's/^[[:space:]]*Convention[[:space:]]*{.*root:[[:space:]]*"\([^"]*\)",[[:space:]]*extension:[[:space:]]*"\([^"]*\)".*/\1 \2/p'
}

# The number of lines in CONVENTIONS naming a row, read or not: a row split over lines is caught.
router_row_lines() {
    awk '/^const CONVENTIONS/ { inside = 1; next } inside && /^\];/ { inside = 0 } inside' "$router_file" |
        grep -c 'Convention[[:space:]]*{'
}

# Prints `<folder> <extension>` for each row of the doc's folder table but `ai/` and `LICENSE`.
doc_rows() {
    awk '/^## The folders/ { inside = 1; next } /^## / { inside = 0 } inside' "$doc_file" |
        sed -n 's/^|[[:space:]]*`\([^`]*\)`[[:space:]]*|[[:space:]]*\([^|]*\)|.*/\1 \2/p' |
        while read -r folder extension; do
            case $folder in ai/|LICENSE) continue ;; esac
            echo "$folder $(printf '%s' "$extension" | tr -d '`')"
        done
}

router_table=$(router_rows)
doc_table=$(doc_rows)
if [ -z "$router_table" ] || [ "$(printf '%s\n' "$router_table" | wc -l)" -ne "$(router_row_lines)" ]; then
    echo "project layout gate: cannot read every Convention row of CONVENTIONS in ${router_file#"$engine_root"/}" >&2
    exit 2
fi
if [ "$router_table" != "$doc_table" ]; then
    echo "TABLE       CONVENTIONS (tree.rs) and docs/PROJECT_LAYOUT.md's folder table differ (< tree.rs, > the doc):"
    diff <(printf '%s\n' "$router_table") <(printf '%s\n' "$doc_table") | grep '^[<>]'
    echo "project layout gate: the two tables disagree; fix one side before the trees are read"
    exit 1
fi

kind_roots=$(printf '%s\n' "$router_table" | awk '{ sub(/\/$/, "", $1); print $1 }')
# `<suffix>:<kind folder>` for every suffix a kind folder holds, longest first.
suffix_homes=$( { printf '%s\n' "$router_table" | awk '{ sub(/\/$/, "", $1); print $2 ":" $1 }'; printf '%s\n' $COMPANIONS; } |
    awk -F: '{ print length($1) " " $0 }' | sort -rn | cut -d' ' -f2-)

is_kind_root() { printf '%s\n' "$kind_roots" | grep -qxF "$1"; }
is_snake_case() { [[ $1 =~ ^[a-z][a-z0-9]*(_[a-z0-9]+)*$ ]]; }
# A song's sample folder: its path under its home, each `/` written `__` — a recovered score's
# included, whose stem carries `.recovered-<token>`.
is_flat_sample_folder() { [[ $1 =~ ^[a-z][a-z0-9]*(_[a-z0-9]+)*(__[a-z][a-z0-9]*(_[a-z0-9]+)*)*(\.recovered-[A-Za-z0-9_-]+)?$ ]]; }

# Prints `<suffix>:<kind folder>` for the longest suffix `name` ends with, or nothing.
suffix_home_of() {
    local name=$1 entry
    while IFS= read -r entry; do
        case $name in *"${entry%%:*}") echo "$entry"; return ;; esac
    done <<< "$suffix_homes"
}

# Prints each `.png` a sync list names, as the sync writes it: `<source> [-> <destination>]`.
synced_pngs() {
    awk '
        { sub(/^[ \t]+/, ""); sub(/[ \t]+$/, "") }
        $0 == "" || /^#/ { next }
        /->/ { split($0, sides, "->"); destination = sides[2]; gsub(/^[ \t]+|[ \t]+$/, "", destination); print destination; next }
        { count = split($0, segments, "/"); print segments[count] }
    ' "$1"
}

violations=0
checked=0
report() {
    echo "LAYOUT      $1  ($2)"
    violations=$((violations + 1))
}

# Folders first: a folder that is no kind is reported once, and the files under it are not.
rejected_folders=()
is_under_rejected() {
    local rejected
    for rejected in "${rejected_folders[@]+"${rejected_folders[@]}"}"; do
        case $1 in "$rejected"/*) return 0 ;; esac
    done
    return 1
}
while IFS= read -r folder; do
    relative=${folder#"$asset_root"/}
    is_under_rejected "$relative" && continue
    IFS=/ read -r -a segments <<< "$relative"
    depth=${#segments[@]}
    name=${segments[depth - 1]}
    kind_depth=0
    [ "${segments[0]}" = ai ] && kind_depth=1
    if [ "$depth" -eq 1 ] && [ "$name" = ai ]; then
        continue
    elif [ "$depth" -eq $((kind_depth + 1)) ]; then
        if ! is_kind_root "$name"; then
            report "assets/$relative/" "no kind folder: the asset root holds the folders of docs/PROJECT_LAYOUT.md"
            rejected_folders+=("$relative")
        fi
    elif is_kind_root "$name"; then
        report "assets/$relative/" "a kind folder inside another: kind folders sit at the asset root"
    elif [ "${segments[kind_depth]}" = music ] && [ "${segments[kind_depth + 1]}" = samples ] && [ "$depth" -eq $((kind_depth + 3)) ]; then
        is_flat_sample_folder "$name" || report "assets/$relative/" "a song's sample folder is its path with each / written __"
    elif ! is_snake_case "$name"; then
        report "assets/$relative/" "a folder name is ASCII snake_case"
    fi
done < <(find "$asset_root" -mindepth 1 -type d | sort)

while IFS= read -r file; do
    relative=${file#"$asset_root"/}
    name=${relative##*/}
    checked=$((checked + 1))
    is_under_rejected "$relative" && continue
    [ "$name" = .keep ] && continue
    in_ai=0
    within=$relative
    if [ "${relative%%/*}" = ai ]; then
        in_ai=1
        within=${relative#ai/}
    fi
    case $within in
        */*) ;;
        *) report "assets/$relative" "a file outside every kind folder"; continue ;;
    esac
    folder=${within%%/*}
    below=${within#*/}
    in_samples=0
    [ "$folder" = music ] && [ "${below%%/*}" = samples ] && [ "$below" != samples ] && in_samples=1
    in_recordings=$in_samples
    [ "$folder" = music ] && [ "${below%%/*}" = imports ] && [ "$below" != imports ] && in_recordings=1

    if [ "$folder" = sprites ] && [ -f "$asset_root/${relative%/*}/sync.list" ] && [ "${below%/*}" = "$below" ]; then
        continue
    fi

    if [ "$name" = LICENSE ]; then
        case " $LICENSED_FOLDERS " in
            *" $folder "*) ;;
            *) report "assets/$relative" "a LICENSE belongs in an audio folder or fonts/" ;;
        esac
        continue
    fi

    entry=$(suffix_home_of "$name")
    if [ -z "$entry" ]; then
        report "assets/$relative" "no folder of docs/PROJECT_LAYOUT.md holds this extension"
        continue
    fi
    suffix=${entry%%:*}
    home=${entry#*:}
    stem=${name%"$suffix"}
    if [ "$in_recordings" -eq 1 ]; then
        if [ "$home" != sounds ]; then
            report "assets/$relative" "a song's sample folder holds recordings and their LICENSE"
            continue
        fi
    elif [ "$folder" = scenes ] && [ "$suffix" = .ron ]; then
        report "assets/$relative" "a scene is named <name>.scene.ron"
        continue
    elif [ "$home" != "$folder" ]; then
        report "assets/$relative" "a $home/ file in $folder/"
        continue
    fi

    if [ "$folder" = fonts ]; then
        is_snake_case "$stem" || echo "NOTE        assets/$relative  (a font keeps its distributed filename: a stated exception, not judged)"
        continue
    fi
    if [ "$folder" = sprites ]; then
        case $suffix in
            .png) [ -f "$asset_root/${relative%.png}.sheet.ron" ] || { report "assets/$relative" "a sprite without its .sheet.ron"; continue; } ;;
            .sheet.ron) [ -f "$asset_root/${relative%.sheet.ron}.png" ] || { report "assets/$relative" "a sidecar without its .png"; continue; } ;;
        esac
    fi
    if [ "$in_samples" -eq 1 ] && [[ $stem =~ ^(.+)-[0-9a-f]{8}$ ]]; then
        stem=${BASH_REMATCH[1]}
    elif [ "$folder" = music ] && [[ $stem =~ ^([^.]+)\.recovered-[A-Za-z0-9_-]+$ ]] && { [ "$suffix" = .mid ] || [ "$suffix" = .mid.ron ] || [ "$suffix" = .midi ] || [ "$suffix" = .midi.ron ]; }; then
        stem=${BASH_REMATCH[1]}
    fi
    if ! is_snake_case "$stem"; then
        report "assets/$relative" "a name is ASCII snake_case with one dot before its extension chain"
    elif [ "$folder" = sounds ] && [ "${stem#snd_}" != "$stem" ]; then
        report "assets/$relative" "a sound is named for its action, without the snd_ prefix"
    elif [ "$in_ai" -eq 1 ] && [ "$in_samples" -eq 0 ] && [ "${stem#ai_}" = "$stem" ]; then
        report "assets/$relative" "a file under ai/ carries the ai_ prefix"
    fi
done < <(find "$asset_root" -type f | sort)

# A synced sprites folder is exactly what its list names: each .png with its sidecar, and the list.
while IFS= read -r sync_list; do
    sprites_folder=${sync_list%/sync.list}
    shown=assets/${sprites_folder#"$asset_root"/}
    listed=$(synced_pngs "$sync_list")
    while IFS= read -r png; do
        [ -z "$png" ] && continue
        [ -f "$sprites_folder/$png" ] || report "$shown/$png" "named in sync.list but missing"
        [ -f "$sprites_folder/${png%.png}.sheet.ron" ] || report "$shown/${png%.png}.sheet.ron" "sync.list names its .png but the sidecar is missing"
    done <<< "$listed"
    while IFS= read -r entry; do
        name=${entry##*/}
        if [ -d "$entry" ]; then
            report "$shown/$name/" "the sync owns this folder: nothing but its pairs and the list sits in it"
            continue
        fi
        [ "$name" = sync.list ] && continue
        printf '%s\n' "$listed" | grep -qxF "$name" && continue
        printf '%s\n' "$listed" | grep -qxF "${name%.sheet.ron}.png" && continue
        report "$shown/$name" "not named in sync.list, which owns this folder"
    done < <(find "$sprites_folder" -mindepth 1 -maxdepth 1 | sort)
done < <(find "$asset_root" -path '*/sprites/sync.list' -type f | sort)

if ! provenance=$("$engine_root/scripts/check_audio_provenance.sh" "$project"); then
    provenance_violations=0
    while IFS= read -r line; do
        case $line in
            "NO LICENSE"*|UNNAMED*) echo "$line"; provenance_violations=$((provenance_violations + 1)) ;;
        esac
    done <<< "$provenance"
    if [ "$provenance_violations" -eq 0 ]; then
        echo "project layout gate: scripts/check_audio_provenance.sh failed over $project without naming a file" >&2
        exit 2
    fi
    violations=$((violations + provenance_violations))
fi

if [ "$violations" -ne 0 ]; then
    echo "project layout gate: $project: $violations violations in $checked files"
    exit 1
fi
echo "project layout gate: $project clean ($checked files)"
