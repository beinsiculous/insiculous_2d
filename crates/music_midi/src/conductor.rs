//! The conductor: one song's background score and its instrument slots, rendered block by
//! block into a stereo pair, driven by transport commands from the game thread.
//!
//! The game thread builds a [`Song`] (parsing its SoundFonts there, not on the audio
//! thread), makes a [`Conductor`] with [`Conductor::new`], loads the song and hands the
//! conductor to whatever pulls audio; it keeps the [`ConductorHandle`] to send
//! [`ConductorCommand`]s and read the [`SharedPosition`].
//!
//! **Blocks.** At the start of every block of [`ConductorConfig::block_size`] frames,
//! whatever lengths [`Conductor::render`] is called with, up to
//! [`Capacities::commands_per_block`] commands are drained and the scheduled trigger
//! events due in the block are fired; the position is published at the end of each. The
//! background synthesizers and the instrument slots render on the same block, so a
//! trigger sounds on the first frame of a block — the 64-frame grid.
//!
//! **Triggers** (see [`crate::triggers`]) play a binding's part on its instrument's
//! slot for one source: a SoundFont preset's synthesizer, or a sample's voices. A seek, a loop wrap and `Play` release every slot's notes, drop
//! every scheduled event and set each part's cursor to the new position; a pause or a
//! stop releases and drops without moving the cursors, and triggers are refused until
//! the song plays again.
//!
//! **The mix** is `music × background + sfx × slots`; every output sample passes a finite
//! guard (a value that is not finite is written as 0 and counted), so nothing downstream
//! is ever handed a NaN.
//!
//! **The clock** advances by the frames rendered while playing, so it holds still only
//! when paused or stopped; zero gain does not stop it.

use std::sync::Arc;
use std::time::Duration;

use common::clock::Instant;

use crate::commands::{command_channel, CommandReceiver, ConductorCommand, ConductorHandle};
use crate::error::ConductorError;
use crate::position::{PositionSnapshot, SharedPosition, TransportState};
use crate::sequencer::{Sequencer, SequencerSink};
use crate::song_file::{InstrumentSource, PresetKey};
use crate::triggers::{DrainClock, EngineShape, Trigger, TriggerEngine};
use crate::{SAMPLE_FADE_BUDGET, SAMPLE_VOICE_LEVEL};

mod background;
mod config;
pub mod ledger;
mod sample_slot;
mod slots;
mod song;
#[cfg(test)]
mod tests;

use background::BackgroundPlayer;
pub use config::{BusGains, Capacities, ConductorConfig};
pub use ledger::{LedgerAction, LedgerCounters, NoteStart, NoteTicket, VoiceLedger};
pub use sample_slot::{SampleCounters, SampleSlot};
use sample_slot::{SampleInstrument, SampleShape};
pub use slots::{InstrumentSlot, MAXIMUM_LANES};
use slots::{Slot, SlotShape, SlotSink};
pub use song::{parse_sound_font, preset_layering, BackgroundPeak, Song};

/// Something a load let through that may still sound wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWarning {
    /// The background score starts more voices at once than its synthesizer's pool, so
    /// its voice stealer may cut a sounding note around `tick`.
    BackgroundMayOverflowPool {
        peak_voices: usize,
        tick: u64,
        polyphony: usize,
    },
}

/// What a successful load reported.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoadReport {
    pub warnings: Vec<LoadWarning>,
}

struct LoadedSong {
    song: Arc<Song>,
    sequencer: Sequencer,
    background: BackgroundPlayer,
    slots: Vec<Slot>,
    triggers: TriggerEngine,
}

/// See the module docs. `Send`, so it can move onto the audio thread.
pub struct Conductor {
    config: ConductorConfig,
    receiver: CommandReceiver,
    position: Arc<SharedPosition>,
    loaded: Option<LoadedSong>,
    transport: TransportState,
    /// Where `Resume` continues from while paused.
    resume_sample: u64,
    /// Where the next `Play` starts, set by a `Seek` while stopped: a host that wants a
    /// song to start part way sends both at once, and they may straddle a block boundary.
    start_tick: Option<u64>,
    slot_left: Vec<f32>,
    slot_right: Vec<f32>,
    scratch_left: Vec<f32>,
    scratch_right: Vec<f32>,
    frames_into_block: usize,
    blocks_rendered: u64,
    non_finite_samples: u64,
    commands_applied: u64,
}

