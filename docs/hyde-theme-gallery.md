# Installing HyDE themes from the themes window

What the upstream gallery offers, and the mechanism the bar can build on it.

## How HyDE distributes themes

- The catalogue is one JSON file: the `hyde-gallery` repository, branch
  `hyde-gallery`, file `hyde-themes.json`. Each entry carries `THEME` (name),
  `LINK` (a repository or a `/tree/<branch>` URL — official themes are
  branches of `hyde-themes`, community ones live in their authors' repos),
  `OWNER`, `DESCRIPTION`, and `COLORSCHEME` — exactly two hex colours.
  42 themes at the time of writing.
- Screenshots are not centralised: they live inside each theme's repository.
  The two `COLORSCHEME` colours are the only preview the index guarantees.
- Install is scriptable, no prompt:
  `hydectl theme import --name "<name>" --url "<link>"`.
- Update is scriptable too: `hyde-shell theme.import --fetch "<name>"`, or
  `--fetch all` for everything under `~/.config/hyde/themes`.
- Removal has no documented command; a theme is a directory under
  `~/.config/hyde/themes/<name>` and upstream simply deletes it.

## The mechanism for the themes window

The window already draws every installed theme as a chip painted in that
theme's own swatch, and owns the one running-switch lock. The gallery slots
under it as a second section with the same grammar:

1. **Index** — a small reader in core fetches `hyde-themes.json` (raw
   GitHub URL), caches it under the XDG cache directory with a
   time-to-live of a day, and parses it into `GalleryTheme { name, link,
   owner, description, colors: [Rgba; 2] }`. No network on the bar's hot
   path: the fetch runs when the themes window opens and the cache is
   stale, on the runtime, publishing through the module's event sender
   like every other service. Offline or a bad response → the section is
   simply absent; the cache serves what it has.
2. **Section "Gallery"** — chips for every indexed theme that is not
   already installed (installed names come from the state the window
   already holds). A chip is painted from its two `COLORSCHEME` colours —
   surface and accent — through the shared `theme_chip` widget, with the
   `DESCRIPTION` as its hover hint through the standard tooltip path.
3. **Install** — pressing a gallery chip runs
   `hydectl theme import --name … --url …` through the same supervised
   runner the switch already uses: the chip enters the `Applying` spinner
   state, every other gallery chip is `Blocked` (an import writes into the
   same theme directories a second import would race), and the switch
   section is blocked for the duration too. Success → refresh the
   installed list (the state file watcher already reports it) and offer
   the new chip in the installed grid; failure → the standard refusal
   notice with the tool's stderr line.
4. **Update** — a later step, not the first cut: `--fetch <name>` behind a
   small refresh glyph on installed chips that came from the gallery.

## Why this shape

- The bar never parses theme repositories itself: the import tool owns the
  format, the bar owns the surface. If upstream changes the theme layout,
  nothing here breaks — only the JSON index matters, and its schema is
  five fields.
- One lock for switch and import together mirrors the reality that both
  write the same directories.
- Chips from index colours keep the window honest offline and need no
  image pipeline; screenshots can come later by fetching each repo's
  preview into the cache, behind the same TTL.

## Prerequisites and risks

- `hydectl` must be present (it ships with HyDE; probe once and hide the
  section when absent, the way unavailable modules already hide).
- An import can take tens of seconds on a slow network — the spinner and
  the block are load-bearing, and the runner must survive the window
  closing mid-install.
- Names in the index may collide with an installed theme of a different
  origin; installed names win and such entries stay hidden.
