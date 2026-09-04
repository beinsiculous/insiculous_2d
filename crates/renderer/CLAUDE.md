# Renderer Crate — Agent Context

You are working in the rendering crate. WGPU 28.0.0 backend with instanced sprite rendering, HDR + bloom post-processing.

## Architecture
```
Renderer (WGPU device, queue, surface, RendererConfig{vsync})
├── RenderTargets (HDR color + depth + bloom ping/pong, rebuilt on resize)
├── SpritePipeline (instanced quads -> HDR target)
│   ├── Vertex/index buffers (quad geometry)
│   ├── Instance buffer (DynamicBuffer — grows on demand, never panics)
│   ├── InstanceCache — skips the instance upload when nothing changed (GPP-15)
│   ├── CameraBinding (buffer + bind-group layout + bind group)
│   └── Texture bind groups (cached per handle; TextureHandle::WHITE = built-in 1x1 white)
├── LinePipeline (line-list geometry -> HDR target, e.g. spring-mass grid)
└── BloomPipeline (extract -> H/V blur ping-pong -> composite to swapchain)
    └── Bind groups cached per target size; per-direction blur uniform buffers
```

## Rendering Flow (one frame)
1. Sprites + lines draw into the HDR target (Rgba16Float) with depth
2. Bloom extracts bright pixels (half-res), blurs H+V × iterations, composites to the sRGB swapchain
3. Camera uniforms uploaded once per pipeline per frame via `CameraBinding`

## File Map
- `renderer.rs` — WGPU lifecycle and frame orchestration; fail-stop on device loss, resize dedup with forced reconfigure on zero-size recovery, and `set_viewport_scissor`.
- `camera_binding.rs` — `CameraBinding`: unified camera uniform buffer, bind group layout, and bind group shared across SpritePipeline and LinePipeline.
- `scissor.rs` — scissor math: `quantize_rect` (outward rounding), `clamp_scissor` (None = empty/skip draw), and `batch_scissor` (clip ∩ pass default).
- `device_status.rs` — `DeviceLossLatch` (one-way fail-stop latch polled before queue/surface work) and pure `resize_action` guard.
- `sprite/batch.rs` — `SpriteBatch` and `SpriteBatcher`: CPU-side grouping keyed by (texture, clip); `set_clip` cursor drives per-batch GPU scissoring.
- `sprite_data.rs` — GPU data structures (`SpriteVertex`, `SpriteInstance` with SDF shape parameters, `DynamicBuffer` with power-of-two growth).
- `texture.rs` — `TextureManager`, `TextureHandle` (with reserved `WHITE`), and `SamplerConfig`.
- `texture_filter.rs` — `TextureFilter`: Linear/Nearest mapping to `SamplerConfig`.
- `bloom.rs` — Bloom passes; composite encodes gamma via `BloomParams.inv_gamma` on non-sRGB swapchains (WebGPU canvases).
- `window.rs` — `insert_canvas_into_dom` (wasm): swaps canvas in place of `#game-canvas` or appends to body (winit does not insert canvas into DOM).

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| Float sorts in `SpriteBatch` must use `total_cmp`, never `partial_cmp().unwrap()` | `src/sprite/batch.rs test_sort_by_depth_orders_ascending_with_nan_last` |
| `DeviceLossLatch` is one-way: marking loss is idempotent and never resets | `src/device_status.rs test_device_loss_latch_is_one_way_and_shared_by_clones` |
| Instance cache invalidation: identical bytes with different batch boundaries must still re-upload | `src/sprite/instance_cache.rs test_same_bytes_with_different_batch_boundaries_still_upload` |
| Scissor clamping: overhang on resize race trims to live surface to satisfy `scissor ⊆ attachment` | `src/scissor.rs test_clamp_trims_to_the_live_surface_and_empties_to_none` |
| Empty scissor intersection result must skip the draw call | `src/scissor.rs test_batch_scissor_intersects_clip_with_default_and_skips_empty` |
| `TextureHandle::WHITE` is reserved (default is `WHITE` and manager allocates from 1 so no loaded texture collides) | `src/texture.rs test_default_handle_is_the_reserved_white_texture` |
| `SpriteVertex` and `SpriteInstance` attributes must match shader locations | `src/sprite_data.rs test_sprite_attributes_match_shader_locations` |
| `DynamicBuffer` grows to next power of two and never shrinks | `src/sprite_data.rs test_dynamic_buffer_grown_capacity` |
| `queue.write_buffer` flushes at `submit()`, not encode time: rewriting one uniform between passes in a single submit makes every pass read the last write | — none |
| Cross-batch draw order follows `HashMap` iteration in `SpriteBatcher` and is not deterministic today; only the sort within a batch is (open defect on the renderer backlog issue) | — none |
| Winit on wasm never inserts canvas into DOM; `insert_canvas_into_dom` must swap into `#game-canvas` or append to body | — none |

## Key Guidelines
- **Cache bind groups — never create per-frame.** Sprite textures cache per handle; bloom caches per target size.
- **`queue.write_buffer` flushes at submit, not encode.** Never rewrite one uniform buffer between passes in the same submit — every pass sees only the last write. Use one buffer per distinct value (see bloom's H/V blur buffers).
- Batch by texture to minimize bind group switches; cross-batch submission order must be deterministic (callers sort by min depth, then handle)
- `DynamicBuffer` grows (next power of two) and never shrinks; pass `&Device` to `update`
- Float sorts use `total_cmp` — no `partial_cmp().unwrap()`
- All tests run headless (GPU-dependent doc examples are compile-only `no_run`)

## Known Tech Debt
Tracked on the Studio Board: cross-batch transparency vs depth writes ARCH-006 — still OPEN, will be
closed by E7 alpha-cutoff #10 once it lands. DRY-006 (shared camera binding) closed via `CameraBinding`.
Deferred **by design** (not debt): no mipmap generation (the old flag
allocated a mip chain and never filled it — re-add only with real mip
generation); `RendererConfig` stays vsync-only until a game needs more
(power preference / MSAA / bloom downsample).

## Testing
- `cargo test -p renderer` — 0 failed, 0 ignored

## Godot Oracle — When Stuck
Use `WebFetch` to read from `https://github.com/godotengine/godot/blob/master/`

| Our Concept | Godot Equivalent | File |
|-------------|-----------------|------|
| SpritePipeline batching | Canvas item rendering | `servers/rendering/renderer_canvas_cull.cpp` — `canvas_render_items` |
| Sprite component | Sprite2D | `scene/2d/sprite_2d.cpp` |
| Camera2D | Camera2D | `scene/2d/camera_2d.cpp` |
| sprite_instanced.wgsl | Canvas shader | `servers/rendering/renderer_rd/shaders/canvas.glsl` |
| Texture caching | Texture storage | `servers/rendering/storage/texture_storage.cpp` |
| Bloom | Glow effect | `servers/rendering/renderer_rd/effects/copy_effects.cpp` |

**Remember:** We use WGPU, not Vulkan/OpenGL. Study Godot's *batching design* and *draw order logic*, not its graphics API calls.
