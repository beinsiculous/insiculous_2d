# Resizable pixel-art sprites

Add `SpriteTiling` beside an entity's `Sprite` and `Transform2D` to resize an
animated prop without stretching its pixels. The component describes one source
frame, including its transparent padding. The existing scale gizmo and
**Transform2D → Scale** set the destination size; **SpriteTiling** sets the source
dimensions and fixed borders.

```rust
use engine_core::prelude::*;

let tiling = SpriteTiling {
    source_size: Vec2::new(96.0, 192.0),
    borders: Vec4::new(48.0, 80.0, 16.0, 80.0),
};
```

Borders are ordered **left, top, right, bottom**, measured in source pixels.
This example leaves a 32×32 repeatable interior. The corners stay fixed, each
edge repeats along its long axis, and the centre repeats along both axes.
Changing the destination from 96×192 to 128×256 adds repetitions while the
grate bars, outline and other details keep their original pixel size. A final
partial repetition is cropped. Keep unique details inside fixed corners and
paint opposite sides of repeatable sections to join cleanly.

The destination size in world pixels is
`Transform2D.scale × Sprite.scale × RENDER_UNIT`. With the default sprite scale,
96×192 uses transform scale `(1.2, 2.4)` and 128×256 uses `(1.6, 3.2)`.
Whole-pixel destination dimensions and nearest filtering preserve pixel-art
edges; use whole source pixels and border widths too. These dimensions remain
the editor's selection and gizmo bounds. When adding the component manually,
set Source Pixels to the actual frame size before resizing: its default is 16×16,
which repeats a complete texture in 16×16 cells until configured.

Animation continues to select `Sprite.tex_region`; all patches sample inside
that active frame. Visibility, colour, depth, emissive strength, sprite offset
and entity/hierarchy rotation behave as they do for an ordinary sprite. Tile
generation does not change collision geometry. Signed transform, sprite and UV
flips mirror the whole assembly. Pixel snapping moves every patch by one shared
anchor correction, keeping rotated and partial sections joined.

If a destination is smaller than its fixed borders, the inner sides of those
borders are cropped to fit. Nothing stretches or draws beyond the destination
rectangle. Source dimensions and borders must be finite and leave at least one
interior pixel on each axis; the inspector and command API clamp edited values,
while scene loading rejects invalid settings. Rendering emits nothing for an
invalid destination or a layout requiring more than 65,536 patches; it logs the
first such failure, including texture, position and dimensions. Zero-size sprites
draw nothing without consuming that warning. Tile expansion also stops before
adding a complete assembly would push the frame's batch beyond 65,536 instances,
so many small repeat patterns cannot multiply into millions of patches.
This covers a 4K backdrop with 16-pixel tiles, while bounding
work for very fine patterns over huge rectangles. Non-finite editable source
sizes reset to 16 pixels; finite sizes clamp to 1–16,777,216 so one-pixel margins
remain representable. Scene loading rejects dimensions outside that range.

Scene saves normalize tile settings so runtime edits cannot write an unloadable
component. Keeping paused edits after Play also normalizes the merged settings.
Scene saves retain the component and destination transform. Undo, redo,
duplication and Play/Stop snapshots use the normal component registry. Prefab
overrides may change source dimensions or borders independently.

Expansion is part of the default `Game::render`. A custom renderer that replaces
that method can call `append_tiled_sprite(&renderer_sprite, &tiling, ctx.sprites)`
with the fully transformed renderer sprite. This helper handles signed scale,
flipped UVs and the shared snap anchor together. The lower-level
`SpriteTiling::tiles` expects positive destination sizes and positive UV extents.
