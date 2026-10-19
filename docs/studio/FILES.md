# External file changes and explicit reload

This is active-file reconciliation, not crash recovery, a merge tool or a complete
filesystem watcher. It does not run project code and does not write project files.

## User workflow

The normal desktop host queues a content check of the active file approximately
every two seconds when its bounded background queue is available. At most one
probe is outstanding. Checks read up to the existing 16 MiB document limit on a
worker, comparing actual UTF-8 contents with the saved baseline rather than just
a timestamp or size. Dirty text is not used as the disk baseline.

A changed, missing or unreadable file gets an inline notice above the editor.
Editing can continue. No automatic reload, saving, deletion or overwrite occurs.
The File menu and command palette also expose **Reload file from disk**; no
preset shortcut is reserved, but the action is remappable in keyboard settings.

Reload always asks for confirmation. For a dirty file it explicitly says that
unsaved edits will be discarded. Reload also resets that file's undo history;
Cancel/Escape keeps it. An incidental Enter with no selected confirmation button
is not a default destructive acceptance. There is no automatic "keep mine"
overwrite button. Copy conflicting edits elsewhere before discarding them.

After confirmation the read happens off the render thread. A successful current
result replaces only that active tab, preserves the caret's Unicode-scalar
line/column within the new bounds, clears selection and uses the new newline
style/saved baseline. Other dirty tabs remain unchanged. Language/navigation
pending results for old contents are cancelled/cleared. Subsequent save still
checks for additional external writes.

A failed read leaves the editor, dirty contents and undo unchanged; a missing file
is not recreated. A result for an edited/undone/saved/reopened tab, another active
file, or another workspace is discarded. Confirmation itself carries the same
identity/revision/baseline guard, not a tab index that might point at another file.
Each request also has a distinct process-local ID: a cancelled completion cannot
complete a later identical retry. Beginning a workspace switch abandons pending
reload state even if scanning that workspace fails. This invalidates results, not
the underlying blocking OS read.

## Boundaries

- Only the active file is polled. Inactive files and the project tree are not
  watched/refreshed; creating, deleting or renaming project entries still requires
  reopening/rescanning the workspace. A tab switched away and back with identical
  identity/revision/baseline may accept its still-applicable read.
- Regular canonical local files only, at most 16 MiB and valid UTF-8; BOM and
  newlines are preserved. Before/after identity checks reject escaping/replaced
  symlinks. They are not an OS capability sandbox or a lock against concurrent
  filesystem mutation between checks.
- Probe/reload reads share the bounded file/search worker. Slow local/network
  filesystems can delay queued work; these blocking OS reads have no hard timeout
  or process-level cancellation. At most one probe is queued, not an unbounded
  per-frame thread or watcher event stream.
- A loaded result is a snapshot. An outside writer can change disk after reading
  and before display. Later polling/save guards detect further observed changes;
  no cross-process synchronization guarantee is made. Polling is not a semantic
  LSP workspace snapshot guard.
- Saves are still synchronous and Save all can partially succeed. Full durable crash recovery (beyond the [bounded draft journal](RECOVERY.md)),
  conflict comparison/merge, Save As, locking and cross-file transactional rollback
  remain future correctness gates.
- Capture mode does not poll/reload or change project files. Disk-change and
  confirmation scenes inject clearly identified layout fixtures, not actual disk
  mutation evidence. Automated worker tests perform real temporary-file changes.
