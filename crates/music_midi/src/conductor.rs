//! The conductor: one song's background score and its instrument slots, rendered block by
//! block into a stereo pair, driven by transport commands from the game thread.
//!
//! The game thread builds a [`Song`] (parsing its SoundFonts there, not on the audio
//! thread), makes a [`Conductor`] with [`Conductor::new`], loads the song and hands the
//! conductor to whatever pulls audio; it keeps the [`ConductorHandle`] to send
//! [`ConductorCommand`]s and read the [`SharedPosition`].
//!
//! **Blocks.** Commands are drained at the start of every block of
//! [`ConductorConfig::block_size`] frames, whatever lengths [`Conductor::render`] is
//! called with, and the position is published at the end of each. The background
//! synthesizers render on the same block, which is the sequencer's grain.
//!
//! **The mix** is `music × background + sfx × slots`; every output sample passes a finite
//! guard (a value that is not finite is written as 0 and counted), so nothing downstream
//! is ever handed a NaN.
//!
//! **The clock** advances by the frames rendered while playing, so it holds still only
//! when paused or stopped; zero gain does not stop it.

use std::sync::mpsc::Receiver;
use std::sync::Arc;

use crate::commands::{command_channel, ConductorCommand, ConductorHandle};
use crate::error::ConductorError;
use crate::position::{PositionSnapshot, SharedPosition, TransportState};
use crate::sequencer::{Sequencer, SequencerSink};
use crate::song_file::{InstrumentSource, PresetKey};
use crate::{
    BLOCK_FRAMES, COMMAND_QUEUE_CAPACITY, INSTRUMENT_SLOT_CAPACITY, LANES_PER_SLOT,
    PENDING_EVENT_CAPACITY, SAMPLE_VOICE_CAP, SOUNDING_NOTE_CAP, SYNTHESIZER_POLYPHONY,
};

mod background;
pub mod ledger;
mod slots;
mod song;
#[cfg(test)]
mod tests;

use background::BackgroundPlayer;
pub use ledger::{LedgerAction, LedgerCounters, NoteStart, SourceKey, VoiceLedger};
use slots::SlotShape;
pub use slots::{InstrumentSlot, MAXIMUM_LANES};
pub use song::{BackgroundPeak, Song};

const SAMPLE_RATES: std::ops::RangeInclusive<u32> = 16_000..=192_000;
const BLOCK_SIZES: std::ops::RangeInclusive<usize> = 8..=1024;
const POLYPHONIES: std::ops::RangeInclusive<usize> = 8..=256;

/// Every fixed capacity of a conductor. The defaults are the measured, frozen values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacities {
    pub instrument_slots: usize,
    pub lanes_per_slot: usize,
    pub sounding_cap: usize,
    /// Voices in each synthesizer's pool, release tails included.
    pub polyphony: usize,
    pub sample_voices: usize,
    pub command_queue: usize,
    pub pending_events: usize,
}

impl Default for Capacities {
    fn default() -> Self {
        Self {
            instrument_slots: INSTRUMENT_SLOT_CAPACITY,
            lanes_per_slot: LANES_PER_SLOT,
            sounding_cap: SOUNDING_NOTE_CAP,
            polyphony: SYNTHESIZER_POLYPHONY,
            sample_voices: SAMPLE_VOICE_CAP,
            command_queue: COMMAND_QUEUE_CAPACITY,
            pending_events: PENDING_EVENT_CAPACITY,
        }
    }
}

/// How a conductor is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConductorConfig {
    pub sample_rate: u32,
    pub block_size: usize,
    pub capacities: Capacities,
}

impl ConductorConfig {
    /// The frozen block and capacities at `sample_rate`.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            block_size: BLOCK_FRAMES,
            capacities: Capacities::default(),
        }
    }

    fn validate(&self) -> Result<(), ConductorError> {
        let capacities = &self.capacities;
        let named = [
            ("instrument slots", capacities.instrument_slots),
            ("lanes per slot", capacities.lanes_per_slot),
            ("sounding cap", capacities.sounding_cap),
            ("polyphony", capacities.polyphony),
            ("sample voices", capacities.sample_voices),
            ("command queue", capacities.command_queue),
            ("pending events", capacities.pending_events),
        ];
        if let Some((capacity, _)) = named.iter().find(|(_, value)| *value == 0) {
            return Err(ConductorError::ZeroCapacity { capacity });
        }
        if !SAMPLE_RATES.contains(&self.sample_rate) {
            return Err(ConductorError::SampleRateOutOfRange {
                sample_rate: self.sample_rate,
            });
        }
        if !BLOCK_SIZES.contains(&self.block_size) {
            return Err(ConductorError::BlockSizeOutOfRange {
                block_size: self.block_size,
            });
        }
        if !POLYPHONIES.contains(&capacities.polyphony) {
            return Err(ConductorError::PolyphonyOutOfRange {
                polyphony: capacities.polyphony,
            });
        }
        if capacities.polyphony < capacities.sounding_cap {
            return Err(ConductorError::PoolBelowSoundingCap {
                polyphony: capacities.polyphony,
                sounding_cap: capacities.sounding_cap,
            });
        }
        if capacities.lanes_per_slot > MAXIMUM_LANES {
            return Err(ConductorError::TooManyLanes {
                lanes: capacities.lanes_per_slot,
                maximum: MAXIMUM_LANES,
            });
        }
        Ok(())
    }
}

