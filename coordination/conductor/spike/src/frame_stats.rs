//! Frame-time windows: the game's own frame times, summarised every few seconds so a
//! run with the conductor can be compared with a run without it.

/// One window of frame times.
#[derive(Debug, Default, Clone)]
pub struct FrameWindow {
    frame_times_milliseconds: Vec<f32>,
    pub elapsed_seconds: f32,
}

/// The summary of a finished window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameSummary {
    pub frames: usize,
    pub mean_milliseconds: f32,
    pub percentile_99_milliseconds: f32,
    pub longest_milliseconds: f32,
}

impl FrameWindow {
    pub fn record(&mut self, delta_seconds: f32) {
        self.frame_times_milliseconds.push(delta_seconds * 1000.0);
        self.elapsed_seconds += delta_seconds;
    }

    /// Summarise the window and start a new one.
    pub fn finish(&mut self) -> FrameSummary {
        let mut times = std::mem::take(&mut self.frame_times_milliseconds);
        self.elapsed_seconds = 0.0;
        if times.is_empty() {
            return FrameSummary { frames: 0, mean_milliseconds: 0.0, percentile_99_milliseconds: 0.0, longest_milliseconds: 0.0 };
        }
        times.sort_by(f32::total_cmp);
        let percentile_index = ((times.len() as f32 * 0.99) as usize).min(times.len() - 1);
        FrameSummary {
            frames: times.len(),
            mean_milliseconds: times.iter().sum::<f32>() / times.len() as f32,
            percentile_99_milliseconds: times[percentile_index],
            longest_milliseconds: times[times.len() - 1],
        }
    }
}
