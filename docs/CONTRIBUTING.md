# Contributing to hydebar

Thank you for your interest in contributing to hydebar! This guide will help you get started.

## Development Philosophy

This project follows the [Rust Manifest](https://github.com/RAprogramm/RustManifest) - a set of task-oriented development principles:

- **Clear scope**: Define what changes and what doesn't
- **Acceptance criteria**: DoD (Definition of Done) before starting work
- **Plan first**: Present plan → Get ACK → Implement
- **Quality gates**: All code must pass `cargo check && cargo test && cargo fmt && cargo clippy`
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

- **Rust** 1.97+ (edition 2024, see `rust-version` in `Cargo.toml`)
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
# Format code
cargo +nightly fmt

# Check for errors
cargo check

# Run tests
cargo test

# Fix clippy warnings
cargo clippy --all-targets --all-features
```

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
        let theme = PresetTheme::CatppuccinMocha;
        let appearance = theme.to_appearance();
        assert_eq!(appearance.animations.enabled, true);
    }
}
```

---

## Theme Development

### Creating a New Theme

1. **Choose colors** - Pick a cohesive palette
2. **Add to PresetTheme enum** in `crates/hydebar-proto/src/config/themes.rs`
   (the enum is `#[serde(rename_all = "kebab-case")]`, so `YourThemeName`
   becomes `appearance = "your-theme-name"`):

```rust
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PresetTheme {
    // ... existing themes
    YourThemeName
}
```

3. **Implement theme function** — follow the existing constructors in
   `themes.rs`; colors are `AppearanceColor::Simple(HexColor::rgb(r, g, b))`:

```rust
fn your_theme_name() -> Appearance {
    Appearance {
        style: AppearanceStyle::Islands,
        opacity: 0.95,
        background_color: AppearanceColor::Simple(HexColor::rgb(26, 27, 38)),
        primary_color: AppearanceColor::Simple(HexColor::rgb(122, 162, 247)),
        secondary_color: AppearanceColor::Simple(HexColor::rgb(22, 22, 30)),
        success_color: AppearanceColor::Simple(HexColor::rgb(158, 206, 106)),
        danger_color: AppearanceColor::Simple(HexColor::rgb(247, 118, 142)),
        warning_color: AppearanceColor::Simple(HexColor::rgb(224, 175, 104)),
        text_color: AppearanceColor::Simple(HexColor::rgb(192, 202, 245)),
        workspace_colors: vec![
            AppearanceColor::Simple(HexColor::rgb(122, 162, 247)),
            AppearanceColor::Simple(HexColor::rgb(187, 154, 247)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(
            HexColor::rgb(247, 118, 142)
        )]),
        menu: MenuAppearance {
            opacity: 0.95,
            backdrop: 0.3
        },
        ..Appearance::default()
    }
}
```

4. **Add to match statement** in `to_appearance()`:

```rust
impl PresetTheme {
    pub fn to_appearance(self) -> Appearance {
        match self {
            // ... existing themes
            PresetTheme::YourThemeName => your_theme_name(),
        }
    }
}
```

5. **Add tests**:

```rust
#[test]
fn your_theme_name_loads() {
    let theme = PresetTheme::YourThemeName;
    let appearance = theme.to_appearance();
    assert!(appearance.animations.enabled);
}
```

6. **Update documentation** in `docs/THEMES.md`:

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

A module lives in `crates/hydebar-core/src/modules/` and implements the
`Module` trait from `crates/hydebar-core/src/modules.rs` (`register`,
`deregister`, `poll_schedule`/`poll`, `view`, `subscription`). Wiring it up
touches four places:

1. **Name** - add a variant to `ModuleName` (and its `BUILT_IN` list,
   `as_str`, `label`) in `crates/hydebar-proto/src/config/modules.rs`
2. **Module** - the state, logic and iced view in
   `crates/hydebar-core/src/modules/your_module.rs` (split into
   `state`/`view`/`commands` submodules when it grows)
3. **Events** - a `ModuleEvent` variant in
   `crates/hydebar-core/src/event_bus.rs` if the module publishes background
   updates
4. **Registration and dispatch** - `crates/hydebar-gui/src/app/` —
   `update/registration.rs` (register while hosted, release when not),
   `update/modules.rs` and the module actions/element wiring under
   `app/modules/`

Any per-module configuration gets its own file under
`crates/hydebar-proto/src/config/` and a field on `Config`.

See existing modules for reference; `modules/weather.rs` is a small
self-contained example, `modules/media_player.rs` a large one organised with
inline `mod` blocks. Modules are always one flat file, never a folder.

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

For performance-critical changes:

```bash
# Profile with perf
perf record --call-graph dwarf ./target/release/hydebar-app
perf report

# Memory profiling
heaptrack ./target/release/hydebar-app

# Benchmark comparisons
hyperfine "./target/release/hydebar-app" "waybar"
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
3. Create git tag: `git tag vX.Y.Z`
4. Push tag: `git push origin vX.Y.Z`
5. GitHub Actions builds packages
6. Publish to AUR, Nix, etc.

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

By contributing, you agree that your contributions will be licensed under the MIT License.

---

## Recognition

Contributors are listed in:
- GitHub contributors page
- Release notes
- Special recognition for major features

---

**Thank you for contributing to hydebar!** Every contribution, big or small, helps make hydebar better for everyone.
