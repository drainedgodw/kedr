# Distribution revision — observed evidence

Baseline: `c8ea79cc7bb7e5ac2fabdb28f3d13d46ffe58298` (`audit`).
This record distinguishes local observation from newly configured hosted gates.

## Real preceding hosted failures, not “all green”

[Studio run 39387663786](https://github.com/drainedgodw/kedr/actions/runs/39387663786)
finished with Linux desktop, Arch makepkg/pacman and MSRV success. Windows and macOS
failed two worker recovery-delete tests: discovery kept a noncanonical storage path,
then deletion rejected it. Windows verbatim prefixes and macOS ancestor aliases
exposed the bug. New discovery canonicalizes the storage base before recording
snapshot identities while continuing to reject symlinked session directories.

A new Unix aliased-storage regression was run against the pre-fix discovery and
**failed**, then passed after the fix. The existing symlinked-session rejection and
worker deletion tests still pass locally. This is not a claim that Windows/macOS
were executed locally; their new hosted results are required.

## Local checks actually executed

- Full Rust workspace: **192 passed, 0 failed, 2 explicitly ignored live tests**.
- The two real rust-analyzer navigation integration tests: **2 passed separately**.
- Python: **38 product/distribution/parser contract tests + 19 installer tests**.
  Synthetic ELF/Mach-O/PE fixtures validate parsers/staging, not native OS execution.
- Full catalog structural inventory: **33 entries, 1 runtime, 32 NOT_IMPLEMENTED**.
  Installer negative coverage refuses every one of the 32 planned entries.
- `cargo fmt --all --check`, workspace/all-target Clippy with `-D warnings`,
  release CodeCedar build, generated keymap/catalog validation and diff whitespace.

## Real deb install on a clean Ubuntu rootfs: PASS

A fresh official Ubuntu Base **24.04.5 amd64** rootfs was unpacked separately from
the build rootfs. Its archive SHA256 matched the official HTTPS SHA256SUMS:
`e77b6f10c2590cef872b33ee9f635a0e3fd1f57fb074c0e52b5c7f56147a0c86`.
This is a chroot on the agent's Linux kernel, not a separate VM/physical Ubuntu PC.
The application binary was locally compiled on Amazon Linux, then packaged with
Ubuntu dpkg-shlibdeps; hosted CI additionally builds inside the target Ubuntu image.

Observed with the real `.deb` and `apt`, not a shell-script stand-in:

1. Install local package; apt downloads/resolves required graphics/keyboard/portal
   dependencies from Ubuntu repositories with normal repository verification.
2. Query installed package dependencies; run `/usr/bin/codecedar --help`; inspect
   linked libraries for unresolved entries.
3. Add Xvfb/Mesa testing equipment only after initial dependency installation.
4. Start the installed executable as a non-root user, software-render the native
   welcome window at 900×640; validate PNG. The single image was individually
   reviewed: welcome text/buttons fit, no clipping observed.
5. Reinstall the actual package through apt, then remove it.
6. Confirm binary/menu/icon removed and an unrelated user configuration file kept.

This does not exercise physical Wayland, real portal dialogs, project compilation,
Git/debugger functionality, IME, accessibility or a complete working-day scenario.
APT's chroot `/dev/pts` logging warnings were present; commands and assertions
completed successfully. Repository signature checking was never disabled.

## RPM: real build, not yet a Fedora install verdict

`rpmbuild -bb` built the real CodeCedar ELF with explicit graphics/portal package
requirements plus automatic ELF/glibc requirements. Payload paths and required
metadata were queried. Local environment is Amazon Linux, not Fedora. A successful
local RPM build is **not** Fedora installation/GUI acceptance. The new separate
Fedora 43 build/clean-install jobs must pass before recommending that artifact.
An earlier tiny C executable was used only as a packaging harness probe; it is
not shipped or counted as product acceptance.

## Windows/macOS changes: native hosted gates required

- MSVC target config requests static CRT. A PE parser checks normal and delayed
  DLL imports against Windows system DLLs and rejects unshipped vendor/VC runtimes.
  Native Windows build plus NSIS install/uninstall must still pass.
- macOS builder checks system-only dylibs, derives minimum OS from actual Mach-O
  load commands (not linker tool version), generates icons/Info.plist, ad-hoc signs
  the app and verifies the DMG. A native test mounts the DMG, copies/validates/runs
  `--help`, replaces/removes only its temporary app, retaining an unrelated file.
  No local macOS execution, Developer-ID signing, notarization or Gatekeeper end-user
  acceptance is claimed by the Linux tests.

## Setup failures retained as evidence

- First shell access found the old workspace absent; restored the exact published
  source from GitHub, not an older local snapshot.
- Initial Ubuntu apt setup failed because the unpacked rootfs had no proper writable
  `/dev/null`; corrected standard device nodes. Did not disable apt signatures.
- One initial catalog invocation lacked cargo on PATH; repeated successfully with
  the correct toolchain environment. It is not counted as a passing first attempt.

## Scope still not completed

Only one product runtime exists. The 32 plans do not have runtime tests or releases.
Runtime library packaging does not install optional Rust compilers/analyzer/project
crates. Signed releases/update channels, all audit gaps, cross-distro/ARM64 coverage
and physical target desktop acceptance remain open.

## First hosted distribution iteration

Studio run [39483386307](https://github.com/drainedgodw/kedr/actions/runs/39483386307)
passed Windows Rust workspace and live analyzer tests, including the previously
failing recovery deletion. It then exposed a Windows-specific test assumption:
the mocked Debian scanner command assertion expected a Unix `/` absolute path.
The assertion now checks the exact platform-native resolved argument. No native
Debian scanner is executed on Windows by that contract test.

Dependency-source inspection also showed AccessKit's UIAutomationCore calls. The
Windows OS-DLL allowlist now explicitly recognizes UIAutomationCore, Oleacc and
MSCTF; VC redistributable/vendor DLLs are still refused, including delay imports.
One extra regression covers those system libraries. Fedora test equipment now
explicitly installs shadow-utils/util-linux for its non-root rendering harness.
New hosted results remain separate; do not label the first failed run green.

## Native hosted follow-up and Fedora dependency correction

Studio follow-up [39483947331](https://github.com/drainedgodw/kedr/actions/runs/39483947331)
observed completed successful macOS, Ubuntu and MSRV jobs. Windows had reached
artifact upload after passing its native checks; Arch was still building at that
observation. These are native hosted results, not Linux parser simulations.

The first distribution run [39483386169](https://github.com/drainedgodw/kedr/actions/runs/39483386169)
built both packages and passed clean Ubuntu installation. Fedora clean installation
correctly failed: `nothing provides wayland-libs-client`. The RPM now requests the
64-bit `libwayland-client.so.0()(64bit)` capability rather than a nonexistent package
name, letting DNF resolve the real distribution provider. A regression checks the
capability and rejects the old name. No `--nodeps`, `--skip-broken` or dependency
verification bypass was added. New Fedora clean-install acceptance is still required.
