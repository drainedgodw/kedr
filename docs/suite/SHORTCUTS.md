# Custom shortcut contract

CodeCedar supports one single-stroke chord per implemented action. This does not
add completion, debugging, refactoring or any unsupported command.

## Use

1. Settings → Keyboard shortcuts; choose the compatible or classic profile.
2. Search by action name, stable ID or current shortcut; choose **Edit**.
3. Type `Ctrl+Alt+L` (`Cmd+Alt+L` on macOS), or choose **Record keys** and press
   the combination. Recording changes only the draft; **Apply** commits it.
4. **Unbind** removes the shortcut while leaving the action in menus/palette.
5. **Use preset** restores that action; **Reset this profile** clears all custom
   bindings for the active platform/profile. Other profiles are preserved.

Escape cancels recording first, then returns from editing to the shortcut list.
A conflict identifies both actions and does not change either assignment. Move or
unbind the conflicting action first. Restoring a preset can also conflict with
an override; reset the whole profile if you want all original assignments back.

## Safety and persistence

- Plain typing, Shift-only character/navigation keys, Escape, clipboard keys and
  common system combinations are not assignable. This protects the editor, but
  is not an exhaustive list of every desktop/window-manager reservation.
- OS/global shortcuts may never reach the app. Arch/Wayland users must check
  their compositor bindings. Logical keys are used, not physical scan codes.
- Custom editor actions preserve editor-focus guards; palettes/modals and search
  fields cannot accidentally edit the active file through those bindings.
- Preset and remapped commands require exact modifiers. No less-specific binding
  silently intercepts a more-specific one.
- Storage uses stable `profile:platform:action` identifiers in the existing local
  preferences file. `mac` and `other` (Windows/Linux) are separate namespaces.
  Empty values mean unbound; omitted values mean preset. Older preferences load
  with no overrides. Values are bounded to 96 bytes and 104 possible entries.
- Invalid/conflicting imported settings restore safe presets with a visible
  warning. Shortcut validation is separate from preference-file I/O; the existing
  persistence path can still report disk/permission errors.
- Dispatch uses a resolved binding cache, rebuilt only when configuration changes.
  Menus, palette hints and settings read the same cache.

Not implemented: multi-stroke chords, double-Shift, arbitrary context hierarchies,
multiple alternative chords per action, portable keymap-file import/export,
per-project maps or universal keyboard/IME/assistive-technology acceptance.
