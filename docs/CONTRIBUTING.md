# Contributing to hydebar

Thank you for your interest in contributing to hydebar! This guide will help you get started.

## Development Philosophy

This project follows the [Rust Manifest](https://github.com/RAprogramm/RustManifest) - a set of task-oriented development principles:

- **Clear scope**: Define what changes and what doesn't
- **Acceptance criteria**: DoD (Definition of Done) before starting work
- **Plan first**: Present plan → Get ACK → Implement
- **Quality gates**: All code must pass `cargo +nightly fmt --all -- --check && cargo check && cargo test && cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **No version changes**: Don't modify crate versions, CHANGELOG, or Cargo.lock without separate task

## Code of Conduct

Be respectful, constructive, and professional. We're building a tool for the community.

## Ways to Contribute

### 1. Report Bugs

Found a bug? Help us fix it:

1. **Search existing issues** - Check if it's already reported: [GitHub Issues](https://github.com/RAprogramm/hydebar/issues)
2. **Create detailed report** with:
   - hydebar version: `hydebar --version`
   - System info: `uname -a`
   - Hyprland version: `hyprctl version`
   - Your config file (remove sensitive data)
   - Steps to reproduce
   - Debug logs: `RUST_LOG=debug hydebar 2>&1 | tee hydebar.log`

### 2. Request Features

Have an idea? We want to hear it:

1. **Check roadmap** - See if it's already planned: [ROADMAP.md](../ROADMAP.md)
2. **Open discussion** - Describe:
   - What you want
   - Why it's useful
   - How it might work
   - Example use cases

### 3. Submit Themes

Create a beautiful new theme:

1. **Design your theme** - Use existing themes as templates
2. **Add to themes.rs** - Follow existing pattern
3. **Add tests** - Verify theme loads correctly
4. **Update documentation** - Add to THEMES.md
5. **Submit PR** - Include screenshot

See [Theme Development](#theme-development) below.

### 4. Write Code

Implement features from the roadmap:

1. **Pick an issue** - Check [ROADMAP.md](../ROADMAP.md) for priorities
2. **Discuss first** - Comment on the issue before starting
3. **Follow guidelines** - See [Development Workflow](#development-workflow)
4. **Write tests** - Cover new functionality
5. **Update docs** - Keep documentation current

### 5. Improve Documentation

Help others understand hydebar:

- Fix typos and unclear sections
- Add examples and screenshots
- Write tutorials
- Translate documentation (future)

---

## Development Setup

### Prerequisites

- **Rust** (edition 2024, see `rust-version` in `Cargo.toml`)
- **Cargo** package manager
- **Hyprland** compositor (for testing)
- **Wayland** development libraries
- **Git** for version control

### System Dependencies

#### Arch Linux
```bash
sudo pacman -S base-devel wayland wayland-protocols rust
```

#### Ubuntu/Debian
```bash
sudo apt install build-essential libwayland-dev wayland-protocols rustc cargo
```

### Clone and Build

```bash
# Fork the repository on GitHub first

# Clone your fork
git clone https://github.com/YOUR_USERNAME/hydebar.git
cd hydebar

# Add upstream remote
git remote add upstream https://github.com/RAprogramm/hydebar.git

# Build
cargo build --release

# Run
./target/release/hydebar-app
```

---

## Development Workflow

### 1. Create a Branch

Always work on a feature branch:

```bash
# Update main
git checkout main
git pull upstream main

# Create branch (name it after issue number)
git checkout -b 123-feature-name
```

### 2. Make Changes

Follow Rust conventions:

```bash
# Format code — the only command that runs on nightly
cargo +nightly fmt

# Check for errors
cargo check

# Run tests
cargo test

# Fix clippy warnings; pedantic and nursery are on for the whole workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### Toolchains

`rust-toolchain.toml` pins the workspace to **stable**, so `check`, `test`
and `clippy` all run there without being asked. Formatting is the single
exception: every setting in `.rustfmt.toml` is nightly-only, and a stable
`rustfmt` would accept the file, ignore them and reformat the tree its own
way — so it is always `cargo +nightly fmt`.

`clippy::pedantic` and `clippy::nursery` are enabled in the workspace
manifest, alongside `missing_debug_implementations`, `unsafe_code` and a
denied `let_underscore_future`. They apply to every invocation, so there is
no flag to remember; CI runs the same command with `-D warnings`, and the
tree is expected to be free of them.

### 3. Commit Changes

Write clear, atomic commits:

```bash
# Check what changed
git status
git diff

# Stage changes
git add file1.rs file2.rs

# Commit with descriptive message
git commit -m "#123 feat: add feature X"
```

**Commit message format:**
- Start with the issue number and a type: `#123 feat:`, `#123 fix:`
- Use imperative mood: "add feature" not "added feature"
- Keep first line under 72 characters
- Add detailed explanation in body if needed
- No tool credit lines or trailers

### 4. Push and Create PR

```bash
# Push to your fork
git push origin 123-feature-name

# Create pull request on GitHub
gh pr create --title "Feature: Add X" --body "Implements #123

Summary of changes:
- Added feature X
- Updated documentation
- Added tests

Testing:
- Tested on Arch Linux with Hyprland
- All tests pass
- No clippy warnings
"
```

**PR Requirements:**
- Clear title describing the change
- Reference the issue number
- Explain what changed and why
- List testing performed
- Include screenshots for UI changes
- All tests must pass
- No clippy warnings
- Code must be formatted

### 5. Review Process

- Maintainer will review your PR
- Address feedback promptly
- Push updates to the same branch
- PR will be merged when approved

---

## Code Style

### Rust Conventions

Follow standard Rust style:

```rust
// Use descriptive names
pub struct AnimationConfig {
    pub enabled: bool,
    pub menu_fade_duration_ms: u64,
}

// Document public APIs
/// Animation configuration for menus and transitions.
pub struct AnimationConfig {
    /// Enable or disable animations globally.
    pub enabled: bool,
}

// Use Result for fallible operations
pub fn load_config() -> Result<Config, ConfigError> {
    // ...
}

// Prefer explicit types for clarity
let duration: Duration = Duration::from_millis(200);
```

### File size

A source file holds at most **150 lines of code**. Doc comments, doctests and
the file's own `#[cfg(test)] mod tests` are not counted — the limit measures
the code that runs in production, and a file's tests are expected to be
longer than what they cover.

Past the limit, the file becomes a directory named after it, one file per
responsibility, nested again where a submodule outgrows its own file. The
path a caller imports stays the same, so splitting a file is never a
breaking change for the rest of the tree.

```bash
# code lines in a file, tests and doc comments excluded
sed '/^#\[cfg(test)\]/,$d' core/src/modules/cpu.rs \
  | grep -vE '^\s*(///|//!|$)' | wc -l
```

### Formatting

Always run before committing:

```bash
cargo +nightly fmt
```

### Error Handling

Use proper error types:

```rust
// Good - specific error type
pub fn parse_config(path: &Path) -> Result<Config, ConfigError> {
    let contents = fs::read_to_string(path)
        .map_err(ConfigError::Io)?;

    toml::from_str(&contents)
        .map_err(ConfigError::Parse)
}

// Avoid - generic errors
pub fn parse_config(path: &Path) -> Result<Config, Box<dyn Error>> {
    // ...
}
```

### Testing

Write tests for new features:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_config_default() {
        let config = AnimationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.menu_fade_duration_ms, 200);
    }

    #[test]
    fn test_theme_loads_correctly() {
        let appearance = preset_appearance("catppuccin-mocha").expect("theme");
        assert!(appearance.animations.enabled);
    }
}
```

---

## Theme Development

### Creating a New Theme

Themes are data, not Rust. A preset is a whole appearance table in a TOML
file; the roster beside it embeds the file. **No Rust logic changes.**

1. **Copy an existing theme.** Duplicate one `.toml` in
   `proto/src/config/themes/`, rename it to a kebab-case name
   (`your-theme-name.toml`), and recolor it. Keep every key an appearance
   needs; a partial table falls back to `Appearance::default()` for the
   rest, which is easier to read as a full table.

2. **Add one row to `THEME_SOURCES`.** In `proto/src/config/themes.rs`,
   add `("your-theme-name", include_str!("themes/your-theme-name.toml"))`.
   The name is `appearance = "your-theme-name"`. `preset_appearance` and
   `preset_names` read the roster; nothing else changes.

3. **Check it loads.** `cargo test -p hydebar-proto` runs the theme
   round-trips; `preset_appearance("your-theme-name")` must return `Some`,
   `preset_appearance("no-such-theme")` stays `None`.

4. **Update documentation** in `docs/THEMES.md`:

```markdown
## Your Theme Name

