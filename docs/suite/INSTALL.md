# Preview installation

Only CodeCedar is implemented. The installer lists the entire catalog but refuses
planned products. This is an unsigned preview, not a stable/signed release.

## Linux: offline selector and automatic menu/icon registration

Download a successful Linux installer artifact from the Studio Actions workflow,
extract it and run from the extracted directory:

```bash
bash install.sh
# Or non-interactive installation of the one available product:
bash install.sh --product codecedar
bash install.sh --verify
bash install.sh --uninstall
```

The selector is a terminal menu, not a graphical wizard. It shows which products
are available and which are planned. No network request, sudo, project execution,
cloud account or desktop-environment guessing occurs during this installation.

It installs for the current user using XDG conventions:
- executable under `$XDG_DATA_HOME/kedr/codecedar` (default `~/.local/share`);
- launcher `~/.local/bin/codecedar`;
- `.desktop` entry under `$XDG_DATA_HOME/applications`;
- SVG and 48/128/256/512 PNG icons in the hicolor icon theme;
- a hashed ownership record for safe repeat-install and removal.

GNOME/KDE/XFCE and other XDG desktops discover the application through their menu
standard. Menu/icon caches are refreshed when the corresponding tools exist.
A menu can refresh lazily; log out/in if the desktop does not notice new entries.
No shell profile, default application, file association or user project is changed.
Unowned/modified destination files and symlinks are refused. Corrupt payloads are
rejected before copying. Checksums detect corruption, not authenticity of an
unsigned distributor. Removal preserves modified files, projects and preferences.

Requirements: Linux x86_64 with glibc, Python 3.10+, a working X11/Wayland desktop,
OpenGL driver, libxkbcommon/X11 libraries and an appropriate XDG portal backend.
The offline installer does **not** resolve distro packages. For Arch dependency
resolution and pacman-managed installation, use the route below instead.

## Arch: the same makepkg/pacman mechanism, without AUR publication

Review the PKGBUILD and source first, then:

```bash
git clone https://github.com/drainedgodw/kedr.git
cd kedr/packaging/arch
bash install.sh
# Or, as a regular user:
makepkg -si
# Remove the managed package:
sudo pacman -R codecedar-git
```

`base-devel`, Git and a regular non-root build user are required. `makepkg -si`
resolves declared build/runtime dependencies through pacman and installs the local
`.pkg.tar.zst`. No AUR account is needed. This is a VCS development package from
the repository, not a registered AUR package. Pin/review the checkout for releases.
The package uses `/usr/bin`, `/usr/share/applications` and hicolor SVG icons.
Choose a portal backend appropriate for your desktop separately.

## Windows: ordinary per-user setup wizard

Download `CodeCedar-Setup.exe` from a successful Windows installer artifact.
The NSIS wizard includes Welcome, License, Components, Directory, Installation
and Finish pages. It installs to the current user's Programs directory, adds a
Start menu shortcut with the product icon, optionally adds a Desktop shortcut,
and registers an uninstaller in Windows Apps settings. Other products are disabled.
No administrator elevation or default-file-association takeover is requested.

The installer is unsigned; Windows may show a reputation warning. No signing or
SmartScreen bypass is promised. Silent installation and removal are supported
for automated acceptance. The uninstaller removes shipped paths, not recursively
the whole install folder; unrelated files survive. Modified shipped Windows files
are not individually hash-protected like the Linux user installer.

## macOS and acceptance limits

macOS remains a build/archive preview; no new macOS installer/notarization is
claimed. CI/container evidence and simulated filesystem tests are distinct from
physical Arch/Wayland or Windows/macOS desktop acceptance. Read VALIDATION before
calling every platform/product tested.

See [validation](../studio/VALIDATION.md) and [feature gaps](PARITY.md).
