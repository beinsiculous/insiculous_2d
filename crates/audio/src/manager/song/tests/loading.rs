//! Loading a song's closure: what refuses it, that a refusal leaves the playing song
//! alone, and what the decoded-file cache keeps.

use std::sync::Arc;

use super::*;
use crate::manager::song::load::{SongProblem, SOUND_FONT_BYTE_LIMIT};

/// A sidecar naming one more instrument, `extra`, playing `source`, bound to `extra`.
fn sidecar_with(source: &str) -> String {
    sidecar(Some((
        &format!(r#"(name: "extra", source: {source}),"#),
        r#"(action: "extra", part: "melody", instrument: "extra"),"#,
    )))
}

fn grow_sparsely(path: &std::path::Path, bytes: usize) {
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|file| file.set_len(bytes as u64))
        .expect("the file grows sparsely");
}

#[test]
fn a_missing_sample_a_missing_bank_or_an_oversized_file_is_refused_and_the_song_plays_on() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    manager.update_at(start + seconds(0.1));
    let playing_label = manager
        .songs
        .playback
        .as_ref()
        .map(|playback| playback.label.clone());

    let missing_sample = assets.write_song(
        "missing_sample",
        &song_midi(true),
        &sidecar_with(r#"Sample(path: "samples/absent.wav", root_note: 60)"#),
    );
    let missing_bank = assets.write_song(
        "missing_bank",
        &song_midi(true),
        &sidecar_with(r#"SoundFont(path: "absent.sf2", bank: 0, preset: 0)"#),
    );
    let oversized_bank = assets.write("huge.sf2", SINE_FONT);
    grow_sparsely(&oversized_bank, SOUND_FONT_BYTE_LIMIT + 1);
    let oversized = assets.write_song(
        "oversized",
        &song_midi(true),
        &sidecar_with(r#"SoundFont(path: "huge.sf2", bank: 0, preset: 0)"#),
    );
    let absent_preset = assets.write_song(
        "absent_preset",
        &song_midi(true),
        &sidecar_with(&format!(
            r#"SoundFont(path: "{FONT_PATH}", bank: 0, preset: 99)"#
        )),
    );

    for (song, names) in [
        (&missing_sample, "samples/absent.wav"),
        (&missing_bank, "absent.sf2"),
        (&oversized, "huge.sf2"),
        (&absent_preset, "extra"),
    ] {
        let error = manager.play_song(song).expect_err("the song is refused");
        let message = error.to_string();
        assert!(message.contains(names), "{message}");
        assert!(
            message.contains(&song.display().to_string()),
            "names the song: {message}"
        );
        if song != &absent_preset {
            assert!(
                message.contains("instruments['extra']"),
                "names the entry: {message}"
            );
        }
        assert!(
            manager.is_song_playing(),
            "{} left the song playing",
            song.display()
        );
        assert_eq!(
            manager
                .songs
                .playback
                .as_ref()
                .map(|playback| playback.label.clone()),
            playing_label
        );
    }
    let error = manager.play_song(&oversized).expect_err("refused");
    assert!(matches!(
        error,
        AudioError::Song { problem, .. }
            if matches!(*problem, SongProblem::TooManyBytes { limit, .. } if limit == SOUND_FONT_BYTE_LIMIT)
    ));
    let before = manager.song_position().expect("playing").seconds;
    manager.update_at(start + seconds(0.3));
    assert!(
        manager.song_position().expect("playing").seconds > before,
        "and its clock runs"
    );
}

#[test]
fn a_missing_sidecar_is_refused_naming_it_without_touching_recorded_music() {
    let assets = AssetDirectory::new();
    let mut manager = AudioManager::awaiting_gesture();
    manager.set_asset_base(assets.path());
    let recorded = write_recorded_track(&assets);
    manager.play_music(&recorded).expect("the track decodes");
    let lonely = assets.write("lonely.mid", &song_midi(true));

    let error = manager.play_song(&lonely).expect_err("no sidecar");

    assert!(error.to_string().contains("lonely.mid.ron"), "{error}");
    assert!(matches!(
        manager.pending_music,
        Some(PendingMusic::Recorded { .. })
    ));
}

#[test]
fn unchanged_files_are_decoded_once_and_a_file_changed_at_its_path_is_decoded_again() {
    let assets = AssetDirectory::new();
    let (mut manager, _) = playing(&assets);
    let song_path = assets.path().join("song.mid");
    let bank_of = |manager: &AudioManager| {
        let song = &manager.songs.playback.as_ref().expect("playing").song;
        Arc::clone(song.sound_font(FONT_PATH).expect("its bank"))
    };
    let sample_of = |manager: &AudioManager| {
        let song = &manager.songs.playback.as_ref().expect("playing").song;
        song.samples().get(SAMPLE_PATH).expect("its sample").clone()
    };
    assert_eq!(manager.songs.cache.decodes(), 2, "one bank and one sample");
    let (first_bank, first_sample) = (bank_of(&manager), sample_of(&manager));

    manager.play_song(&song_path).expect("the same song again");
    assert_eq!(manager.songs.cache.decodes(), 2, "nothing decoded again");
    assert!(Arc::ptr_eq(&first_bank, &bank_of(&manager)));
    assert!(first_sample.shares_samples_with(&sample_of(&manager)));

    assets.write(SAMPLE_PATH, OTHER_SINE_WAV);
    manager
        .play_song(&song_path)
        .expect("the song with a changed sample");
    assert_eq!(
        manager.songs.cache.decodes(),
        3,
        "only the changed sample decoded"
    );
    assert!(
        Arc::ptr_eq(&first_bank, &bank_of(&manager)),
        "the bank is still shared"
    );
    let changed = sample_of(&manager);
    assert!(!first_sample.shares_samples_with(&changed));
    assert_eq!(changed.sample_rate(), 22_050, "the new file's audio plays");
}

#[test]
fn a_song_built_in_memory_shares_the_decoded_files_of_the_song_on_disk() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    let bank = {
        let song = &manager.songs.playback.as_ref().expect("playing").song;
        Arc::clone(song.sound_font(FONT_PATH).expect("its bank"))
    };
    let file = music_midi::parse_song_file("composer", &sidecar(None)).expect("parses");
    let (data, _) = SongData::from_smf(&song_midi(false)).expect("reads");

    let song = manager
        .prepare_song("the composer's song", file, data)
        .expect("builds");
    manager
        .play_song_in_memory(Arc::clone(&song))
        .expect("plays");
    manager.update_at(start + seconds(0.05));

    assert_eq!(
        manager.songs.cache.decodes(),
        2,
        "nothing decoded for the preview"
    );
    assert!(Arc::ptr_eq(
        &bank,
        song.sound_font(FONT_PATH).expect("its bank")
    ));
    assert!(manager.is_song_playing());
    assert!(manager.trigger("tone", keyboard()).is_ok());
}

#[test]
fn a_recorded_track_that_fails_to_load_leaves_a_playing_or_pending_song_alone() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    manager.update_at(start + seconds(0.1));
    let missing = assets.path().join("missing.wav");
    let undecodable = assets.write("noise.wav", b"not a wav at all");

    manager.play_music(&missing).expect_err("no such file");
    manager.play_music(&undecodable).expect_err("not decodable");

    assert!(manager.is_song_playing(), "the song plays on");
    assert!(!manager.is_music_playing());

    let mut gated = AudioManager::awaiting_gesture();
    gated.set_asset_base(assets.path());
    let song = assets.write_song("pending", &song_midi(true), &sidecar(None));
    gated.play_song(&song).expect("the song loads and waits for the gesture");

    gated.play_music(&missing).expect_err("no such file");

    assert!(matches!(gated.pending_music, Some(PendingMusic::Song(_))));
}
