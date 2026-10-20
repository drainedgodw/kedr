# Dependency-complete runtime packaging — preview gates

This revision improves distribution of **CodeCedar only**. The other 32 catalog
entries still have no implementation, executable or releasable runtime. They must
not inherit CodeCedar's test verdict or be shipped as renamed empty wrappers.

## What “dependencies included” means

- On a package-managed Linux installation, the package manager installs required
  shared libraries from the distribution's configured repositories. They are not
  copied into the application or fetched by a privileged curl/bootstrap script.
- Windows MSVC builds statically link the C runtime. A PE import audit checks both
  normal and delay-loaded DLL tables and refuses non-system DLL dependencies.
- The macOS builder accepts only Apple system dylibs/frameworks, packages the native
  executable/icon/metadata/license in a `.app`, ad-hoc signs it and creates a DMG.
  It rejects an unbundled Homebrew or `@rpath` dependency instead of silently
  producing an app that only works on its build machine.
- The OS, GPU/desktop drivers and user project toolchains are not bundled.
  Cargo/rustc, rust-analyzer, Rust standard-library sources and project dependencies
  remain optional external developer tools. Their setup is **not solved** by this
  runtime packaging revision. Running project code and language analysis still
  requires the user's explicit trust. No silent rustup installation is added.

These are unsigned previews, not a stable release, signed update repository or a
claim of support for every distribution/CPU/desktop. Build success, package install,
native rendering and interactive/physical desktop acceptance are separate gates.

## Installation routes

| Target | Artifact / command | Dependency resolver | Acceptance boundary |
| --- | --- | --- | --- |
| Arch x86_64 | `packaging/arch/PKGBUILD`, `makepkg -si` | pacman | Actual Arch build/install/help/remove already exercised in Studio CI; no AUR publication |
| Ubuntu 24.04 x86_64 | matching `.deb`, `sudo apt install ./codecedar_*.deb` | apt | New distribution CI builds on Ubuntu and installs in a separate clean Ubuntu image |
| Fedora 43 x86_64 | matching `.rpm`, `sudo dnf install ./codecedar-*.rpm` | dnf | New distribution CI builds on Fedora and installs in a separate clean Fedora image |
| Windows MSVC | `CodeCedar-Setup.exe` | C runtime linked into executable; Windows supplies OS DLLs | PE import gate plus existing NSIS install/remove acceptance; hosted result required |
| macOS runner architecture | `CodeCedar-macOS.dmg` → drag `CodeCedar.app` to Applications | Apple system frameworks only | Mach-O linkage, bundle signature, executable help and DMG integrity; no notarization/physical GUI acceptance |
| Other Linux / ARM64 | no newly accepted universal artifact | not claimed | Builder recognizes ARM64 ELF; this is not an ARM64 release/acceptance result |

Download only the artifact from a successful corresponding workflow job. A newly
added workflow is not evidence that the workflow has passed. Check the exact commit,
job result and artifact architecture before distributing it.

Use `apt install ./file.deb`, not just `dpkg -i`: dpkg alone does not download missing
dependencies. Use `dnf install ./file.rpm`, not `rpm --nodeps`. These routes require
network access for missing distro packages and sudo authorization. No package
manager security/signature checks are disabled. The downloaded application package
itself is still unsigned; checksums alone do not establish publisher authenticity.

Native packages own `/usr/bin/codecedar`, the desktop entry and hicolor SVG icon.
They do not execute project code, modify shell startup files, change default file
associations, or manage user documents/preferences/recovery. They install no service
and require no project-specific install-time scriptlet. Package managers may refresh
standard icon/menu caches through their own distribution triggers.

Remove an older per-user XDG installation first if it shadows `/usr/bin/codecedar`
on PATH. Native and per-user installations are distinct; they do not overwrite or
pretend to upgrade one another. The existing offline selector is an advanced fallback,
**not dependency-complete**: it still needs Python and preinstalled desktop libraries.

Removal: `sudo apt remove codecedar`, `sudo dnf remove codecedar`, or
`sudo pacman -R codecedar-git`. On macOS remove the app from Applications. User data
is deliberately preserved. No automatic update channel or cross-format migration
is delivered here.

## Build contracts

```bash
# Run on the target distro, with its packaging tools and the same architecture.
cargo build --locked --release -p kedr-studio
python3 scripts/build_native_linux.py --format deb --binary target/release/kedr-studio --out native-packages
python3 scripts/build_native_linux.py --format rpm --binary target/release/kedr-studio --out native-packages
# On native macOS only:
python3 scripts/build_macos_app.py --binary target/release/kedr-studio --out macos-package
# On Windows, after a project-configured MSVC build:
python scripts/check_windows_runtime.py --binary target/release/kedr-studio.exe
```

Debian requires `dpkg-dev`/`dpkg-deb`; RPM requires `rpm-build`. Explicit runtime
requirements cover graphics/keyboard/portal libraries loaded through dlopen.
`dpkg-shlibdeps` and RPM's ELF dependency generator additionally derive actual
linked-library ABI requirements (including glibc). Missing ABI scanner output is
an error, not a reason to emit a dependency-free package. A binary built on a newer
distro is not relabelled as supporting an older glibc.

The declared XDG portal service is required; GTK is recommended by default on apt/dnf.
Users with a working KDE/other portal backend may choose their desktop's backend
instead. An actual working portal session still requires desktop acceptance; package
presence does not prove a native folder picker works on every compositor.

## Automated acceptance and limits

`.github/workflows/distribution.yml` separates build images from clean runtime images.
`scripts/test_native_linux.sh` is destructive and refuses to run without explicit
`KEDR_DISPOSABLE_PACKAGE_TEST=1`; it is for disposable containers only, not a user's PC.
It installs the real package with its package manager, checks linked libraries and
`--help`, then adds Xvfb/Mesa test equipment and renders the installed executable as
an unprivileged user. It reinstalls/removes the package and checks user-file retention.
The Xvfb welcome capture runs no project tasks and is not an interactive Wayland,
accessibility, IME, multi-GPU, crash-restoration or full IDE acceptance test.

Python tests cover malformed/truncated binaries, architecture detection, symlink
refusal, unsafe versions, fail-closed ABI scanning, staging ownership paths, Mach-O
system-only linking/minimum OS metadata and normal/delay PE import dependency checks.
Synthetic fixtures test parsers/contracts, not whether a native app executes.

## Still required before a stable product release

1. Native hosted build/install acceptance of this revision on every advertised target.
2. Signed artifacts/notarization where applicable; reproducible provenance and updates.
3. Physical desktop/portal/IME/accessibility/performance/endurance acceptance.
4. An explicit, usable developer-toolchain acquisition story (without running code
   or privileged download scripts behind the user's back).
5. Actual implementations and separate product-specific acceptance for the 32 plans.

See [full product audit](PRODUCT_AUDIT.md), [installation](INSTALL.md) and
[revision acceptance](DISTRIBUTION_VALIDATION.md) for evidence rather than promises.
