# admin-stdout-views

Lightning Admin overlay canvases whose wells show allowlisted host
stdout. Folded from `add-admin-stdout-views` (2026-10-30).

Code: `admin/src-tauri/src/canvas.rs`, `admin/src-tauri/src/stdout.rs`,
`admin/src/lib/canvas/`.

## Requirements

### Requirement: Owner-drawn canvas overlay

Lightning Admin SHALL create an undecorated, transparent overlay window
for a stdout view. The window SHALL have no OS title bar, no traffic
lights, and no system window shadow. Its label SHALL match `canvas-*`.
Chrome SHALL be a tiny close control, four cyan corner ticks, a
frosted-glass fill, a 45° chiseled octagon outer silhouette, and an
angled-glass bevel around the rim. Close and Escape SHALL destroy that
window. The existing decorated main window SHALL remain the discovery,
network-name, and guild surface.

#### Scenario: Canvas has Papyrus chrome

- GIVEN Lightning Admin is running on macOS
- WHEN the operator opens a stdout-view canvas
- THEN a window exists with cyan corner ticks and an octagon silhouette,
  and it has no OS title bar

#### Scenario: Main window stays

- GIVEN a visible stdout-view canvas
- WHEN the operator looks at the Lightning Admin main window
- THEN discovery, network name, and guild controls are still there

#### Scenario: Close destroys the canvas

- GIVEN a visible stdout-view canvas
- WHEN the close control is activated or Escape is pressed
- THEN that canvas window is destroyed

### Requirement: Modifier move and resize

A pointer drag with no modifier SHALL NOT move or resize the canvas.
The close control SHALL NOT start a move or resize. Inner size SHALL
stay inside `[80, 4096]` logical pixels.

Gesture start is the modifier set at pointer-down:

- Option/Alt down, with or without Control, SHALL start a resize.
- Control down and Option/Alt up SHALL start a move.
- Shift alone SHALL NOT start a move or resize.

During an in-progress **move**, the outer position follows the pointer
and inner size is unchanged. Control may be released; the move continues
until pointer-up. Option/Alt down during a move SHALL NOT convert it
into a resize.

During an in-progress **resize**, live Option/Alt SHALL choose the
anchor: held, the window grows or shrinks about its center; released,
the top-left stays fixed and the bottom-right follows the pointer.
Live Shift SHALL keep the aspect ratio from pointer-down. Control
during a resize SHALL NOT convert it into a move.

#### Scenario: Unmodified drag does nothing

- GIVEN a visible canvas
- WHEN the user drags on the pane with no modifier
- THEN the window's position and size are unchanged

#### Scenario: Control-drag moves

- GIVEN a visible canvas
- WHEN the user Control-drags on the pane with Option/Alt up at pointer-down
- THEN the window's outer position follows the pointer and its inner
  size is unchanged

#### Scenario: Option at pointer-down starts a resize

- GIVEN a visible canvas
- WHEN the user pointer-downs with Option/Alt held and drags
- THEN the gesture is a resize, even if Control is also down

#### Scenario: Option released during resize uses the corner

- GIVEN an in-progress resize
- WHEN Option/Alt is released and the pointer keeps moving
- THEN the top-left stays fixed and the bottom-right follows the pointer

#### Scenario: Option held during resize uses the center

- GIVEN an in-progress resize
- WHEN Option/Alt stays held
- THEN the window grows or shrinks about its center

#### Scenario: Shift keeps aspect ratio

- GIVEN an in-progress resize
- WHEN Shift is held
- THEN width and height keep the aspect ratio from pointer-down

### Requirement: Allowlisted stdout well

The interior of a stdout-view canvas SHALL display one named source.
A source SHALL be a fixed argument vector from an allowlist. The host
SHALL spawn that vector without a shell. The well SHALL be a monospace
transcript of combined stdout and stderr, not HTML. Exit status SHALL
be visible on any non-zero exit. A missing binary, timeout, spawn
failure, or non-zero exit SHALL show as a failure in the well, not as
a fallback to `scan_link_local` or `if-addrs`.