\`\`\`toml
appearance = "your-theme-name"
\`\`\`

**Style:** Description
**Best For:** Use cases
**Colors:** Key colors (#hex values)
```

7. **Add screenshot** - Include in PR

---

## Module Development

### Creating a New Module

Scaffold it instead of copying an existing one:

```bash
cargo xtask new-module demo --kind polling --label "Demo" --doc "What it does."
```

Kinds: `simple` for a static entry, `polling` for a sampled readout,
`listener` for an entry fed by an event stream it owns, `menu` for a
listener entry opening a menu on press. The command writes the
configuration, state, view and trait files, then prints the wiring
checklist: the roster row, the `Config` field, the bus event and menu kind
where the shape needs them, and the gui dispatch arms mirroring the clock
module. Follow it top to bottom — the completed wiring passes
`cargo check --workspace`, which also names any arm left behind. Finish
with `cargo fmt`, `cargo clippy` and `cargo test` like any other change.

What lands on disk: `proto/src/config/<name>.rs` holds the module
configuration, `core/src/modules/<name>.rs` the shared state and messages,
`core/src/modules/<name>/{module,state,view}.rs` the trait implementation,
the message folding and the drawing. Fill the marked `TODO`s, replacing the
placeholder readout, events and cadences with the real ones. A module
lives in `core/src/modules/` and implements the `Module` trait from
`core/src/modules.rs` (`register`, `deregister`, `poll_schedule`/`poll`,
`subscription`); drawing stays in the module's own `bar_view`.

