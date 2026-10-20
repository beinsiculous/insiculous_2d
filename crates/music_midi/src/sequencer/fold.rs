//! What every MIDI channel looks like at a point in the score — the state a seek or a
//! loop wrap must restore on a synthesizer that starts from its defaults.
//!
//! A fold records only what the score set: an entry the score never touched stays unset,
//! and a freshly reset synthesizer already holds its default. Replaying a fold sends the
//! bank, the program, every ordinary controller, the registered parameters and the
//! pitch bend, in an order that rebuilds them: a controller list replayed by number would
//! send data entry (6) before the parameter selection (100, 101) it applies to, and a
//! pitch-bend range would come back wrong.

use super::score::{ChannelMessage, ScoreEvent};

/// MIDI's sixteen channels.
pub const CHANNEL_COUNT: usize = 16;

const BANK_SELECT: u8 = 0;
const DATA_ENTRY: u8 = 6;
const BANK_SELECT_FINE: u8 = 32;
const DATA_ENTRY_FINE: u8 = 38;
const DATA_INCREMENT: u8 = 96;
const DATA_DECREMENT: u8 = 97;
const NON_REGISTERED_PARAMETER_FINE: u8 = 98;
const NON_REGISTERED_PARAMETER_COARSE: u8 = 99;
const REGISTERED_PARAMETER_FINE: u8 = 100;
const REGISTERED_PARAMETER_COARSE: u8 = 101;
/// Controllers 120 to 127 are channel mode messages: actions such as all-notes-off, not
/// state, so replaying one at the destination would cut what the fold restores.
const FIRST_CHANNEL_MODE_MESSAGE: u8 = 120;
const RESET_ALL_CONTROLLERS: u8 = 121;
/// What reset-all-controllers returns to its default, besides the pitch bend and the
/// parameter selection: modulation, expression and the hold pedal.
const RESET_BY_RESET_ALL_CONTROLLERS: [u8; 3] = [1, 11, 64];
/// The registered parameters the synthesizer honours: pitch-bend range, fine tuning and
/// coarse tuning. Data entry for any other parameter changes nothing it plays.
const HONOURED_REGISTERED_PARAMETERS: usize = 3;
/// MIDI's "no parameter selected" value for either half of a parameter number.
const NULL_PARAMETER_HALF: u8 = 127;

/// Which parameter number a data entry applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParameterSelection {
    Unselected,
    Registered,
    NonRegistered,
}

/// One registered parameter's data entry, coarse and fine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct RegisteredParameter {
    coarse: Option<u8>,
    fine: Option<u8>,
}

/// One channel's state as the score left it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ChannelState {
    program: Option<u8>,
    bank: Option<u8>,
    bank_fine: Option<u8>,
    /// Every controller that is plain state, by number; selection, data entry, bank and
    /// mode messages are kept apart.
    controllers: [Option<u8>; 128],
    pitch_bend: Option<u16>,
    registered_parameters: [RegisteredParameter; HONOURED_REGISTERED_PARAMETERS],
    selection: ParameterSelection,
    registered_coarse: u8,
    registered_fine: u8,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            program: None,
            bank: None,
            bank_fine: None,
            controllers: [None; 128],
            pitch_bend: None,
            registered_parameters: [RegisteredParameter::default(); HONOURED_REGISTERED_PARAMETERS],
            selection: ParameterSelection::Unselected,
            registered_coarse: NULL_PARAMETER_HALF,
            registered_fine: NULL_PARAMETER_HALF,
        }
    }
}

impl ChannelState {
    fn registered_number(&self) -> usize {
        usize::from(self.registered_coarse) << 7 | usize::from(self.registered_fine)
    }

    fn apply_controller(&mut self, controller: u8, value: u8) {
        match controller {
            BANK_SELECT => self.bank = Some(value),
            BANK_SELECT_FINE => self.bank_fine = Some(value),
            REGISTERED_PARAMETER_COARSE => {
                self.registered_coarse = value;
                self.selection = ParameterSelection::Registered;
            }
            REGISTERED_PARAMETER_FINE => {
                self.registered_fine = value;
                self.selection = ParameterSelection::Registered;
            }
            NON_REGISTERED_PARAMETER_COARSE | NON_REGISTERED_PARAMETER_FINE => {
                self.selection = ParameterSelection::NonRegistered;
            }
            DATA_ENTRY | DATA_ENTRY_FINE => {
                let number = self.registered_number();
                if self.selection != ParameterSelection::Registered
                    || number >= HONOURED_REGISTERED_PARAMETERS
                {
                    return;
                }
                let parameter = &mut self.registered_parameters[number];
                if controller == DATA_ENTRY {
                    parameter.coarse = Some(value);
                } else {
                    parameter.fine = Some(value);
                }
            }
            DATA_INCREMENT | DATA_DECREMENT => {}
            RESET_ALL_CONTROLLERS => {
                for reset in RESET_BY_RESET_ALL_CONTROLLERS {
                    self.controllers[usize::from(reset)] = None;
                }
                self.pitch_bend = None;
                self.selection = ParameterSelection::Unselected;
                self.registered_coarse = NULL_PARAMETER_HALF;
                self.registered_fine = NULL_PARAMETER_HALF;
            }
            mode if mode >= FIRST_CHANNEL_MODE_MESSAGE => {}
            plain => self.controllers[usize::from(plain & 0x7F)] = Some(value),
        }
    }

