# Conductor — reviewer scorecard

One row per review. `real` = findings accepted; `false` = rebutted on inspection of the tree;
`policy` = conflicted with a standing ruling (the issues, DEION_STYLE §6, a decision of record);
`notable` quotes the catch worth remembering, because the review files are transient; `time`
is wall clock from the dispatch timestamp to the review file's mtime.

| subject | reviewer | findings | real | false | policy | notable | time |
|---|---|---|---|---|---|---|---|
| plan v1 (round 1) | kimi | 10 | 8 | 2 | 1 | "`build_wasm.sh` … for kind `games` it runs `cargo run -- --achievements-manifest` (line 325) on the crate and hard-fails" — the spike could not have been built as scripted | 2 m 49 s |
| plan v1 (round 1) | gemini | 7 | 7 | 0 | 0 | "independent MPSC queues provide no FIFO causal ordering across channels" — a death sound after `cmd.despawn` would be dropped; and "voices remain active … until their ADSR release envelope finishes", so the ledger had to budget release tails | 2 m 57 s |
| plan v1 (round 1) | codex | 8 | 8 | 0 | 0 | "The executor creates Tong's song at the mandated `assets/ai/music/ai_tong.mid` … browser upload rejects its intended location" — the plan's own AI-music path was outside its own save policy | 1 m 06 s |
| plan v2 (round 2) | kimi | 6 | 6 | 0 | 0 | "the host's `EntityId → SourceId` map still says 'active', so it never re-sends `ActivateSource`; every subsequent `trigger` reaches the new conductor as an unknown source" — song replacement orphaned every live source; fixed by lazy activation | 5 m 38 s |
| plan v2 (round 2) | gemini | 8 | 6 | 0 | 2 | "if >16 distinct entities trigger the same instrument within the sounding/releasing envelope window, all 16 channels are occupied" — lanes now leased while sounding; two rebuts were the issues' own loop-wrap and despawn text | 5 m 03 s |
| plan v2 (round 2) | codex | 7 | 7 | 0 | 0 | "A preset falls below −60 dB after note-off but its synth voice remains allocated longer … A sample-delta test cannot prove their absence" — the estimate was removed and the guarantee restated as a bound | 1 m 04 s |