See existing modules for reference: `keyboard_submap` is a small listener
one, `media_player/` a large one split into `state`, `messages`,
`commands` and `view`.

---

## Testing Guidelines

### Run Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_animation_config

# With output
cargo test -- --nocapture

# Documentation tests
cargo test --doc
```

### Test Coverage

Aim for:
- All public APIs tested
- Edge cases covered
- Error paths tested
- Integration tests for complex features

### Performance Testing

The bar earns its place on a desktop by costing almost nothing while it stands
there, so a performance change is only done when it is measured. The first tool
is the resource budget check, not a profiler: it turns "feels light" into a
number and a pass/fail.

```bash
# Measure a running bar against the budgets (finds the one running bar, or pass a PID)
cargo xtask resource-check
cargo xtask resource-check <pid>
```

It reads the kernel's own accounting from `/proc` for a bar that is already
running under a live compositor: resident memory, anonymous (heap) memory,
thread count, and the CPU it burns while idle, each against a budget. The exit
status is non-zero when a budget is exceeded, so the same command gates a
release. It measures a running process rather than spawning one because the bar
only lives under a Wayland compositor — a harness that started its own would be
testing the compositor, not the bar.

Budgets are read from the environment, so a baseline can be tightened without a
code change: `RSS_MAX_KIB`, `ANON_MAX_KIB`, `THREADS_MAX`, `CPU_MAX_PCT`,
`SETTLE_SECS`, `CPU_SECS`. The defaults are a measured floor plus headroom, not
aspiration.

**What the floor is made of.** On a Radeon under the default renderer the bar
resides at about 96 MiB, and most of it is not the bar: the graphics driver
stack (LLVM for the Vulkan shader compiler, `libgallium`, the Radeon driver)
accounts for roughly 46 MiB on its own. The bar's own heap is a fraction of
that. Two consequences worth knowing before chasing a number:

- The bar probes for a hardware Vulkan device at startup and names Vulkan to
  wgpu when one is present, so it does not also map the OpenGL driver — worth
  about 19 MiB and two threads. A `WGPU_BACKEND` in the environment overrules
  the probe.
- The software renderer (`ICED_BACKEND=tiny-skia`) was measured and is worse on
  this workload — about 320 MiB resident and twice the idle CPU, because it
  keeps a full framebuffer in RAM and composites on the CPU. The GPU renderer
  is the cheaper default; the software path is a fallback for a machine with no
  GPU, not a lighter option.

For deeper profiling once a budget is the thing to move:

```bash
# Where the CPU goes
perf record --call-graph dwarf ./target/release/hydebar
perf report

