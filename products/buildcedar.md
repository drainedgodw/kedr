# BuildCedar

![BuildCedar logo](../assets/brand/buildcedar.svg)

Continuous integration service. Part of the Kedr family; comparison category: TeamCity.

**Status: planned. There is no BuildCedar application, build artifact or installer yet.**
The name and logo identify a product direction, not delivered feature parity.

## Download

| Platform | Availability |
| --- | --- |
| Linux / Arch | Not released; no installation command yet |
| Windows | Not released; no installer yet |
| macOS | Not released; future CI is not physical-machine acceptance |

Do not download CodeCedar and treat it as BuildCedar. Actual release links and verified
platform-specific commands will be added only after a working build exists.

## Intended capabilities — not implemented

- [ ] Build agents, queues and isolation
- [ ] Pipeline configuration and secrets boundaries
- [ ] Artifacts, logs and build history

## Shared interface and shortcuts

Desktop products reuse the Kedr workbench where appropriate; services use the
same visual/interaction contract but need their own service and web/runtime host.
No service is claimed to exist merely because the desktop shell exists.
[Shared UI contract](../docs/studio/DESIGN.md) ·
[Implemented keyboard profiles](../docs/suite/KEYMAP.md) ·
[Parity release gates](../docs/suite/PARITY.md)

## Development

There is no buildable product package for this entry yet. All future code belongs
in this monorepo. Implementation needs a reviewed specification, actual backend,
integration tests, security boundaries and platform validation.

[Back to all Kedr products](../README.md#product-catalog)
