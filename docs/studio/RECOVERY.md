# Local draft recovery — first bounded implementation

CodeCedar now checkpoints unsaved editor text in a **separate local plaintext
journal**, approximately every two seconds while the desktop render loop is
running and the dedicated bounded worker can accept work. It never autosaves
project files and does not execute project code. Disable journal discovery and
writes for a session with `kedr-studio --no-recovery /path/to/workspace`.
Capture mode never starts the journal worker, discovers previous snapshots or
writes recovery state. The appearance dialog discloses local plaintext storage.

## Storage and privacy

- Linux: `$XDG_STATE_HOME/Kedr/recovery-v1`, or
  `$HOME/.local/state/Kedr/recovery-v1`.
- macOS: `$HOME/Library/Application Support/Kedr/recovery-v1`.
- Windows: `%LOCALAPPDATA%\Kedr\recovery-v1`.

The base path must be absolute. Each process has a distinct session directory,
so concurrent application instances do not share a writable journal. On Unix,
session directories use 0700 and snapshot files 0600; Windows relies on inherited
user-state-directory ACLs. This is **not encryption**. Snapshots contain source
text, original saved baselines and absolute paths, which may include secrets.
Do not use recovery on an untrusted/shared account. Old interrupted-session
snapshots are retained until explicitly deleted; there is no time-based expiry
or secure-erasure guarantee. The data is not uploaded by the application.

## Restore and delete decisions

At startup a background scan finds prior session snapshots. A compact banner
opens **Review recovery**. Open the matching workspace (a button is provided when
necessary), then choose **Restore drafts**. Restoration is explicit, not an Enter
default or an automatic startup operation. Later/Escape keeps the source snapshot.

Only editor buffers are restored. All target paths, quotas and existing tabs are
validated before any buffer is changed. New/currently clean tabs may receive a
recovered draft; a current dirty target, changed workspace/tab version, saved
baseline, reopened tab, workspace opening in progress or 32-tab overflow rejects
the entire result. Starting another workspace open or approved quit cancels the
pending restore acceptance, not the underlying blocking read. Unrelated dirty
tabs are preserved. Restored caret offsets
are clamped to UTF-8 boundaries; selections and prior undo history are not restored.
Original saved baselines remain attached: a later Save refuses externally changed
or missing disk contents. A missing file can be recovered in memory, but is not
recreated by Save. Copy the recovered text elsewhere when necessary; Save As and
conflict diff/merge are not implemented yet.

Restoring deliberately retains the original snapshot. **Delete snapshot…** asks
for a separate confirmation and removes the selected backup plus retired older
numbered revisions in that session, never a project file or current editor buffer.
Older revisions are removed first; failures retain the selected revision and report
an error. This prevents an old revision left by failed cleanup from reappearing.
Temporary and unrelated files are not removed. Another instance may still be running;
its independent newer revisions are not removed. Offers are discovered at startup,
not continuously refreshed. A deleted/changed source reports an error on access.

An approved normal quit (including an explicit Discard changes decision) waits
for the journal worker to commit an empty revision before closing. Only that
process's own session is retired. Failure keeps the window open and reports the
error; a slow OS write can delay this wait. Existing source recovery offers are
not erased just because restored buffers were saved or the application quit.

## Commit protocol and limits

A worker creates an exclusive temporary file, writes and syncs it, then renames
it to a new immutable numbered JSON revision. Unix also syncs the session directory.
Only after the new revision commits are older owned revisions removed. Startup
uses the newest revision and ignores incomplete `.tmp` files; a corrupt newest
revision is reported and retained, **not silently rolled back** to potentially
obsolete edits. There is no cross-process writer lock or hostile-filesystem
sandbox. Canonical root/target checks reject observed symlink escapes/replaced
identities; filesystem mutation between checks remains a race boundary.

- At most 32 drafts per snapshot and 16 MiB of combined draft text plus saved
  baselines. Quota failure reports an error and keeps the last good snapshot;
  it does not silently save a partial set of drafts.
- JSON reads allow at most 100 MiB (escaping expands text), startup reads at most
  64 MiB of selected journal files and offers at most 32 snapshots. It bounds
  directory scanning to 1024 entries and 128 revisions per session. Exceeded
  limits report retained data rather than claiming complete discovery.
- Journal I/O uses its own one-slot request queue, separate from file/search work.
  Snapshot comparison/materialization still happens on the UI thread; large
  buffers can cause pauses. There are no hard OS-read/write timeouts. A stalled
  write can postpone future checkpoints and approved shutdown.
- Keystrokes since the last successful checkpoint can be lost. A process killed
  before its first checkpoint has nothing to recover. This is not per-keystroke
  journaling, a full editor/session restore, encrypted storage, power-loss proof
  persistence or a substitute for version control/backups.
- Crash durability of new directory creation, storage hardware, Windows inherited
  ACLs, physical-machine behavior and long-running sessions need further testing.
  Synchronous project saves and nontransactional Save all are unchanged.

Model tests, actual worker lifecycle tests and a child process killed after a
committed revision establish the implemented subset. Native recovery captures are
visibly labelled **layout fixtures**, not evidence of a real GUI crash/restore.
See [validation](VALIDATION.md) and the prioritized [audit](../suite/AUDIT.md).
