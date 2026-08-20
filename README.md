# labyrinth_game

A 3D game in Rust with [Bevy](https://bevy.org) (ECS).

**Status: graybox seed.** A window, a camera, a grey ground plane, a grey cube,
and a directional light. There is no game yet — this is the rendering smoke
test everything else will be built on top of.

## Requirements

- Rust stable (developed against 1.95.0)
- macOS builds with no extra setup. On Linux, install Bevy's system
  dependencies — see `docs/linux_dependencies.md` in the `bevy` crate, or the
  package list in `.github/workflows/ci.yml`.

## Build and run

```sh
cargo run          # build and open the window
cargo test         # run the spec tests (headless, no window)
cargo check        # type-check only
```

For faster iteration, `cargo rund` and `cargo checkd` build with Bevy
dynamically linked. **Never use dynamic linking for a release build** — the
resulting binary requires `libbevy_dylib` shipped alongside it. See `CLAUDE.md`.

## Layout

| Path | What it is |
|---|---|
| `src/main.rs` | The entire program: a `SPEC` block, the `setup` system, and its tests |
| `.cargo/config.toml` | Dev-only fast-build aliases; linker rationale |
| `CLAUDE.md` | Working notes: verified versions, commands, conventions |

## Contributing

Behaviour changes go spec → failing test → code. Add the SPEC line to the
`src/main.rs` doc comment, add the test, watch it fail for the right reason,
then implement.

`main` is protected: direct pushes are rejected. Work happens on a branch and
merges via pull request, and the `verify gate` CI check must pass first.

## Licence

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE),
at your option — matching the Rust ecosystem and Bevy itself.
