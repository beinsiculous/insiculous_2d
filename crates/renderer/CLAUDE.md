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
│   ├── Camera uniform buffer + bind group (cached)
│   └── Texture bind groups (cached per handle; TextureHandle::WHITE = built-in 1x1 white)
├── LinePipeline (line-list geometry -> HDR target, e.g. spring-mass grid)
└── BloomPipeline (extract -> H/V blur ping-pong -> composite to swapchain)
    └── Bind groups cached per target size; per-direction blur uniform buffers
```

## Rendering Flow (one frame)
1. Sprites + lines draw into the HDR target (Rgba16Float) with depth
2. Bloom extracts bright pixels (half-res), blurs H+V × iterations, composites to the sRGB swapchain
3. Camera uniforms uploaded once per pipeline per frame

## File Map
- `renderer.rs` — WGPU device/queue/surface lifecycle, `RendererConfig`, frame orchestration.
  Registers wgpu's device-lost + uncaptured-error callbacks at creation; every render-path
  entry (`acquire_frame`, `render_with_sprites`, `set_lines`, `resize`, `recreate_surface`)
  guards on the loss latch. `resize` dedups same-size reconfigures and arms a forced
  reconfigure after a skipped zero-size request (hidden web canvas round trip).
  `set_viewport_scissor(Option<[u32;4]>)` (per-frame, like `set_lines`) bounds the
  game-world passes — sprites, lines, bloom composite — to a rect; the UI pass is exempt
- `scissor.rs` — pure scissor math (issue #41): `quantize_rect` (outward rounding,
  NaN-safe), `clamp_scissor` (`None` = empty ⇒ skip draw), `intersect_scissor`,
  `batch_scissor` (per-batch decision: clip ∩ pass default, clamped). All headless-tested
- `white_texture.rs` — the built-in 1x1 white texture resource (extracted from renderer.rs)
- `device_status.rs` — `DeviceLossLatch` (one-way Arc<AtomicBool> set by the lost callback,
  polled before all queue/surface work) + pure `resize_action` guard. Fail-stop by design:
  no auto-recovery (the device/queue Arcs fan out into every pipeline)
- `sprite.rs` — `Sprite` data type; parent of the sprite submodules
- `sprite/batch.rs` — `SpriteBatch` (carries `clip: Option<[u32;4]>`), `SpriteBatcher`
  (CPU-side grouping keyed by `(texture, clip)`; `set_clip` cursor drives per-batch GPU
  scissoring for clipped UI — game paths never set a clip and batch exactly as before;
  `batch_for(texture)` = the unclipped batch)
- `sprite/pipeline.rs` — `SpritePipeline` (GPU pipeline, bind group caches, draw)
- `sprite_data.rs` — GPU data structures (`SpriteVertex`, `SpriteInstance` incl. `shape: [f32;4]` SDF params [kind, corner_radius, border_width, _] — kind 0=quad/1=rounded rect/2=circle, 76-byte stride, attr @10; fragment masks with sdRoundedBox + 1.5px AA), `DynamicBuffer`
- `texture.rs` — `TextureManager`, `TextureHandle` (incl. `WHITE`), `SamplerConfig`
- `texture_filter.rs` — `TextureFilter` (Linear/Nearest → `SamplerConfig` via `From`; the pixel-art knob engine_core plumbs from `GameConfig` and `.sheet.ron` sidecars); public path is still `renderer::TextureFilter`
- `render_targets.rs` — HDR/depth/bloom textures, resize handling
- `bloom.rs` — bloom passes + `BloomConfig` (runtime-tunable); composite takes
  a `SwapchainTarget { view, is_srgb }` — non-sRGB swapchains (WebGPU canvases
  expose NO sRGB formats) get gamma-encoded in the shader via
  `BloomParams.inv_gamma` so web brightness matches native
- `window.rs` — window creation + **`insert_canvas_into_dom` (wasm)**: winit
  NEVER inserts its canvas into the DOM (detached canvas = every pass valid,
  page silently black); this swaps it in place of the page's `#game-canvas`
  placeholder (id/size/a11y attrs copied, canvas focused) or appends to body.
  Called from engine_core's `WindowManager::create` — any new window-creation
  path must call it too. Adopting an existing canvas via `with_canvas` was
  tried and abandoned (Aug 2026)
- `line_pipeline.rs` — `LinePipeline`, `LineVertex`
- `shaders/` — `sprite_instanced.wgsl`, `line.wgsl`, `bloom_{extract,blur,composite}.wgsl`

## Key Guidelines
- **Cache bind groups — never create per-frame.** Sprite textures cache per handle; bloom caches per target size.
- **`queue.write_buffer` flushes at submit, not encode.** Never rewrite one uniform buffer between passes in the same submit — every pass sees only the last write. Use one buffer per distinct value (see bloom's H/V blur buffers).
- Batch by texture to minimize bind group switches; cross-batch submission order must be deterministic (callers sort by min depth, then handle)
- `DynamicBuffer` grows (next power of two) and never shrinks; pass `&Device` to `update`
- Float sorts use `total_cmp` — no `partial_cmp().unwrap()`
- All tests run headless (GPU-dependent doc examples are compile-only `no_run`)

## Known Tech Debt
Tracked on the Studio Board: issue #89 (shared camera binding DRY-006;
cross-batch transparency vs depth writes ARCH-006 — still OPEN, will be
closed by E7 alpha-cutoff #10 once it lands). Deferred **by design** (not debt): no mipmap generation (the old flag
allocated a mip chain and never filled it — re-add only with real mip
generation); `RendererConfig` stays vsync-only until a game needs more
(power preference / MSAA / bloom downsample).

## Testing
- 73 tests (71 unit + 2 doc), run with `cargo test -p renderer`

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
