# Kedr Studio

A native, local-first development workspace built on the Kedr Rust text core.
**Status: first desktop preview, not a complete JetBrains replacement.**

## What works now

- Project explorer, tabs, quick file navigation and searchable command palette.
- Viewport-only code rendering, basic line syntax colors and configurable dark/light themes.
- UTF-8 editing, Unicode-scalar cursor movement, selections, clipboard, undo/redo,
  autoindent, transactional multiline indentation and literal find/replace.
- Bounded saved-project-file search in the background.
- Explicit save, external-change refusal, unsaved-change guards for close/switch/quit.
- Real Cargo check/test output, only after explicit project trust/confirmation.
- Native folder picker, recent workspaces and appearance preferences.

The shell and design system live in `kedr-ui`, not a product-specific UI fork.
Future products must reuse that shell, navigation and interaction vocabulary.
There is currently **one desktop product**, not several renamed copies.

## Download preview builds

Open [Studio builds](https://github.com/drainedgodw/kedr/actions/workflows/studio.yml),
select a successful run for the desired commit, and download its platform artifact.
Artifacts are named `kedr-studio-<OS>-<architecture>`. GitHub may require sign-in to
retrieve Actions artifacts. Availability is determined by the actual job result,
not by this guide or a workflow configuration. These are unsigned preview builds;
there is no stable installer or notarized macOS distribution yet.

## Build on Arch Linux

Install Rust 1.88+ and normal desktop graphics/portal dependencies:

```bash
sudo pacman -S --needed rustup base-devel mesa libxkbcommon libxkbcommon-x11 libx11 libxcursor libxi libxrandr xdg-desktop-portal
rustup default stable
git clone https://github.com/drainedgodw/kedr.git
cd kedr
cargo build --locked --release -p kedr-studio
./target/release/kedr-studio /path/to/your/project
```

Install the portal backend appropriate to the desktop, e.g. `xdg-desktop-portal-gtk`
or `xdg-desktop-portal-hyprland`. If the folder picker is unavailable, pass the
workspace directory on the command line. Linux needs a working X11 or Wayland
session and OpenGL driver. Local validation used Linux X11 with software rendering,
not the maintainer's Arch machine or a physical Wayland session.

## Build on Windows

Install Rust stable via rustup and Visual Studio Build Tools with Desktop C++ tools.
In PowerShell:

```powershell
git clone https://github.com/drainedgodw/kedr.git
cd kedr
cargo build --locked --release -p kedr-studio
.\target\release\kedr-studio.exe "C:\Users\you\code\project"
```

Windows/macOS build and model/UI test jobs are configured in Actions. Do not assume
that an unobserved run succeeded. macOS also requires Xcode command-line tools for
source builds; it has not been manually inspected on a physical Mac.

## First five minutes

1. Open a workspace with the welcome button or supply its directory at launch.
2. Choose a file in the explorer or use **Ctrl P**.
3. Use **Ctrl Shift P** to discover commands; **Ctrl F** finds in the current file.
4. **Ctrl S** saves. Changed-on-disk files are refused, not silently overwritten.
5. In a Cargo workspace, **Check / F5** opens an explicit trust decision before running code.

On macOS the command modifier is Command; the current UI shortcut hints use Ctrl.
Use the settings icon for Midnight/Daylight appearance and code font size.

## Limits you should know

- No LSP completion, semantic refactoring, debugger, terminal emulator, Git UI,
  plugin runtime, remote development or recovery/autosave implementation yet.
- Syntax colors are a simple line lexer, not a parser/type checker.
- Cursor movement is by Unicode scalars, not grapheme clusters. System-level IME
  integration is implemented but only synthetic composition events were tested.
- Documents are limited to 16 MiB, with 32 open tabs in this preview. A single
  paste is limited to 1 MiB; replace-all to 10,000 matches; indent to 4,096 lines.
- Very long lines use an 8 KiB rendered byte window around the caret. Full contents
  remain in the core and are saved without truncation. No long-line wrapping/minimap.
- Project discovery is bounded. Saved-file search reads up to 1 MiB/file and 32 MiB
  total, returning up to 1,000 hits; it does not search unsaved tab contents.
- Saving is currently synchronous. Cancellation stops the direct Cargo child;
  process-tree cancellation and exit-drain ordering are not guaranteed yet.
- Native widgets expose accessible labels; the custom editor does not yet provide
  a complete screen-reader text/selection model. Default bundled font coverage is
  not all writing systems. There is no accessibility-compliance claim.
- Appearance/recent-path preferences are stored locally. No cloud service,
  telemetry, account or paid activation is required by Kedr.

See [product direction](../docs/studio/PRODUCT.md), [shared UI contract](../docs/studio/DESIGN.md)
and [validation](../docs/studio/VALIDATION.md).
