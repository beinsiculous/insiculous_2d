# Flyweight

Book: https://gameprogrammingpatterns.com/flyweight.html

Intent: when many objects share the same heavy data, store that data once and give each object only a cheap reference plus the few fields that are truly its own.

## The problem it solves

A forest of ten thousand trees doesn't need ten thousand copies of the tree mesh and its
textures. A tile map doesn't need every grass tile to carry its own copy of "grass": movement
cost, sprite and sound. Copying that shared, unchanging part into every instance wastes memory,
wastes the time it takes to build it, and, on a GPU, wastes the bandwidth of pushing it again
and again. The pattern splits each object into *intrinsic* state, which is identical across
instances and shared, and *extrinsic* state, which is per instance (position, tint, the current
frame). Instances hold a reference to the shared part and carry only the extrinsic fields.

## Reach for it when

- Many instances share large immutable data: textures, meshes, glyph bitmaps, decoded audio.
- You're about to load or upload the same asset twice.
- A grid or a table holds the same few "kinds" thousands of times. Store a reference to the
  kind, not a copy of it.
- The GPU can draw many instances of one mesh in one call. Instancing is flyweight in hardware.

## Don't reach for it when

- There are only a handful of instances. A shared table and a handle add indirection for no
  measurable saving.
- The "shared" data is actually edited per instance. A flyweight has to be immutable, or one
  instance's change silently becomes everyone's.
- Lifetime is unclear. If nobody knows when the shared data may be freed, you've traded a memory
  cost for a correctness bug.

## Tradeoffs

- **Immutability is the price of sharing.** The moment one user needs to tweak the shared part,
  it must stop being shared, either by copy-on-write or by moving the field to the instance.
- **Handles go stale.** An instance that holds an id for a freed resource reads garbage, or
  worse, a resource that has since reused the id. Something has to own the lifetime.
- **Dedupe needs a key.** "Same asset" has to be defined: path, path plus sampling filter, or
  content. Get the key wrong and you either share things that differ or duplicate things that
  don't.
- **The saving is purely in efficiency.** The book is explicit that the pattern buys nothing in
  structure. Use it where memory, load time or draw calls actually hurt.

## In Rust

Rust offers three ways to share intrinsic data, from loosest to tightest:

- **Handles into a manager** (`struct TextureHandle(u32)`): `Copy`, trivially stored in
  components, and serializable as an id or a path. The manager owns the lifetime. This is the
  usual game-engine form, because components stay plain data.
- **`Arc<T>` / `Arc<[u8]>`**: the shared data frees itself when the last user drops it. It's good
  for bytes handed across threads (an audio decoder). It's awkward inside components that must
  serialize.
- **`&'static` tables** (`const KINDS: [Kind; N]`) for kinds known at compile time. These are
  zero-cost, and the borrow checker proves nobody mutates them.

The dedupe step is a cache in front of the loader. An illustrative sketch, not engine code:

```rust
fn load(&mut self, path: &str) -> TextureHandle {
    if let Some(&handle) = self.by_path.get(path) {
        return handle;
    }
    let handle = self.upload(path);
    self.by_path.insert(path.to_string(), handle);
    handle
}
```

## In insiculous_2d

**Verdict: textbook.** The July 2026 audit counted it as textbook already. Components hold handles,
loads are deduped, sheets are loaded once and stamped onto many entities, and the GPU draws one
quad many times.

- `crates/renderer/src/texture.rs` · `TextureHandle` — a `Copy` `u32` id. A `Sprite` stores the
  handle, never the texture. `TextureHandle::WHITE` (id 0) is the reserved 1×1 white texture
  every flat-colour sprite shares.
- `crates/engine_core/src/assets.rs` · `AssetManager::load_texture_filtered` — the dedupe:
  same file plus same filter means the same GPU texture.

```rust excerpt=crates/engine_core/src/assets.rs
        // Same file + same filter = same GPU texture. Without this, every
        // scene reload re-decoded and re-uploaded its whole texture set.
        let cache_key = (original_path_string.clone(), filter);
        if let Some(&handle) = self.loaded_by_path.get(&cache_key) {
            return Ok(handle);
        }
```

- `crates/engine_core/src/assets/sprite_sheet.rs` · `SpriteSheet` — one loaded sheet (texture,
  grid, clips). `SpriteSheet::sprite` and `SpriteSheet::animation` stamp it onto as many entities
  as the game spawns.
- `crates/engine_core/src/assets/sheet_spec.rs` · `SheetSpec` — the `const` contract a game
  names once per sheet (path, cell, opaque bounds). That's the `&'static` table form.
- `crates/renderer/src/sprite_data.rs` · `SpriteInstance` — the extrinsic part only (position,
  rotation, scale, UV region, tint), `Pod`, uploaded per instance. The quad geometry is one shared
  vertex/index buffer.
- `crates/renderer/src/sprite/batch.rs` · `SpriteBatcher` — sprites are grouped by texture (and
  clip), so one texture is one bind and one draw.

```rust excerpt=crates/renderer/src/sprite/pipeline.rs
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, instance_offset..(instance_offset + instance_count));
```

- `crates/engine_core/src/contexts.rs` · `GlyphCacheKey` — glyph textures are keyed *without*
  colour, because they are alpha masks tinted at draw time. The key does include the font id, or
  a locale font switch would serve stale shapes.
- `crates/audio/src/manager/mod.rs` · `AudioManager` — sound bytes are held as `Arc<[u8]>` and
  each play decodes from a cursor over the shared buffer rather than cloning it.

**Where it deliberately stops.** `SpriteSheet::animation` copies the clip table and the sheet
path into each entity's `SpriteAnimation`. That keeps the component self-contained for scene
save, snapshots and the inspector. It's the right call at today's entity counts, and it's the
first thing to share (`Arc`, or a handle to the sheet) if spawning thousands of animated
entities ever shows up in a profile.

## Pitfalls here

- **The dedupe key includes the filter.** The same PNG loaded `Nearest` and `Linear` is two
  textures on purpose. Don't "fix" it into one.
- **Generated textures carry a path too.** `#white` and `#solid:RRGGBB` are canonical references
  that survive save/load. `#rgba` (from `create_texture_from_rgba`) is a placeholder that cannot
  rebuild, so a reload degrades it to white.
- **A scene's `#solid:` colour is shared, a direct `create_solid_color` is not.** Resolving
  `#solid:RRGGBB` goes through `AssetManager::shared_solid_color`, one texture per colour in the
  same cache file loads use. Calling `create_solid_color` yourself makes a fresh texture each
  time, so cache the handle if you call it more than once.
- `crates/engine_core/src/assets.rs` · `AssetManager::shared_solid_color` — the dedupe for
  generated colours.
- **Batches are per texture.** Ten sprites on ten different one-off textures are ten draws, so
  sharing the texture is what makes batching pay.