/// The two bus gains the conductor mixes under; master volume is the output's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BusGains {
    pub music: f32,
    pub sfx: f32,
}

impl BusGains {
    pub const UNITY: BusGains = BusGains {
        music: 1.0,
        sfx: 1.0,
    };
}

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
    slots: Vec<InstrumentSlot>,
}

/// See the module docs. `Send`, so it can move onto the audio thread.
pub struct Conductor {
    config: ConductorConfig,
    receiver: Receiver<ConductorCommand>,
    position: Arc<SharedPosition>,
    loaded: Option<LoadedSong>,
    transport: TransportState,
    /// Where `Resume` continues from while paused.
    resume_sample: u64,
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
        let (handle, receiver, position) = command_channel(config.capacities.command_queue);
        let block = || vec![0.0; config.block_size];
        let conductor = Self {
            config,
            receiver,
            position,
            loaded: None,
            transport: TransportState::Stopped,
            resume_sample: 0,
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

    /// Load `song`, replacing any song loaded before, stopped at its start. Refuses a
    /// song with more instruments than slots, or whose instrument preset is missing or
    /// layers too thickly for the pool at the sounding cap; warns, without refusing, when
    /// the background score alone may overflow its pool. A sample instrument takes a
    /// slot in the count but plays nothing yet.
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
        let mut slots = Vec::with_capacity(instruments.len());
        for instrument in instruments {
            let InstrumentSource::SoundFont { path, bank, preset } = &instrument.source else {
                continue;
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
            slots.push(InstrumentSlot::new(sound_font, shape, *bank, *preset)?);
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
        });
        self.transport = TransportState::Stopped;
        self.resume_sample = 0;
        self.publish();
        Ok(report)
    }

    /// The loaded song, if any.
    pub fn song(&self) -> Option<&Arc<Song>> {
        self.loaded.as_ref().map(|loaded| &loaded.song)
    }

    /// The slot of the `index`th SoundFont instrument the song declares.
    pub fn slot_mut(&mut self, index: usize) -> Option<&mut InstrumentSlot> {
        self.loaded.as_mut()?.slots.get_mut(index)
    }

    /// Render `left.len()` frames into `left` and `right` (overwritten; a longer one keeps
    /// its extra frames untouched), applying commands at every block boundary. Without a
    /// song it renders silence.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32], buses: BusGains) {
        let frames = left.len().min(right.len());
        let mut offset = 0;
        while offset < frames {
            if self.frames_into_block == 0 {
                self.drain_commands();
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

    /// Apply one command now, as a block boundary would.
    pub fn apply(&mut self, command: ConductorCommand) {
        self.commands_applied += 1;
        let Some(loaded) = self.loaded.as_mut() else {
            return;
        };
        match command {
            ConductorCommand::Play => {
                loaded.sequencer.seek_to_tick(0, &mut loaded.background);
                self.transport = TransportState::Playing;
            }
            ConductorCommand::Stop => {
                loaded.release_all();
                self.transport = TransportState::Stopped;
            }
            ConductorCommand::Pause if self.transport == TransportState::Playing => {
                loaded.release_all();
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
                    loaded.sequencer.seek_to_tick(tick, &mut loaded.background);
                }
                TransportState::Paused => {
                    let score = loaded.sequencer.score();
                    self.resume_sample = score
                        .tempo_map()
                        .sample_at_tick(tick.min(score.end_tick()), self.config.sample_rate);
                }
                TransportState::Stopped => {}
            },
            ConductorCommand::SetLooping(looping) => loaded.sequencer.set_looping(looping),
        }
    }

    fn drain_commands(&mut self) {
        while let Ok(command) = self.receiver.try_recv() {
            self.apply(command);
        }
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
            loaded.sequencer.advance(frames, &mut loaded.background);
            if loaded.sequencer.has_ended() {
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
            let scratch_left = &mut self.scratch_left[..frames];
            let scratch_right = &mut self.scratch_right[..frames];
            slot.render(scratch_left, scratch_right);
            for (mixed, rendered) in slot_left.iter_mut().zip(scratch_left.iter()) {
                *mixed += rendered;
            }
            for (mixed, rendered) in slot_right.iter_mut().zip(scratch_right.iter()) {
                *mixed += rendered;
            }
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
        }
        self.position.publish(&snapshot);
    }
}

impl LoadedSong {
    fn release_all(&mut self) {
        self.background.release_all();
        for slot in &mut self.slots {
            slot.release_all();
        }
    }
}
