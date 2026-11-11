#!/usr/bin/env bash
# The project layout gate's own test. A tree holding every stated exception and every name the
# engine generates passes, and so does a synced sprites folder whose names the gate does not
# judge; a tree breaking each rule once fails naming each break, and so does a synced folder that
# is not what its list names. The two tables are checked against each other in a scratch copy of
# the engine: a renamed root, a reordered, duplicated or dropped row each fail, and a row the gate
# cannot read stops it. Last, a song saved by the real editor (headless, through the command
# API) with two samples of one name passes — the collision's `<stem>-<hash>` name included.
#
# Run from anywhere: scripts/tests/check_project_layout_test.sh
# The song save builds and runs the editor binary (`--features editor`) on its first run.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 2

gate=scripts/check_project_layout.sh
fixtures=scripts/tests/project_layout
failures=0
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fail() {
    echo "FAIL  $1"
    failures=$((failures + 1))
}

expect_line() {
    grep -qF "$2" <<< "$1" || fail "expected \"$2\" in: $1"
}

if ! clean_output=$("$gate" "$fixtures/clean"); then
    fail "the tree holding every exception and generated name should pass: $clean_output"
fi
expect_line "$clean_output" "NOTE        assets/fonts/BlackSamsGold-ej5e.ttf"
if grep -q "fonts/font.ttf" <<< "$clean_output"; then
    fail "a snake_case font needs no note: $clean_output"
fi

if ! synced_output=$("$gate" "$fixtures/synced"); then
    fail "a synced sprites folder matching its list should pass, its names unjudged: $synced_output"
fi

broken_output=$("$gate" "$fixtures/broken")
broken_status=$?
[ "$broken_status" -eq 1 ] || fail "the broken tree should exit 1, exited $broken_status"
for expected in \
    "LAYOUT      assets/images/  (no kind folder" \
    "LAYOUT      assets/notes.txt  (a file outside every kind folder" \
    "LAYOUT      assets/sprites/scenes/  (a kind folder inside another" \
    "LAYOUT      assets/scenes/hero.png  (a sprites/ file in scenes/" \
    "LAYOUT      assets/locales/level.scene.ron  (a scenes/ file in locales/" \
    "LAYOUT      assets/scenes/scene.ron  (a scene is named <name>.scene.ron" \
    "LAYOUT      assets/sprites/lonely.png  (a sprite without its .sheet.ron" \
    "LAYOUT      assets/sprites/orphan.sheet.ron  (a sidecar without its .png" \
    "LAYOUT      assets/sounds/snd_jump.wav  (a sound is named for its action" \
    "LAYOUT      assets/sounds/BigBoom.wav  (a name is ASCII snake_case" \
    "LAYOUT      assets/sounds/two.dots.wav  (a name is ASCII snake_case" \
    "LAYOUT      assets/ai/music/theme.mid  (a file under ai/ carries the ai_ prefix" \
    "LAYOUT      assets/scenes/LICENSE  (a LICENSE belongs in an audio folder" \
    "LAYOUT      assets/scripts/EnemyAi/  (a folder name is ASCII snake_case" \
    "LAYOUT      assets/scripts/notes.md  (no folder of docs/PROJECT_LAYOUT.md holds this extension" \
    "LAYOUT      assets/music/samples/Theme/  (a song's sample folder is its path" \
    "LAYOUT      assets/music/samples/theme/score.mid  (a song's sample folder holds recordings" \
    "UNNAMED     assets/sounds/unnamed.wav"; do
    expect_line "$broken_output" "$expected"
done
if grep -q "images/hero.png" <<< "$broken_output"; then
    fail "a file under a folder that is no kind is covered by the folder's line: $broken_output"
fi

broken_synced_output=$("$gate" "$fixtures/broken_synced")
[ $? -eq 1 ] || fail "the broken synced folder should exit 1: $broken_synced_output"
for expected in \
    "LAYOUT      assets/sprites/gone.png  (named in sync.list but missing" \
    "LAYOUT      assets/sprites/bee.sheet.ron  (sync.list names its .png but the sidecar is missing" \
    "LAYOUT      assets/sprites/extra.png  (not named in sync.list" \
    "LAYOUT      assets/sprites/nested/  (the sync owns this folder"; do
    expect_line "$broken_synced_output" "$expected"
done
if grep -q "listed" <<< "$broken_synced_output"; then
    fail "a pair the list names and the folder holds is not reported: $broken_synced_output"
fi