impl Conductor {
    /// A conductor with no song, and the handle that drives it. Refuses capacities the
    /// synthesizer cannot honour: see [`ConductorError`].
    pub fn new(config: ConductorConfig) -> Result<(Self, ConductorHandle), ConductorError> {
        config.validate()?;
        let (handle, receiver, position) = command_channel(
            config.capacities.command_queue,
            config.capacities.lifecycle_reserve,
        );
        let block = || vec![0.0; config.block_size];
        let conductor = Self {
            config,
            receiver,
            position,
            loaded: None,
            transport: TransportState::Stopped,
            resume_sample: 0,
            start_tick: None,
            slot_left: block(),
            slot_right: block(),
            scratch_left: block(),
            scratch_right: block(),
            frames_into_block: 0,
            blocks_rendered: 0,
            non_finite_samples: 0,
            commands_applied: 0,
        };
        conductor.publish();
        Ok((conductor, handle))
    }

    pub fn config(&self) -> &ConductorConfig {
        &self.config
    }

    /// Load `song`, replacing any song loaded before, stopped at its start with no
    /// source live (a flush's floor carries over); the next block starts on the next frame rendered, so the slots'
    /// synthesizers keep the conductor's block grid. Refuses a song with more instruments
    /// than slots, whose instrument preset is missing or layers too thickly for the pool
    /// at the sounding cap, or whose sample instrument's sample the song was not given
    /// (`SampleNotSupplied`); warns, without refusing, when the background score alone
    /// may overflow its pool. A refused song leaves the loaded one playing. A sample
    /// slot's voices are allocated here, to the live cap plus the fade budget.
    ///
    /// Commands still queued are drained against the new song, so a host changing songs
    /// sends `StopAllSources` and loads once it is drained, or triggers resolved against
    /// the old song's bindings play the new one's.
    pub fn load(&mut self, song: Arc<Song>) -> Result<LoadReport, ConductorError> {
        let capacities = self.config.capacities;
        let instruments = &song.file().instruments;
        if instruments.len() > capacities.instrument_slots {
            return Err(ConductorError::TooManyInstruments {
                count: instruments.len(),
                capacity: capacities.instrument_slots,
            });
        }
        let shape = SlotShape {
            sample_rate: self.config.sample_rate,
            block_size: self.config.block_size,
            polyphony: capacities.polyphony,
            sounding_cap: capacities.sounding_cap,
            lanes: capacities.lanes_per_slot,
        };
        let sample_shape = SampleShape {
            output_rate: self.config.sample_rate,
            live_voice_cap: capacities.sample_voices,
            fade_budget: SAMPLE_FADE_BUDGET,
            voice_level: SAMPLE_VOICE_LEVEL,
        };
        let mut slots = Vec::with_capacity(instruments.len());
        for instrument in instruments {
            let (path, bank, preset) = match &instrument.source {
                InstrumentSource::SoundFont { path, bank, preset } => (path, bank, preset),
                InstrumentSource::Sample {
                    path,
                    root_note,
                    gain,
                    playback,
                } => {
                    let pcm = song.samples().get(path).ok_or_else(|| {
                        ConductorError::SampleNotSupplied {
                            instrument: instrument.name.clone(),
                            path: path.clone(),
                        }
                    })?;
                    let sample = SampleInstrument {
                        pcm: pcm.clone(),
                        root_note: *root_note,
                        gain: *gain,
                        playback: *playback,
                    };
                    slots.push(Slot::Sample(SampleSlot::new(sample, sample_shape)));
                    continue;
                }
            };
            let missing = || ConductorError::PresetMissing {
                instrument: instrument.name.clone(),
                soundfont: path.clone(),
                bank: *bank,
                preset: *preset,
            };
            let key = PresetKey {
                soundfont: path.clone(),
                bank: *bank,
                preset: *preset,
            };
            let layering = song.instrument_layering(&key).ok_or_else(missing)?;
            let needed = layering.saturating_mul(capacities.sounding_cap);
            if needed > capacities.polyphony {
                return Err(ConductorError::LayeringExceedsPool {
                    instrument: instrument.name.clone(),
                    layering,
                    sounding_cap: capacities.sounding_cap,
                    needed,
                    polyphony: capacities.polyphony,
                });
            }
            let sound_font = song.sound_font(path).ok_or_else(missing)?;
            slots.push(Slot::Synth(Box::new(InstrumentSlot::new(
                sound_font, shape, *bank, *preset,
            )?)));
        }
        let background_font = song.sound_font(&song.file().soundfont).ok_or_else(|| {
            ConductorError::SoundFontNotSupplied {
                path: song.file().soundfont.clone(),
            }
        })?;
        let background = BackgroundPlayer::new(
            background_font,
            self.config.sample_rate,
            self.config.block_size,
            capacities.polyphony,
        )?;
        let mut triggers = TriggerEngine::new(
            &song,
            EngineShape {
                sources: capacities.sources,
                pending_events: capacities.pending_events,
                sounding_cap: capacities.sounding_cap,
                sample_voices: capacities.sample_voices,
            },
        )?;
        if let Some(previous) = &self.loaded {
            triggers.keep_flush_floor_of(&previous.triggers);
        }
        let sequencer = Sequencer::new(
            Arc::clone(song.background()),
            self.config.sample_rate,
            self.config.block_size,
            song.file().looping,
        );
        let mut report = LoadReport::default();
        let peak = song.background_peak();
        if peak.voices > capacities.polyphony {
            report
                .warnings
                .push(LoadWarning::BackgroundMayOverflowPool {
                    peak_voices: peak.voices,
                    tick: peak.tick,
                    polyphony: capacities.polyphony,
                });
        }
        self.loaded = Some(LoadedSong {
            song,
            sequencer,
            background,
            slots,
            triggers,
        });
        self.transport = TransportState::Stopped;
        self.resume_sample = 0;
        self.start_tick = None;
        self.frames_into_block = 0;
        self.publish();
        Ok(report)
    }