    fn replay(&self, emit: &mut impl FnMut(ChannelMessage)) {
        let controller = |controller, value| ChannelMessage::Controller { controller, value };
        if let Some(bank) = self.bank {
            emit(controller(BANK_SELECT, bank));
        }
        if let Some(bank_fine) = self.bank_fine {
            emit(controller(BANK_SELECT_FINE, bank_fine));
        }
        if let Some(program) = self.program {
            emit(ChannelMessage::Program { program });
        }
        for (number, value) in self.controllers.iter().enumerate() {
            if let Some(value) = value {
                emit(controller(number as u8, *value));
            }
        }
        let mut replayed_a_parameter = false;
        for (number, parameter) in self.registered_parameters.iter().enumerate() {
            if parameter.coarse.is_none() && parameter.fine.is_none() {
                continue;
            }
            replayed_a_parameter = true;
            emit(controller(REGISTERED_PARAMETER_COARSE, (number >> 7) as u8));
            emit(controller(REGISTERED_PARAMETER_FINE, (number & 0x7F) as u8));
            if let Some(coarse) = parameter.coarse {
                emit(controller(DATA_ENTRY, coarse));
            }
            if let Some(fine) = parameter.fine {
                emit(controller(DATA_ENTRY_FINE, fine));
            }
        }
        match self.selection {
            ParameterSelection::Registered => {
                emit(controller(
                    REGISTERED_PARAMETER_COARSE,
                    self.registered_coarse,
                ));
                emit(controller(REGISTERED_PARAMETER_FINE, self.registered_fine));
            }
            ParameterSelection::NonRegistered => {
                emit(controller(
                    NON_REGISTERED_PARAMETER_COARSE,
                    NULL_PARAMETER_HALF,
                ));
                emit(controller(
                    NON_REGISTERED_PARAMETER_FINE,
                    NULL_PARAMETER_HALF,
                ));
            }
            ParameterSelection::Unselected if replayed_a_parameter => {
                emit(controller(REGISTERED_PARAMETER_COARSE, NULL_PARAMETER_HALF));
                emit(controller(REGISTERED_PARAMETER_FINE, NULL_PARAMETER_HALF));
            }
            ParameterSelection::Unselected => {}
        }
        if let Some(value) = self.pitch_bend {
            emit(ChannelMessage::PitchBend { value });
        }
    }
}

/// Every channel's state folded from the start of a score: see the module docs. Two
/// folds are equal when the same replay would leave a synthesizer in the same state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChannelStateFold {
    channels: [ChannelState; CHANNEL_COUNT],
}

impl ChannelStateFold {
    /// Forget everything: the state of a synthesizer just reset.
    pub fn clear(&mut self) {
        self.channels = Default::default();
    }

    /// Fold one message into the state. Notes change no state and are ignored.
    pub fn apply(&mut self, channel: u8, message: ChannelMessage) {
        let state = &mut self.channels[usize::from(channel) % CHANNEL_COUNT];
        match message {
            ChannelMessage::Controller { controller, value } => {
                state.apply_controller(controller, value);
            }
            ChannelMessage::Program { program } => state.program = Some(program),
            ChannelMessage::PitchBend { value } => state.pitch_bend = Some(value),
            ChannelMessage::NoteOn { .. } | ChannelMessage::NoteOff { .. } => {}
        }
    }

    /// Fold every event of `events`, in order.
    pub fn fold(&mut self, events: &[ScoreEvent]) {
        for event in events {
            self.apply(event.channel, event.message);
        }
    }

    /// The messages that rebuild `channel`'s state on a freshly reset synthesizer, in the
    /// order they must be sent.
    pub fn replay(&self, channel: u8, mut emit: impl FnMut(ChannelMessage)) {
        self.channels[usize::from(channel) % CHANNEL_COUNT].replay(&mut emit);
    }

    /// The program the score last selected on `channel`, if it selected one.
    pub fn program(&self, channel: u8) -> Option<u8> {
        self.channels[usize::from(channel) % CHANNEL_COUNT].program
    }

    /// The bank the score last selected on `channel` (controller 0), if it selected one.
    pub fn bank(&self, channel: u8) -> Option<u8> {
        self.channels[usize::from(channel) % CHANNEL_COUNT].bank
    }

    /// An ordinary controller's last value on `channel`, if the score set it.
    pub fn controller(&self, channel: u8, controller: u8) -> Option<u8> {
        self.channels[usize::from(channel) % CHANNEL_COUNT].controllers
            [usize::from(controller & 0x7F)]
    }
}
