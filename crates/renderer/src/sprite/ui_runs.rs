//! The UI pass's draw order: every UI instance drawn back to front, in as few runs as
//! the overlaps allow.

use glam::Vec2;

use crate::sprite::SpriteBatch;
use crate::sprite_data::SpriteInstance;

/// An axis-aligned box in world units: `(min, max)`.
type Bounds = (Vec2, Vec2);

/// The box an instance's quad covers: the quad spans ±0.5 of its scale, turned by its
/// rotation. A non-finite position, scale or rotation yields the whole plane, so such a
/// quad overlaps everything and is never ordered past anything — a NaN box would compare
/// false with every other and blind the overlap guard.
fn instance_bounds(instance: &SpriteInstance) -> Bounds {
    let half = Vec2::new(instance.scale[0].abs(), instance.scale[1].abs()) * 0.5;
    let (sine, cosine) = instance.rotation.sin_cos();
    let reach = Vec2::new(
        half.x * cosine.abs() + half.y * sine.abs(),
        half.x * sine.abs() + half.y * cosine.abs(),
    );
    let centre = Vec2::from(instance.position);
    if !(centre.is_finite() && reach.is_finite()) {
        return (Vec2::NEG_INFINITY, Vec2::INFINITY);
    }
    (centre - reach, centre + reach)
}

fn overlaps(first: Bounds, second: Bounds) -> bool {
    first.0.x < second.1.x && second.0.x < first.1.x && first.0.y < second.1.y && second.0.y < first.1.y
}

fn union(first: Bounds, second: Bounds) -> Bounds {
    (first.0.min(second.0), first.1.max(second.1))
}

/// Orders the UI pass into runs that draw strictly back to front, reusing its buffers
/// from frame to frame so a steady frame allocates nothing.
///
/// Batching by texture alone orders whole batches by their lowest depth, so a texture
/// with instances both behind and in front of another texture draws its front ones too
/// early — and a transparent texel still writes depth, so the later, farther texture then
/// fails the depth test inside that quad and a hole shows the scene beneath. A glyph the
/// HUD and a panel's text both use is exactly that (every glyph is its own texture).
///
/// Each instance, taken in depth order, joins the latest run of its own texture and clip
/// unless a run drawn after that one overlaps it on screen; otherwise it starts a run. So
/// no instance is ever drawn before something it overlaps and is in front of, which is the
/// only order a depth test can punch a hole in — while instances that overlap nothing
/// between them still share one draw, as they did when the UI batched by texture.
#[derive(Default)]
pub struct UiRunBuilder {
    /// `(batch, instance)` indices of every instance, sorted into drawing order.
    order: Vec<(u32, u32)>,
    /// The runs; only the first `run_count` are this frame's. Later ones are kept for
    /// their capacity.
    runs: Vec<SpriteBatch>,
    /// The box covering each run's instances, index-aligned with `runs`.
    run_bounds: Vec<Bounds>,
    /// Each run's instances' own boxes, index-aligned with `runs` and with each run's
    /// instances, so the overlap check never recomputes one.
    member_bounds: Vec<Vec<Bounds>>,
    run_count: usize,
}