    /// The loaded song, if any.
    pub fn song(&self) -> Option<&Arc<Song>> {
        self.loaded.as_ref().map(|loaded| &loaded.song)
    }

    /// The unit part `part`'s cursor plays next (`Loop`) or may play first
    /// (`Sequence`); `None` without a song or past the parts.
    pub fn part_cursor(&self, part: usize) -> Option<usize> {
        self.loaded.as_ref()?.triggers.cursor_of(part)
    }

    /// The synthesizer slot of the song's `index`th instrument, when that is a SoundFont
    /// preset.
    pub fn slot_mut(&mut self, index: usize) -> Option<&mut InstrumentSlot> {
        match self.loaded.as_mut()?.slots.get_mut(index)? {
            Slot::Synth(slot) => Some(slot.as_mut()),
            Slot::Sample(_) => None,
        }
    }

    /// The sample slot of the song's `index`th instrument, when that is a sample.
    pub fn sample_slot_mut(&mut self, index: usize) -> Option<&mut SampleSlot> {
        match self.loaded.as_mut()?.slots.get_mut(index)? {
            Slot::Sample(slot) => Some(slot),
            Slot::Synth(_) => None,
        }
    }

    /// Render `left.len()` frames into `left` and `right` (overwritten; a longer one keeps
    /// its extra frames untouched), applying commands at every block boundary with the
    /// wall clock read now. Without a song it renders silence.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32], buses: BusGains) {
        self.render_at(Instant::now(), left, right, buses);
    }

    /// [`Conductor::render`] with `now` as the wall clock of this call's first frame. Each
    /// block drains at `now` plus the frames rendered before it in this call, so a long
    /// pull reads its blocks as the instants they play at: a trigger's age is that
    /// instant minus its stamp.
    pub fn render_at(
        &mut self,
        now: Instant,
        left: &mut [f32],
        right: &mut [f32],
        buses: BusGains,
    ) {
        let frames = left.len().min(right.len());
        let sample_rate = f64::from(self.config.sample_rate);
        let mut offset = 0;
        while offset < frames {
            if self.frames_into_block == 0 {
                let block_now = now + Duration::from_secs_f64(offset as f64 / sample_rate);
                self.begin_block(block_now);
            }
            let chunk = (self.config.block_size - self.frames_into_block).min(frames - offset);
            let range = offset..offset + chunk;
            self.render_chunk(&mut left[range.clone()], &mut right[range], buses);
            self.frames_into_block = (self.frames_into_block + chunk) % self.config.block_size;
            if self.frames_into_block == 0 {
                self.blocks_rendered += 1;
                self.publish();
            }
            offset += chunk;
        }
    }

    /// Apply one command now, as a block boundary would, reading the wall clock now.
    pub fn apply(&mut self, command: ConductorCommand) {
        self.apply_at(command, Instant::now());
    }

    fn apply_at(&mut self, command: ConductorCommand, now: Instant) {
        self.commands_applied += 1;
        let Some(loaded) = self.loaded.as_mut() else {
            return;
        };
        match command {
            ConductorCommand::Play => {
                let tick = self.start_tick.take().unwrap_or(0);
                loaded.release_slots();
                loaded.sequencer.seek_to_tick(tick, &mut loaded.background);
                loaded.triggers.restart_at(tick, &loaded.song);
                self.transport = TransportState::Playing;
            }
            ConductorCommand::Stop => {
                loaded.release_all();
                loaded.triggers.clear_pending();
                self.transport = TransportState::Stopped;
            }
            ConductorCommand::Pause if self.transport == TransportState::Playing => {
                loaded.release_all();
                loaded.triggers.clear_pending();
                self.resume_sample = loaded.sequencer.position_sample();
                self.transport = TransportState::Paused;
            }
            ConductorCommand::Resume if self.transport == TransportState::Paused => {
                loaded
                    .sequencer
                    .seek_to_sample(self.resume_sample, &mut loaded.background);
                self.transport = TransportState::Playing;
            }
            ConductorCommand::Pause | ConductorCommand::Resume => {}
            ConductorCommand::Seek(tick) => match self.transport {
                TransportState::Playing => {
                    loaded.release_slots();
                    loaded.sequencer.seek_to_tick(tick, &mut loaded.background);
                    loaded.triggers.restart_at(tick, &loaded.song);
                }
                // The sequencer moves too, so a stop after this publishes the seeked
                // position rather than the one the pause held.
                TransportState::Paused => {
                    loaded.sequencer.seek_to_tick(tick, &mut loaded.background);
                    self.resume_sample = loaded.sequencer.position_sample();
                    loaded.triggers.restart_at(tick, &loaded.song);
                }
                TransportState::Stopped => self.start_tick = Some(tick),
            },
            ConductorCommand::SetLooping(looping) => loaded.sequencer.set_looping(looping),
            ConductorCommand::Trigger {
                binding,
                source,
                stamp,
            } => {
                let clock = DrainClock {
                    song: &loaded.song,
                    tempo_map: loaded.song.background().tempo_map(),
                    sample_rate: self.config.sample_rate,
                    block_frames: self.config.block_size,
                    drained_at: now,
                    position_sample: loaded.sequencer.position_sample(),
                    playing: self.transport == TransportState::Playing,
                };
                let trigger = Trigger {
                    binding,
                    source,
                    stamp,
                };
                loaded
                    .triggers
                    .trigger(trigger, &clock, &mut SlotSink(&mut loaded.slots));
            }
            ConductorCommand::DeactivateSource { source } => loaded
                .triggers
                .deactivate(source, &mut SlotSink(&mut loaded.slots)),
            ConductorCommand::StopAllSources { flush_epoch } => loaded
                .triggers
                .flush(flush_epoch, &mut SlotSink(&mut loaded.slots)),
        }
    }

    /// Drain this block's commands, then fire the scheduled events due in it — up to
    /// the loop seam, since a wrap drops what lies beyond.
    fn begin_block(&mut self, now: Instant) {
        for _ in 0..self.config.capacities.commands_per_block {
            let Some(command) = self.receiver.try_recv() else {
                break;
            };
            self.apply_at(command, now);
        }
        if self.transport != TransportState::Playing {
            return;
        }
        let Some(loaded) = self.loaded.as_mut() else {
            return;
        };
        let block_end = loaded.sequencer.position_sample() + self.config.block_size as u64;
        let due_before = block_end.min(loaded.sequencer.end_sample());
        loaded
            .triggers
            .fire_due(due_before, &mut SlotSink(&mut loaded.slots));
    }

    fn render_chunk(&mut self, left: &mut [f32], right: &mut [f32], buses: BusGains) {
        let frames = left.len();
        let Some(loaded) = self.loaded.as_mut() else {
            left.fill(0.0);
            right.fill(0.0);
            return;
        };
        loaded.background.begin_block();
        if self.transport == TransportState::Playing {
            let wraps_before = loaded.sequencer.loop_wraps();
            loaded.sequencer.advance(frames, &mut loaded.background);
            if loaded.sequencer.loop_wraps() != wraps_before {
                loaded.release_slots();
                loaded.triggers.restart_at(0, &loaded.song);
            }
            if loaded.sequencer.has_ended() {
                loaded.release_slots();
                loaded.triggers.clear_pending();
                self.transport = TransportState::Stopped;
            }
        } else {
            loaded.background.render(frames);
        }
        let slot_left = &mut self.slot_left[..frames];
        let slot_right = &mut self.slot_right[..frames];
        slot_left.fill(0.0);
        slot_right.fill(0.0);
        for slot in &mut loaded.slots {
            slot.mix_into(
                slot_left,
                slot_right,
                &mut self.scratch_left[..frames],
                &mut self.scratch_right[..frames],
            );
        }
        let (background_left, background_right) = loaded.background.output();
        for (output, (background, slots)) in left
            .iter_mut()
            .zip(background_left.iter().zip(slot_left.iter()))
            .chain(
                right
                    .iter_mut()
                    .zip(background_right.iter().zip(slot_right.iter())),
            )
        {
            let mixed = buses.music * background + buses.sfx * slots;
            *output = if mixed.is_finite() {
                mixed
            } else {
                self.non_finite_samples += 1;
                0.0
            };
        }
    }

    fn publish(&self) {
        let mut snapshot = PositionSnapshot {
            transport: self.transport,
            blocks_rendered: self.blocks_rendered,
            non_finite_samples: self.non_finite_samples,
            commands_applied: self.commands_applied,
            ..PositionSnapshot::default()
        };
        if let Some(loaded) = &self.loaded {
            let sequencer = &loaded.sequencer;
            snapshot.sample_position = if self.transport == TransportState::Paused {
                self.resume_sample
            } else {
                sequencer.position_sample()
            };
            snapshot.tick = sequencer
                .score()
                .tempo_map()
                .tick_at_sample(snapshot.sample_position, self.config.sample_rate);
            snapshot.ended = sequencer.has_ended();
            snapshot.reconstructions = sequencer.reconstructions();
            snapshot.loop_wraps = sequencer.loop_wraps();
            let triggers = loaded.triggers.counters();
            snapshot.accepted_triggers = triggers.accepted;
            snapshot.rejected_triggers = triggers.rejected;
            snapshot.late_triggers = triggers.late;
            snapshot.stale_triggers = triggers.stale;
            snapshot.silent_triggers = triggers.silent;
            snapshot.live_sources = loaded.triggers.live_sources() as u64;
            snapshot.pending_events = loaded.triggers.pending_len() as u64;
            snapshot.sample_voices = loaded
                .slots
                .iter()
                .map(Slot::sample_voice_count)
                .sum::<usize>() as u64;
        }
        self.position.publish(&snapshot);
    }
}

impl LoadedSong {
    fn release_all(&mut self) {
        self.background.release_all();
        self.release_slots();
    }

    fn release_slots(&mut self) {
        for slot in &mut self.slots {
            slot.release_all();
        }
    }
}