# Where the allocations go
heaptrack ./target/release/hydebar

# Which mappings hold the resident memory
awk '/^[0-9a-f]+-/{n=$6; if(n=="")n="[anon]"} /^Rss:/{r[n]+=$2} END{for(k in r) printf "%9d KiB  %s\n", r[k], k}' /proc/<pid>/smaps | sort -rn | head
```

---

## Documentation Guidelines

### Code Documentation

Document public APIs:

```rust
/// Animation configuration for menus and transitions.
///
/// Controls fade-in/fade-out effects and hover animations.
///
/// # Examples
///
/// ```
/// use hydebar_proto::config::AnimationConfig;
///
/// let config = AnimationConfig {
///     enabled: true,
///     menu_fade_duration_ms: 200,
///     hover_duration_ms: 100,
/// };
/// ```
pub struct AnimationConfig {
    /// Enable or disable all animations globally.
    pub enabled: bool,

    /// Duration of menu fade animations in milliseconds.
    pub menu_fade_duration_ms: u64,
}
```

### User Documentation

Update relevant docs:
- `README.md` - Overview, quick start, custom modules, system info
- `docs/GETTING_STARTED.md` - First-time setup
- `docs/THEMES.md` - Theme showcase
- `docs/TROUBLESHOOTING.md` - Common issues
- `docs/FAQ.md` - Frequently asked questions

---

## Release Process

Maintainers handle releases:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. **Check the resource budget.** Build the release binary, run it on a real
   session, and measure it: `cargo build --release && cargo xtask
   resource-check`. Every budget must hold. A regression here is a regression in
   the one promise the bar makes — that it costs nothing while idle — so it is
   settled before the tag, not after. See [Performance
   Testing](#performance-testing) for what the budgets mean and what the floor
   is made of.
4. Create git tag: `git tag vX.Y.Z`
5. Push tag: `git push origin vX.Y.Z`
6. GitHub Actions builds packages
7. Publish to AUR, Nix, etc.

---

## Getting Help

### For Contributors

- **Discussions** - Ask questions: [GitHub Discussions](https://github.com/RAprogramm/hydebar/discussions)
- **Issues** - Check existing issues: [GitHub Issues](https://github.com/RAprogramm/hydebar/issues)
- **Code review** - Learn from PR feedback

### Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [iced Documentation](https://docs.rs/iced/)
- [Wayland Protocol](https://wayland.freedesktop.org/docs/html/)
- [Hyprland Wiki](https://wiki.hyprland.org/)

---

## License

By contributing, you agree that your contributions will be licensed under the
GNU General Public License, version 3 or later, the licence the bar as a whole
is distributed under.

---

## Recognition

Contributors are listed in:
- GitHub contributors page
- Release notes
- Special recognition for major features

---

**Thank you for contributing to hydebar!** Every contribution, big or small, helps make hydebar better for everyone.
