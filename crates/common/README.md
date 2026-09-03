# Common Crate

The `common` crate hosts the **foundational, cross-cutting types** that most
engine crates depend on. It exists to keep core math/data primitives consistent
and avoid duplication.

## What Belongs Here
- Fundamental math/data types used across **multiple** crates (e.g. `Color`,
  `Transform2D`, `Camera`, `Rect`).
- Small, stable utilities that are **crate-agnostic** and unlikely to change
  with engine features.
- Lightweight macros that reduce boilerplate without introducing domain logic.

## Graduation Rule (Keep `common` Lean)
If a module or helper is used by **fewer than 3 crates**, it should graduate out
of `common` into a more specific home (e.g. `render_shared`, `ecs_shared`, or the
owning crate). This keeps `common` from becoming a catch‑all and ensures it only
contains truly cross-cutting primitives.

Exceptions should be rare and documented (e.g. a module that is expected to be
used by more crates soon).

## Macros
`common::macros` provides small helper macros used to reduce boilerplate. The
current macro is:

- `with_fields!` — Generates builder-style setters for struct fields. This is
  intentionally generic and can be used in any crate for simple builder APIs.

These macros should remain **minimal and generic**. If a macro embeds
domain-specific behavior or targets only one crate, it should live alongside
that crate instead of in `common`.