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
| batch 1 code (draft-1-review.diff) | kimi | 6 | 5 | 0 | 1 | "`stop_source` … does `sender.take()` and then `sender.send(SpikeCommand::Stop)` — a `try_send` that returns `false` on a full channel … pressing Space again starts a *second* conductor on top of the first" — the stop button could lie | 7 m 43 s |
| batch 1 code (draft-1-review.diff) | gemini | 7 | 5 | 0 | 2 | "after `self.seek(loop_start)` … immediately executes `self.synthesizer.render(left, right)` before returning" — the spike's loop wrap skipped the downbeat for one block; recovered from the ndjson after agy's output-limit ERROR | 7 m 01 s |
| batch 2 code (draft-2-review.diff) | kimi | 5 | 4 | 0 | 1 | "the `soundfont` path (the one every song has) is never validated against the loaded resources" — the one hole in fail-loud validation; carried to batch 3 | 6 m 57 s |
| batch 2 code (draft-2-review.diff) | gemini | 5 | 3 | 0 | 2 | "`restore_notes` only checks `self.check_track(image.track)` rather than `self.check_writable_track`" — a one-identifier hardening, applied | 13 m 37 s |
| batch 3 code (draft-3-review.diff) | kimi | 4 | 1 | 2 | 1 | "the slot pool guarantee ignores tails; the stealer can still cut a live note … confirming evidence: rustysynth 1.3.8's voice-steal order" — the evidence (release priority ≤ 1) was already in rebuttal-2; and bank 128 is stored unmasked | 11 m 20 s |
| batch 3 code (draft-3-review.diff) | gemini | 6 | 3 | 0 | 3 | "`seek_to_sample` partitions events strictly with `< sample`, which excludes state events … unlike `seek_to_tick`" — a real asymmetry in the resume path, carried to batch 4 | 12 m 22 s |
| batch 4 code (draft-4-review.diff) | kimi | 4 | 3 | 0 | 1 | "`sources.admit` … runs *before* any refusal check … a full-table admission evicts the oldest **live** source … even when the trigger that caused the eviction is then refused" — the round's real catch, under-weighed by the planner's own review | 4 m 51 s |
| batch 4 code (draft-4-review.diff) | gemini | 4 | 1 | 3 | 0 | "`MusicalPosition.beat` is the 0-indexed beat within the current bar" — it is the cumulative count, as its doc says; both "critical" findings misread the code | 7 m 05 s |
| batch 5 code (draft-5-review.diff) | kimi | 4 | 2 | 1 | 1 | "`SampleVoice::render_add` ends the voice the first output frame its position reaches `last_frame` … every trigger's tail steps from ≈ `sample_end × gain` to 0 in one frame" — the fixtures' authored end fades hid it; carried to batch 6 with the unbounded read | 5 m 38 s |
| batch 5 code (draft-5-review.diff) | gemini | 3 | 0 | 1 | 2 | "Moving `sources.admit` after `pending.admit` deadlocks source eviction" — attacks round 7's accepted fix; the capacity it names frees as notes end, and the plan's overflow rule is reject-newest | 7 m 07 s |
