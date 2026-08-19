# labyrinth

A 3D game built in Rust with Bevy (ECS). Currently a **graybox seed**: a window,
a camera, a ground plane, a cube, and a light. There is no game yet.

## Stack

| Component | Version | How to confirm |
|---|---|---|
| rustc | 1.95.0 | `rustc --version` |
| cargo | 1.95.0 | `cargo --version` |
| bevy | 0.19.1 | `grep bevy Cargo.toml` |
| edition | 2024 | `Cargo.toml` |
| target | aarch64-apple-darwin | `rustup show` |

`cargo add bevy` reported "Locking 550 packages". Bevy is the only direct
dependency.

## Toolchain verify command

Run this before the first build of a session. A shell prompt showing an
environment name proves nothing.

```sh
rustc --version && cargo --version && cargo check
```

Last run: passed, 0 warnings, 0 errors.

## Commands

```sh
cargo check                # type-check (fast)
cargo test                 # run the spec tests, headless, no window
cargo run                  # build + open the window
cargo build --release      # release build — see the dynamic-linking caveat below
```

Fast-iteration aliases, defined in `.cargo/config.toml`:

```sh
cargo checkd               # check with Bevy dynamically linked
cargo rund                 # run with Bevy dynamically linked
```

VERIFIED: `cargo checkd` exits 0 in 26.47s with 0 warnings and pulls
`bevy_dylib v0.19.1` into the build graph, so the alias does activate the
feature. `cargo rund` is UNVERIFIED — it shares the feature flag but has not
been executed. (`cargo check` emits only `.rmeta`; the linked `libbevy_dylib`
appears on a real build.)

### Dynamic linking caveat — read before shipping

`bevy/dynamic_linking` is **deliberately not** in `Cargo.toml`'s dependency
features. It exists only in the `rund` / `checkd` aliases.

A binary built with dynamic linking requires `libbevy_dylib` to be distributed
alongside the executable. **Never enable it for a release or shipped build.**
Keeping it out of `Cargo.toml` means `cargo build --release` cannot pick it up
by accident — do not "simplify" this by moving the feature into `Cargo.toml`.

### Build performance

`[profile.dev] opt-level = 1` for our crate, `[profile.dev.package."*"]
opt-level = 3` for dependencies: Bevy compiles optimised once and is cached,
while our code stays fast to recompile.

Measured on this machine:

- clean debug build: 6m09s
- incremental rebuild after editing `src/main.rs`: 1.68s (`cargo test`)
- `cargo check` when nothing changed: 0.43s
- `du -sh target` -> 4.8G (this is why `/target` is gitignored)

No custom linker is configured. Bevy's setup guide states the macOS default
system linker is faster than LLD, so a linker override would be a pessimisation
here. `ld -v` reports ld-1267, and arm64 is absent from its "will use
ld-classic for" list. lld/mold/wild/zld are not installed.

## Bevy API rule — verify, never recall

**Bevy is pre-1.0 and breaks between releases. Do not answer Bevy API questions
from memory. Check the installed version.**

The installed source is the ground truth and is on this machine:

```sh
BV=~/.cargo/registry/src/index.crates.io-*/bevy-0.19.1
ls $BV/examples/3d/                 # 412 runnable examples
ls $BV/_release-content/migration-guides/   # per-change migration notes
```

Three things in 0.19.1 differ from earlier-version habits, found this way:

- `DirectionalLight` / `PointLight` use **`shadow_maps_enabled`**, not
  `shadows_enabled`.
- Lights live in a separate **`bevy_light`** crate (re-exported via prelude).
- The shipped `examples/3d/3d_scene.rs` is now written in BSN scene notation
  (`bsn_list!`). This codebase uses the classic `Commands` style instead, which
  354 of the 412 shipped examples use versus 5 for BSN.

If `cargo check` fails on API drift, fix against `$BV` or the migration guides —
not against what seems familiar.

## Code

`src/main.rs` is the whole program. Its `# SPEC` doc comment lists four
numbered claims (S1–S4); each has a corresponding test in the `tests` module.

Tests run headless — `MinimalPlugins` + `AssetPlugin` + `init_asset::<Mesh>()`
and `init_asset::<StandardMaterial>()`. No window, no GPU. `StandardMaterial`
is normally registered by the heavyweight `MaterialPlugin`, which is why the
test app registers it directly.

Current status: 4 tests, all passing.

## Repository and CI

GitHub: `adityapatil371/labyrinth_game` — **public**. Remote protocol: ssh.

The repo is public specifically so branch protection is available: GitHub
gates that feature behind Pro on private repos (verified — the protection and
rulesets endpoints returned 403 "Upgrade to GitHub Pro or make this repository
public" while private, and 404 "Branch not protected" immediately after the
flip).

`main` is protected, and this is **enforced by GitHub, not by convention**:

- Changes must go through a pull request (0 approvals required, so a solo
  author is not locked out).
- The `verify gate` status check must pass before merge. `strict` is on, so a
  branch must also be up to date with `main`.
- `enforce_admins` is on: the rules apply to the repo owner too.
- Force pushes and branch deletion are disabled on `main`.

Verified by attempting a direct push, which was rejected with
`GH006: Protected branch update failed` / `Changes must be made through a
pull request` / `Required status check "verify gate" is expected`.

If the required check is ever renamed, the protection rule must be updated to
match the new job name, or it can never pass. The context string is the
workflow job's `name:` — currently `verify gate` in `.github/workflows/ci.yml`.
- `.github/workflows/ci.yml` runs the verify gate (`rustc --version &&
  cargo --version && cargo check`, then `cargo test`) on push to `main` and on
  every PR, on `ubuntu-latest`.
- The runner installs Bevy's Ubuntu system packages, taken verbatim from the
  `docs/linux_dependencies.md` shipped in the bevy 0.19.1 crate. The dev
  machine is macOS and needs none of them.
- Tests are headless, so CI needs no GPU or display.
- CI cost note: the dev profile builds dependencies at `opt-level = 3`, so an
  uncached run compiles ~550 crates. `Swatinem/rust-cache` caches that; if CI
  time becomes a problem, add a leaner `[profile.ci]` rather than lowering the
  dev profile.

## Conventions

- Behaviour changes go spec -> failing test -> code. Add the SPEC line, add the
  test, watch it fail for the right reason, then implement.
- Refactors change no behaviour: the 4 tests stay green throughout.
- Work on a branch, merge via PR, keep CI green. Direct pushes to `main` are
  rejected by GitHub.
- This repo sets a local `user.email` of `adityapatil371@users.noreply.github.com`
  so the public history carries no personal address. Do not override it with
  `--author` or a global identity when committing here.
- Numbers in docs carry the command that produced them.
- Claims about Bevy or the toolchain are marked verified or assumed. Assumed is
  the default.

## Deliberately absent

Not added, and not to be added until a specific need appears: physics crate,
audio crate, asset pipeline, procedural generation, ML. The scaffold is minimal
on purpose.