impl UiRunBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build this frame's runs from `batches`, and return them in drawing order.
    pub fn build(&mut self, batches: &[&SpriteBatch]) -> &[SpriteBatch] {
        self.order.clear();
        for (batch_index, batch) in batches.iter().enumerate() {
            self.order.extend((0..batch.instances.len()).map(|instance_index| (batch_index as u32, instance_index as u32)));
        }
        let instance = |&(batch_index, instance_index): &(u32, u32)| {
            let batch = batches[batch_index as usize];
            (batch, &batch.instances[instance_index as usize])
        };
        // A total key, so the unstable sort (which allocates nothing) is deterministic.
        self.order.sort_unstable_by(|first, second| {
            let ((first_batch, first_instance), (second_batch, second_instance)) = (instance(first), instance(second));
            first_instance
                .depth
                .total_cmp(&second_instance.depth)
                .then_with(|| first_batch.texture_handle.id.cmp(&second_batch.texture_handle.id))
                .then_with(|| first_batch.clip.cmp(&second_batch.clip))
                .then_with(|| first.cmp(second))
        });

        self.run_count = 0;
        for position in 0..self.order.len() {
            let (batch, sprite) = instance(&self.order[position]);
            let bounds = instance_bounds(sprite);
            let joinable = self.joinable_run(batch, bounds);
            match joinable {
                Some(run) => {
                    self.runs[run].instances.push(*sprite);
                    self.member_bounds[run].push(bounds);
                    self.run_bounds[run] = union(self.run_bounds[run], bounds);
                }
                None => self.open_run(batch, *sprite, bounds),
            }
        }
        &self.runs[..self.run_count]
    }

    /// The latest run `batch`'s instances may join with one covering `bounds`: the most
    /// recent run of the same texture and clip, if no run drawn after it overlaps.
    fn joinable_run(&self, batch: &SpriteBatch, bounds: Bounds) -> Option<usize> {
        for run in (0..self.run_count).rev() {
            let candidate = &self.runs[run];
            if candidate.texture_handle == batch.texture_handle && candidate.clip == batch.clip {
                return Some(run);
            }
            if overlaps(self.run_bounds[run], bounds)
                && self.member_bounds[run].iter().any(|&drawn| overlaps(drawn, bounds))
            {
                return None;
            }
        }
        None
    }

    fn open_run(&mut self, batch: &SpriteBatch, sprite: SpriteInstance, bounds: Bounds) {
        if self.run_count == self.runs.len() {
            self.runs.push(SpriteBatch::new(batch.texture_handle));
            self.run_bounds.push(bounds);
            self.member_bounds.push(Vec::new());
        }
        let run = &mut self.runs[self.run_count];
        run.instances.clear();
        run.texture_handle = batch.texture_handle;
        run.clip = batch.clip;
        run.sorted = true;
        run.instances.push(sprite);
        self.run_bounds[self.run_count] = bounds;
        let members = &mut self.member_bounds[self.run_count];
        members.clear();
        members.push(bounds);
        self.run_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::fixtures::{batch_with, instance_at_depth};
    use crate::texture::TextureHandle;

    fn depths(batch: &SpriteBatch) -> Vec<f32> {
        batch.instances.iter().map(|instance| instance.depth).collect()
    }

    // === UiRunBuilder ===

    fn runs_summary(runs: &[SpriteBatch]) -> Vec<(u32, Vec<f32>)> {
        runs.iter().map(|run| (run.texture_handle.id, depths(run))).collect()
    }

    /// An instance at `depth`, a `size`-wide square centred on `(x, y)`.
    fn quad(x: f32, y: f32, size: f32, depth: f32) -> SpriteInstance {
        let mut instance = instance_at_depth(depth);
        instance.position = [x, y];
        instance.scale = [size, size];
        instance.rotation = 0.0;
        instance
    }

    /// The panel case: a glyph texture the HUD uses behind a panel and the panel's own text
    /// uses in front of it must draw in three runs, the panel's fill between its two uses.
    #[test]
    fn test_ui_runs_draw_a_glyph_over_the_panel_it_sits_on_after_the_panel() {
        let glyph = TextureHandle { id: 7 };
        let hud_zero = quad(0.0, 0.0, 10.0, 0.1);
        let panel_zero = quad(0.0, 0.0, 10.0, 0.9);
        let glyphs = batch_with(&[hud_zero, panel_zero], glyph);
        let fill = batch_with(&[quad(0.0, 0.0, 100.0, 0.5)], TextureHandle::WHITE);

        let mut builder = UiRunBuilder::new();
        let runs = builder.build(&[&glyphs, &fill]);

        assert_eq!(
            runs_summary(runs),
            vec![(7, vec![0.1]), (TextureHandle::WHITE.id, vec![0.5]), (7, vec![0.9])],
            "back to front, the panel's fill between the two glyphs"
        );
        assert!(runs.iter().all(|run| run.sorted));
    }

    /// Labels side by side: the second label's rect and glyphs overlap nothing drawn in
    /// between, so they join the first label's runs and the draw count stays at one per
    /// texture, as when the UI batched by texture alone.
    #[test]
    fn test_ui_runs_merge_what_overlaps_nothing_in_between() {
        let glyph = TextureHandle { id: 9 };
        let rects = batch_with(&[quad(0.0, 0.0, 20.0, 0.1), quad(100.0, 0.0, 20.0, 0.3)], TextureHandle::WHITE);
        let glyphs = batch_with(&[quad(0.0, 0.0, 5.0, 0.2), quad(100.0, 0.0, 5.0, 0.4)], glyph);

        let mut builder = UiRunBuilder::new();
        let runs = builder.build(&[&rects, &glyphs]);

        assert_eq!(
            runs_summary(runs),
            vec![(TextureHandle::WHITE.id, vec![0.1, 0.3]), (9, vec![0.2, 0.4])],
            "two draws for two labels"
        );
    }

    #[test]
    fn test_ui_runs_keep_a_clip_with_its_instances() {
        let texture = TextureHandle { id: 4 };
        let unclipped = batch_with(&[quad(0.0, 0.0, 5.0, 0.2)], texture);
        let mut clipped = batch_with(&[quad(0.0, 0.0, 5.0, 0.3)], texture);
        clipped.clip = Some([1, 2, 3, 4]);

        let mut builder = UiRunBuilder::new();
        let runs = builder.build(&[&clipped, &unclipped]);

        assert_eq!(runs.iter().map(|run| run.clip).collect::<Vec<_>>(), vec![None, Some([1, 2, 3, 4])]);
    }

    #[test]
    fn test_ui_runs_box_a_turned_quad_by_its_corners_and_a_turned_line_tightly() {
        let mut turned = quad(0.0, 0.0, 10.0, 0.0);
        turned.rotation = std::f32::consts::FRAC_PI_4;
        let (min, max) = instance_bounds(&turned);
        let corner_reach = (5.0f32 * 5.0 * 2.0).sqrt();
        assert!((max.x - corner_reach).abs() < 1e-4 && (min.y + corner_reach).abs() < 1e-4);

        // A 300 x 2 separator stood on end is 2 wide, not a 300-wide circle.
        let mut separator = quad(0.0, 0.0, 1.0, 0.0);
        separator.scale = [300.0, 2.0];
        separator.rotation = std::f32::consts::FRAC_PI_2;
        let (min, max) = instance_bounds(&separator);
        assert!((max.x - 1.0).abs() < 1e-3 && (min.x + 1.0).abs() < 1e-3, "{min} {max}");
        assert!((max.y - 150.0).abs() < 1e-3);
    }

    /// A NaN position must not let a later quad slip past it: it is treated as covering
    /// everything, so the glyph drawn in front of it still waits for it.
    #[test]
    fn test_ui_runs_treat_a_non_finite_quad_as_overlapping_everything() {
        let glyph = TextureHandle { id: 5 };
        let mut broken = quad(0.0, 0.0, 10.0, 0.5);
        broken.position = [f32::NAN, 0.0];
        let fill = batch_with(&[broken], TextureHandle::WHITE);
        let glyphs = batch_with(&[quad(500.0, 500.0, 4.0, 0.1), quad(900.0, 900.0, 4.0, 0.9)], glyph);

        let mut builder = UiRunBuilder::new();
        let runs = builder.build(&[&glyphs, &fill]);

        assert_eq!(runs.len(), 3, "the front glyph does not join the run drawn before the broken quad");
    }

    #[test]
    fn test_ui_runs_reuse_their_buffers_and_forget_last_frame() {
        let texture = TextureHandle { id: 2 };
        let busy = batch_with(&[quad(0.0, 0.0, 5.0, 0.1), quad(0.0, 0.0, 5.0, 0.2)], texture);
        let quiet = batch_with(&[quad(0.0, 0.0, 5.0, 0.1)], texture);

        let mut builder = UiRunBuilder::new();
        assert_eq!(builder.build(&[&busy]).len(), 1);
        assert_eq!(runs_summary(builder.build(&[&quiet])), vec![(2, vec![0.1])], "no instance left over");
        assert!(builder.build(&[]).is_empty());
    }
}