# The tables, in a scratch engine: the gate finds both through its own location.
engine="$work/engine"
mkdir -p "$engine/scripts" "$engine/crates/editor/src/asset_browser" "$engine/docs"
cp scripts/check_project_layout.sh scripts/check_audio_provenance.sh "$engine/scripts/"
router=crates/editor/src/asset_browser/tree.rs
doc=docs/PROJECT_LAYOUT.md
run_with_tables() {
    "$engine/scripts/check_project_layout.sh" "$fixtures/clean"
}
restore_tables() {
    cp "$router" "$engine/$router"
    cp "$doc" "$engine/$doc"
}

restore_tables
run_with_tables > /dev/null || fail "the scratch engine with the real tables should pass"

expect_table_failure() {
    local output status
    output=$(run_with_tables)
    status=$?
    [ "$status" -eq 1 ] || fail "$1 should exit 1, exited $status: $output"
    expect_line "$output" "TABLE       CONVENTIONS (tree.rs) and docs/PROJECT_LAYOUT.md's folder table differ"
    restore_tables
}

sed 's|root: "sounds/"|root: "audio/"|' "$router" > "$engine/$router"
expect_table_failure "a root renamed in the router alone"
awk '/^\| `scenes\/`/ { held = $0; next } { print } /^\| `scripts\/`/ { print held }' "$doc" > "$engine/$doc"
expect_table_failure "two rows of the doc's table swapped"
sed '/root: "fonts\/"/p' "$router" > "$engine/$router"
expect_table_failure "a router row duplicated"
sed '/^| `locales\/`/d' "$doc" > "$engine/$doc"
expect_table_failure "a row dropped from the doc"
awk '/AssetKind::Script,/ { sub(/, root:/, ",\n        root:") } { print }' "$router" > "$engine/$router"
split_output=$(run_with_tables 2>&1)
split_status=$?
[ "$split_status" -eq 2 ] || fail "a router row split over two lines should stop the gate with 2, exited $split_status"
expect_line "$split_output" "cannot read every Convention row"
restore_tables

# A real song save: two samples named blip.wav with other bytes, saved by the editor itself.
song_project="$work/song_save"
mkdir -p "$song_project/assets/music" "$song_project/assets/soundfonts" "$song_project/assets/sounds/extra"
cp crates/music_midi/fixtures/dansenapolitaine.mid "$song_project/assets/music/theme.mid"
cp examples/assets/soundfonts/sine.sf2 examples/assets/soundfonts/LICENSE "$song_project/assets/soundfonts/"
cp crates/music_midi/fixtures/sine_22050_mono.wav "$song_project/assets/sounds/blip.wav"
cp crates/music_midi/fixtures/sine_48000_mono.wav "$song_project/assets/sounds/extra/blip.wav"
printf 'Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\n\nFiles: blip.wav extra/blip.wav\nCopyright: none (generated)\nLicense: public-domain\n' \
    > "$song_project/assets/sounds/LICENSE"
provenance='"provenance": {"author": "2026 Jesse", "license": "MIT"}'
song_output=$(printf '%s\n' \
    'song init music/theme.mid' \
    'song open music/theme.mid' \
    "song edit {\"SetProvenance\": {$provenance}}" \
    "song edit {\"AddInstrument\": {\"instrument\": {\"name\": \"first\", \"source\": {\"Sample\": {\"path\": \"sounds/blip.wav\", \"root_note\": 60}}, $provenance}}}" \
    "song edit {\"AddInstrument\": {\"instrument\": {\"name\": \"second\", \"source\": {\"Sample\": {\"path\": \"sounds/extra/blip.wav\", \"root_note\": 60}}, $provenance}}}" \
    'song save' |
    cargo run --quiet --bin editor --features editor -- "$song_project" --headless 2> "$work/song_save.log")
answered=$(grep -c '"ok":true' <<< "$song_output")
[ "$answered" -eq 6 ] || fail "the editor should answer all six song lines ok, answered $answered: $song_output $(tail -5 "$work/song_save.log")"
collision=$(find "$song_project/assets/music/samples/theme" -name 'blip-*.wav' 2> /dev/null)
[ -n "$collision" ] || fail "the save should have written the colliding sample under a <stem>-<hash> name"
if ! song_layout=$("$gate" "$song_project"); then
    fail "a real song save with a sample-name collision should pass: $song_layout"
fi

if [ "$failures" -ne 0 ]; then
    echo "project layout test: $failures failed"
    exit 1
fi
echo "project layout test: passed"
