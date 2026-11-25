<!-- # ![Feather](./assets/feather.svg) Pluteus -->
<div align="center">

  [![latest version](https://img.shields.io/github/v/release/echoudhury/pluteus)](https://github.com/EChoudhury/Pluteus/releases)

  <img width="150" alt="Pluteus logo" src="./assets/pluteus.png">
  <h3>Pluteus</h3>
  A desktop writing app for focused, session-based writing. Vibe-coded for your writing pleasure.
  <br/><br/>
  <!-- <img width="800" alt="FileBrowser Quantum file listing in dark mode" src="https://filebrowserquantum.com/images/generated/listing/view-mode-normal-dark.jpg"> -->
</div>

## Features

- Built with Rust, [iced](https://iced.rs) 0.13, tokio, and rfd.
- Open an existing `.txt` / `.md` document, or create a blank one in a location of your choice.
- Session goals: a countdown in minutes, or a number of newly written words.
- Autosave: every 10 new words, after 10 seconds of inactivity, and when leaving the writer.
- Typography: font size, line spacing, and four typefaces (Courier, Sans serif, Times New Roman, Montserrat).
- Themes: Dark, Light, Sepia.
- Optional full-screen sessions and optional session/autosave readouts.
- Focus blur: only the sentence at the cursor stays fully opaque; toggled on the setup screen.
- Native file dialogs via `rfd`; file I/O runs without blocking the UI.

## Requirements

- Stable Rust (edition 2021).
- Windows: MSVC build tools (the default rustup toolchain).
- Linux: an XDG desktop portal for the native file dialogs.
- macOS: no extra setup.

## Build and run

```bash
cargo run
cargo run --release
```

Checks:

```bash
cargo test
cargo clippy --all-targets
```

## Usage

1. Under **Document**, choose **Open file…** or **New file…**.
2. Pick a goal (minutes or words), then adjust typography and theme.
3. Optionally enable **Full screen**, **Session info**, or **Focus blur**.
4. Start writing. `Esc` saves and returns to the setup screen.

## Project layout

```
src/main.rs     app bootstrap, screen routing, window mode
src/types.rs    messages, state, themes, typefaces
src/setup.rs    setup screen, file binding, settings
src/writer.rs   editor, session timer, autosave, subscriptions
src/focus.rs    sentence-level focus highlighter
src/toast.rs    floating notification overlay
assets/         bundled Fira Sans and Montserrat fonts (OFL)
```

## Notes

- Focus blur is a low-contrast fade implemented with iced's text highlighter; iced does not expose a GPU blur filter.
- Fira Sans and Montserrat are bundled in `assets/` under the SIL Open Font License 1.1 (see `assets/*-OFL.txt`). Courier New and Times New Roman come from the system fonts.
- Autosave replaces the bound file with the current editor contents.

## License

Project code: MIT. Bundled fonts: SIL OFL 1.1.