Each fill SHALL be a finite one-shot: a timeout and a capture-byte cap
SHALL exist (values named by `add-host-ifconfig-view`). Capture SHALL
stop at the cap and the well SHALL show truncation. At most one child
process SHALL run for a canvas. A new fill SHALL cancel and reap any
in-flight child for that canvas. Destroying the canvas SHALL cancel
and reap its child. The well SHALL distinguish loading, successful
output (including empty), failure, and truncation. This landing SHALL
NOT require polling or a refresh control.

On a host that is not macOS, source `ifconfig` SHALL show unsupported
in the well and SHALL NOT spawn a process or another source.

#### Scenario: Allowlisted argv runs

- GIVEN source `ifconfig` is on the allowlist as `["/sbin/ifconfig"]`
  on macOS
- WHEN that source fills a canvas well
- THEN the process is spawned as that argv with no shell, and the well
  contains the combined stdout and stderr transcript

#### Scenario: Unknown source is refused

- GIVEN a name that is not on the allowlist
- WHEN a well is asked to fill from that name
- THEN no process is spawned and the well shows a refusal

#### Scenario: In-flight fill is cancelled

- GIVEN a canvas with a still-running source process
- WHEN a new fill starts or the canvas is destroyed
- THEN the previous child is terminated and reaped, and a destroyed
  canvas does not later show that child's output

#### Scenario: Timeout and cap are visible

- GIVEN a source that hangs or writes past the capture cap
- WHEN the fill ends
- THEN the well shows failure on timeout, or success with a truncation
  mark when the cap stopped capture

#### Scenario: Unsupported host

- GIVEN Lightning Admin on a host that is not macOS
- WHEN the default `ifconfig` source would fill a well
- THEN no process is spawned and the well shows unsupported

#### Scenario: Discovery is a different surface

- GIVEN a canvas well showing ifconfig stdout
- WHEN the operator uses Scan on the main window
- THEN `scan_link_local` still fills the neighbor list and does not
  replace the canvas transcript

### Requirement: First source is host ifconfig

On macOS the allowlist SHALL include a source named `ifconfig` whose
argv is `/sbin/ifconfig` with no extra arguments. Opening a stdout-view
canvas with no other source selected SHALL fill from `ifconfig`.

#### Scenario: Default well is ifconfig

- GIVEN Lightning Admin on macOS with `/sbin/ifconfig` present
- WHEN the operator opens a stdout-view canvas with the default source
- THEN the well contains the current `ifconfig` transcript

### Requirement: One canvas in this landing

This capability's first implementation SHALL provide one stdout-view
canvas. Opening another canvas, independent per-canvas sources, and a
spatial grid, diorama, or 3d layout SHALL wait on `add-crystal-grid`
(and `add-view-swap` for source switching). Fold of this change SHALL
not require more than one canvas.

#### Scenario: Default open is a single canvas

- GIVEN Lightning Admin on macOS
- WHEN the operator opens a stdout-view canvas with the default source
- THEN one canvas window exists and its well fills from `ifconfig`

### Requirement: ifconfig fill bounds

The `ifconfig` source SHALL spawn argv `["/sbin/ifconfig"]` with a **5 second**
timeout and a **65536 byte** capture cap. Combined stdout and stderr SHALL be
shown as text. Hitting the cap SHALL set truncated and SHALL NOT wait for the
child to exit beyond the timeout. Timeout SHALL be a failure. Non-macOS SHALL
be unsupported with no spawn.

#### Scenario: Default open fills ifconfig

- GIVEN Lightning Admin on macOS with `/sbin/ifconfig`
- WHEN the operator opens the stdout-view canvas
- THEN the well contains that process's combined transcript or a visible
  failure, and is not left empty without a state

#### Scenario: Cap truncates

- GIVEN a fill that writes more than 65536 bytes
- WHEN capture stops
- THEN the well is truncated and the truncation is visible

#### Scenario: Timeout fails

- GIVEN a child that does not exit within 5 seconds
- WHEN the timeout fires
- THEN the well shows failure and the child is killed and reaped

