# Facet

The native clients live in `native/`: Rust/eframe with native BlueZ on Linux, and Kotlin/Compose with Android GATT on Android. Neither UI uses a browser or WebView. The existing web client remains available, with its history unchanged.

## Run and install

```sh
nix develop path:./native/env --command cargo build --release --manifest-path native/linux/Cargo.toml
nix develop path:./native/env --command bash -c 'cd native; ./gradlew :androidApp:assembleRelease'
nix develop path:./native/env --command node scripts/install-native.mjs
cstimer-native
```

The application menu entry is **csTimer Native**. The optimized, locally signed APK is copied to `.data/native-release/cstimer-android.apk`. It uses the development signing key for local installation, not a Play Store production key. Debug-only preview activities and synthetic records are excluded from that release APK.

Linux data belongs to `~/.config/cstimer-native`. Android storage belongs to `org.cstimer.nativeapp` and uses Android Keystore-backed encryption. Settings → Import backup explicitly imports native or supported 3×3 csTimer JSON sessions; no browser profile, older Cube Lab database, credentials or user history is automatically copied. Imported csTimer rows retain their original source, comments, penalties, timing and tags. Network synchronization is optional and requires an explicitly imported server connection.

## UI reference and rendering

`native/assets/design-contract.json` records the approved web interface's measured logical bounds, palette and typography at desktop, laptop and phone sizes. Native logical pixels correspond to CSS pixels at the reference scale. Geist Regular/Medium/SemiBold files are identical to the web fonts. Maple Regular/SemiBold are lossless sfnt conversions of the same licensed WOFF2 subset. SVG contours, molded caps, cubie geometry, palette, camera, GAN mark and toon-lighting formulas come from the approved web source.

The cube uploads an indexed scene once, transforms turns in the vertex shader, and uses one geometry draw (~49,302 triangles). Linux resolves native-resolution MSAA to a cached texture and only redraws geometry when the cube changes. Android renders on a dedicated graphics thread, coalesces notifications, and does no idle rendering. Both use bounded turn queues and retain the GAN mark's turn orientation. Timing and Bluetooth processing never depend on animation completion. Linux session rows are virtualized and the session snapshot is cached by revision; Android uses lazy rows and remembered calculations.

Timing, raw event recovery, CFOP detection and drills reuse the earlier native device/recording core, with csTimer's headless solver and review verification code retained for local cross, recovery and algorithm tools. Android uses the original author's JVM min2phase implementation for fast cold-start random-state scrambles; its review interpreter is loaded only when needed. Solver and review work stays off UI and Bluetooth threads. Alternatives are verified on the reconstructed case and labeled as experiments, never predicted personal speed improvements.

Exact font files, measurements and colors are shared. Browser, egui and Android text rasterizers may produce slightly different edge coverage. Screenshot/layout checks are evidence for the tested sizes and scales, not a claim of pixel identity on every compositor, font scale or GPU.

## Verification

```sh
nix develop path:./native/env --command cargo test --manifest-path native/linux/Cargo.toml
nix develop path:./native/env --command bash -c 'cd native; ./gradlew :shared:desktopTest :androidApp:assembleDebug :androidApp:assembleRelease'
node scripts/capture-native-reference.mjs
nix develop path:./native/env --command env NATIVE_CAPTURE_ALL=1 NATIVE_CAPTURE_WORKFLOWS=1 node scripts/capture-native-desktop.mjs
# Isolated emulator only; never attaches a physical phone or reads its storage.
node scripts/capture-native-android.mjs
node scripts/verify-native-parity.mjs
```

Evidence is stored in ignored `.data/native-parity/`. Rust screenshots capture actual eframe/Glow windows; Android screenshots capture the actual APK in an isolated emulator. Tests exercise sensor timing/recovery, goal settling, CFOP boundaries, HTM, commuting scrambles, import preservation, random-state generation, verified variants, tags, replay, release startup and Bluetooth-off feedback. Physical GAN16 pairing, real radio latency, live gyro telemetry and performance on a physical Android device remain hardware validation work.

## Sources and licenses

csTimer retains its GPLv3 notices. Existing native protocol/analysis sources were copied from the separate local Cube Lab checkout without changing that repository or its database. GAN protocol attribution, Tao Yu / Alg-Trainer's MIT license, all font OFL files and GANCUBE mark attribution are retained. JVM solver source and pinned revision are documented in [native/solver/UPSTREAM.md](native/solver/UPSTREAM.md), from [Shuang Chen's min2phase](https://github.com/cs0x7f/min2phase).

## Current native interactions

The desktop header's **Companion** action enables the native GNOME Shell top-bar item through the Shell extension service. The optional **Floating companion** desktop action, or `facet --floating-companion`, opens a detached 210 × 58 native window sharing the main application's Bluetooth connection. It stays above other windows and retains the existing `cstimer-companion` Forge floating rule. The companion shows the latest solve (independent of history selection), TPS and HTM move count, with live metrics during a solve. Double turns count as one move. The scramble fits two compact rows. `facet-companion` and `facet --companion` request the GNOME panel item from the running Rust app. A floating window is only requested explicitly with `--floating-companion`. Both reuse the running app and Bluetooth connection.

The installer also supplies `cstimer-companion@local`, a GNOME Shell 50 extension. It uses spare space on the left of the top bar without displacing the centered clock, showing a connection dot, time, TPS, moves and the remaining scramble. When space is tight, the scramble contracts; the dropdown retains the full scramble and metrics plus actions to open the timer or floating companion. The main Rust app must stay running; it can be minimized. Session D-Bus transfers only the displayed metrics and scramble, with no additional Bluetooth connection. A newly installed extension requires one logout/login before GNOME discovers it. Enable it with `gnome-extensions enable cstimer-companion@local`; disable it with the corresponding `disable` command. Nix home configuration retains its enabled state.

`scripts/test-native-panel.sh` validates the actual extension in a private headless GNOME 50 session using synthetic data, including action forwarding and disconnection when the Rust service closes. It does not restart or modify the live Shell session.

Cross shows its execution duration alone: planning belongs to inspection. The remaining CFOP stages show pause + execution. Both clients blur completed scramble tokens and share the same stickerless cube palette and turquoise/blue/lemon/lilac phase highlights.

The offline MIT-licensed Tao Yu catalog includes **41 F2L / 160 variants, 57 OLL / 227 variants, 21 PLL / 58 variants**, plus six cross study setups. All stored training solutions are checked in all six cross frames. Cross practice uses generated scrambles and verified solver routes rather than claiming a finite exhaustive cross algorithm set. Solve review displays all verified available OLL/PLL alternatives, with no three-result truncation.

New Bluetooth recordings retain gyro observations separately as `rotations`: snapped x/y/z pose deltas, local timestamps and quaternions. The renderer and recorder use the same 24-pose snapper with eight-degree hysteresis. Reconstruction interleaves these observations with face-turn entries, marking gyro timestamps approximate because the protocol has no corresponding device timestamp. Multi-axis gaps are shortest pose-delta descriptions, not reconstructed hand paths. Raw sensor face labels retain their fixed color frame; copied executable notation excludes gyro rows. Rotations never affect HTM, TPS or CFOP recognition. Older solves cannot gain missing gyro data retroactively.

## Tiled windows and current colors

Narrow desktop windows use a focused timer instead of retaining two fixed sidebars. Sidebars require at least 1180 logical pixels of width and 580 pixels of workspace height. History remains in navigation and tools open from CFOP. Short windows use a compact CFOP dock; very short windows open CFOP detail separately. Averages open on demand when their full measurements would collide with the dock. The clock and cube remain centered. The desktop minimum is 360 × 400, with the same geometry rules used in Kotlin for small windows and landscape layouts.

Cube albedos now use cool white, coral red, emerald, lemon, tangerine and sky blue. The toon ramp stays matte and rounded, with slightly brighter diffuse lighting. CFOP uses turquoise, cornflower blue, lemon and lilac, with soft blue pause values; existing history histogram colors are unchanged.

Capture actual resizes using `NATIVE_CAPTURE_ALL=1 NATIVE_CAPTURE_TILES=1 NATIVE_CAPTURE_WORKFLOWS=1`, then run `node scripts/verify-native-responsive.mjs`. This checks 16 resized native windows, their rendered cubes, containment, separated timer regions and cached geometry. The standard large desktop and phone layouts retain their approved composition.

Short history windows keep the scorebook scrollable by moving the distribution plot into the **Distribution** action; at the smallest heights, session summary statistics use that panel too. Narrow history columns reserve space for favorite/tag actions. The compact Bluetooth control is a single click target and is covered by an input regression test.

## Replay, best times and automatic cross review

Replay now orients the detected cross to the bottom, using the saved analysis rather than the current session setting. The notation view uses that same frame; Raw mode keeps fixed-color sensor labels. Seeking and orientation only affect presentation, never raw facelets, moves or timing.

Session-best adjusted times (including ties) are highlighted in green in the timer, history and pace plot. Penalties and DNFs retain the normal eligibility rules. Histogram bin count responds to available width and sample count, with contiguous bars. Sparse runs are merged into wider numeric intervals; samples, counts and summary statistics are preserved.

Tools automatically verify white/yellow cross and Xcross routes for the current scramble. The lower Cross/Algs/Technique tabs belong to the latest solve in the session, independently of the solve selected in history. Last-solve cross review starts from its recorded initial state and compares move counts on the detected cross face. No time savings are predicted. Results are computed off the UI thread and cached in a bounded 16-entry queue.

The native bridge now parses scrambles with `cubeutil.parseScramble(..., 'FRUBLD')` before calling the upstream cross solver. Passing text directly previously produced incorrect empty routes. Every new suggested route is replayed and checked for the intended cross; Xcross additionally verifies a solved corner/edge pair. Historical zero-move cached cross solutions are hidden when that cross was not initially solved.

## Interface consistency

Settings use a bounded content column, aligned fields, 14px section headings and compact theme choices. Bluetooth addresses, server credentials and attribution stay in expandable sections; Save changes applies to the whole form. Shared secondary pages use an 18px title and 22px primary measurements. Existing Paper/Carbon colors, centered timer, fonts and stored preferences are retained.

History summaries use the same time/TPS/moves order for Best, Mean, ao5 and ao12, with best time in green. Narrow, tall windows use two columns; short tiles retain four compact columns and move distribution into its existing action. Mean TPS and moves exclude DNFs like mean time; manual results do not receive invented sensor metrics. The desktop rail uses separate labels and numeric styling for Best and Mean. History rows are clipped to their scroll area above the footer.

Capture the settings/history workflows using `NATIVE_CAPTURE_CLEANUP=1 node scripts/capture-native-desktop.mjs` inside the native environment. `scripts/capture-native-cleanup-android.mjs` captures equivalent native Compose screens in the isolated debug emulator.

The Rust timer clears its temporary history selection whenever the current result ID changes, before rendering the clock and metrics. New Bluetooth and manual results therefore replace historical values together; metadata updates to the same result do not interrupt deliberate history review.

The left session rail labels Mean once in the Best/Mean summary. Its pace chart retains the dashed session-mean reference without repeating the label; full History keeps the chart label. This behavior is shared by Rust and Kotlin.

## Launcher icon

`native/assets/app-icon/` contains the original vector cube mark and desktop PNG exports. `node scripts/generate-app-icon.mjs` produces desktop SVG/PNG and Android vector foregrounds from the same geometry. Linux installs SVG plus 32–512px raster sizes; native windows share a cached bundled 256px icon. Android uses an adaptive foreground within its safe region and a separate monochrome layer on Android 13+. No external brand logo or font is used in this icon.

## Facet branding

Facet is the visible name of the native app, its desktop entries and companion, Android launcher, and GNOME panel extension. `facet` and `facet-companion` are friendly launch commands. Existing application IDs, storage locations, session-bus names and compatibility launchers are retained so upgrades keep history, settings, favorites and Forge rules. Upstream csTimer attribution and backup-format names remain accurate. The current mark is the three-facet F in `native/assets/app-icon/facet.svg`; the icon generator exports desktop PNGs and Android adaptive/monochrome layers from those paths.

## Row averages and compact labels

The timer rail and full History include ao5 next to each solve time, using that row and its preceding four chronological session solves. The first four rows show a dash. Best and worst are trimmed, +2 penalties apply, one DNF may be trimmed, and two DNFs produce DNF. Filtering never changes the averaging window. Rust caches the results with the session revision; Kotlin computes them once per changed record list. Time, TPS and move values retain their existing meanings.

The puzzle label and CFOP pause/execution legend are removed; the colored split notation remains. History filters, tags and search share a 36px outer height, with vertically centered text. The desktop rails are 256–272px to accommodate the new column. GNOME companion activation is asynchronous and uses the Shell's EnableExtension method; a newly installed extension gives a one-time logout/login instruction rather than opening a floating window.

## Reconstruction steps and case alternatives

Space toggles play/pause while a review is open; repeated key-down events are consumed without repeated toggles. Pausing retains the playhead between turns, and replay restarts from the initial state at the end. Seeking a turn or stage updates the playhead. Android handles external-keyboard Space inside the review dialog.

Reconstruction groups original event spans under Cross, F2L, OLL and PLL, retaining elapsed timestamps, inferred slices and snapped gyro rows. Group boundaries use raw event counts rather than timestamp comparisons, including equal timestamps. Missing reliable boundaries fall back to Recorded.

Expandable case rows show verified cross/Xcross solutions, F2L pair cases, OLL and PLL cases, and AUF-only finishes. F2L checkpoints use the last stable insertion of each pair, and every offered variant must preserve the cross and already solved pairs. Classification is withheld when no pattern can be verified. Alternatives are displayed and copied in the replay's detected cross-down frame; original algorithms remain available in solver metadata. No time savings are inferred from move counts.

The shared verifier now explicitly copies cubie arrays with `CubieCube.init`: the upstream constructor takes no arguments. F2L algorithms normalize lowercase wide-move notation before verification and HTM counting. Independent geometric tests replay offered algorithms and validate their goals in all six cross frames.

The left rail shares the tools rail's 16px side inset; responsive screenshot checks assert both margins.

Reconstruction alternatives retain the selected catalog variant index when opening Practice. A single loading indicator is used while case verification runs on the bounded solver worker. Screenshot capture waits for verification to finish; screenshots and interaction checks use isolated synthetic records.

## Scramble completion and case analysis

Scramble TPS is cached at the final recorded turn when the target is matched. Inspection and cue waiting do not extend its denominator or rerun the move counter. A new scrambling turn invalidates the snapshot; a new target resets it. Device timestamps remain authoritative, including half-turn move counting.

The current scramble token uses an accent-filled container; a partial half turn uses an amber half-filled container. Completed tokens remain blurred. The floating widget and Android use the same distinction, and the native GNOME panel uses blue/amber highlights.

Each case detail has Algorithms and Analysis tabs, including every F2L, OLL and PLL case. Analysis includes weighted case-level recognition/execution means, sample coverage, execution trend and per-algorithm usage. Existing synchronized analytics are retained. When those are absent, precisely timed local drills provide an offline baseline; matching is based on observed turns rather than the selected algorithm label. DNFs, manual entries and imprecise execution endpoints are excluded. Full-solve transitions are not presented as cue recognition.

New recordings save `rotationFrame: live-v1`, their actual `initialOrientation`,
and snapped gyro poses in the same calibrated frame as the live cube. Replay
uses that frame directly, without adding a cross-down rotation. Legacy recordings
keep their relative, cross-down view because they lack the live calibration.
Playback, pause and backward seeking use absolute poses rather than live gyro
calibration or accumulated deltas. Rotation rows seek to their observation time;
rotations never add HTM, TPS or CFOP turns. Legacy solves without gyro events
retain the cross-down view. Gyro timestamps remain approximate local observations.

Solve deletion: desktop session/history rows offer Review/Delete on right-click;
the solve-review header also exposes Delete. Android offers the same actions
from the history row's More menu or a long-press (including the session rail),
and a trash button in review. Every path asks for confirmation. Deletion removes
the saved solve and pending upload, refreshes statistics, and clears a deleted
latest result. A configured server must acknowledge deletion before local removal;
Android serializes deletion with sync to prevent stale uploads restoring a solve.

Facet background service (GNOME/Linux): the installer enables the user unit
`facet.service` on `graphical-session.target`. `facet` opens the existing timer;
`facet-companion` activates its GNOME panel item. The service starts with the
main window hidden. Closing the window hides it while the same process keeps Bluetooth tracking, timing, recording and
panel snapshots alive. Hidden mode
skips dashboard rendering. `facet --hide` hides the window explicitly;
`systemctl --user stop facet.service` exits through the normal shutdown path.
It requires a graphical login, and does not enable system-wide lingering.

A newly installed local GNOME extension may not exist in an already-running
Shell's extension registry. GNOME 50's ReloadExtension API is deprecated and
cannot discover it. The installer preserves other extensions and enables the
Facet UUID for the next login. In this situation one sign-out/sign-in is needed;
the installer never restarts Shell, logs out the user, or enables unsafe mode.

To stop Facet completely, use Settings → Quit Facet, Ctrl+Q, the GNOME
companion's Quit menu item, `facet --quit`, or
`systemctl --user stop facet.service`. The in-app header ×, system window close, and Alt+F4 hide the main window and keep the companion running.
Use Keep in background (Settings or the panel) to hide it explicitly. Clean Quit exits normally and is not restarted by
systemd's on-failure policy. To turn off login startup too, run
`systemctl --user disable --now facet.service`.

Solve review uses the same compact composition in Rust and Compose: a metric
header, persistent cube/playhead with four CFOP rows, and Moves / Analysis tabs. The scramble is directly visible in Analysis. Move chips retain elapsed timestamps and seek to recorded events,
including gyro poses. Verified case alternatives stay next to their CFOP group.
Analysis shows the largest phase, longest measured gap, and colored F2L pair windows
with entry gap, execution, HTM, TPS, replay and state-verified alternatives.
Pair boundaries ignore temporary displaced slots while an insertion opens the cross.
Entry gaps are not labeled as recognition time. Secondary timing explanations and existing notes stay collapsed; manual solves
never gain invented TPS or turn counts. Small desktop windows can scroll the
whole review as a fallback while ordinary layouts keep the player visible.

Miscramble guidance uses the normal filled cursor on the first outstanding
recovery move. Later recovery moves remain red; the original scramble's next
and partial highlights pause until recovery finishes. Completed moves remain
blurred. This presentation is shared by the desktop timer, floating companion,
GNOME snapshot and Android; face-turn recovery and timing rules are unchanged.

Replay metadata survives cache/sync round trips even when an older API omits
optional gyro fields, provided the record ID, starting facelets, duration and
raw move/timestamp sequence still match. Complete remote pose frames and changes
to the underlying solve take precedence. No existing solve or raw turn is rewritten.

Reconstruction includes previous/next turn buttons and Left/Right keyboard shortcuts.
Notation mode steps through complete grouped turns (including inferred M/E/S and
half turns); Raw mode steps through sensor events. Gyro rows remain individual
steps, preserving their timestamps. Stepping pauses playback and works in reverse.
Slice grouping uses bounded adjacent packet gaps (100 ms), rather than a 100 ms
limit on a complete four-packet double slice. Inferred slices stay marked; recovered
packets, missing moves and clock resets retain the conservative raw fallback.

The scramble cursor highlights only the first outstanding token, even when an
opposite face can be turned early. A completed opposite-face token dims immediately
without moving the cursor; partial styling belongs only to the cursor token.

CFOP stage times use a bold total above a smaller right-aligned gap + execution
line; Cross has no gap line. The same hierarchy is shared by the timer dock,
solve review and F2L pair cards. Move/TPS metrics remain secondary.

Session charts offer Time / TPS / Moves in the rail, History trend, histogram and
compact statistics dialog. Changing the metric updates the mean reference, best
markers, bins and distribution statistics together. TPS uses execution time,
without +2 penalties; moves use the existing HTM counter. DNFs and unavailable
metrics leave empty trend slots and do not enter the mean or histogram. No
manual-entry TPS or move counts are synthesized. Move bins use whole-move intervals.

The GNOME companion uses two stable scramble rows, retaining and dimming early
opposite-face turns. Its stacked time/TPS/moves block reserves compact panel
space against other items' minimum widths, so long focused-window titles
ellipsize instead of hiding the companion. A main-thread event-loop pump drains
cube events and publishes snapshots independently of Wayland redraw callbacks.
Panel reads do not repaint the dashboard; hidden/covered views discard obsolete
visual turn queues. `scripts/test-native-background-wayland.sh` checks fresh
snapshots under another focused fullscreen app and background show/hide/quit.
GNOME Shell 50 requires a new session to load changes to an already imported
extension module; installing files does not replace the live JavaScript class.

Wayland presentation uses a nonblocking EGL swap interval: a covered surface
must not stall the single model/event-loop thread. Winit's frame scheduling
and demand-driven egui repaints continue to control actual rendering.

The GNOME companion places two scramble rows first and a right-aligned time /
TPS · moves block last. Adwaita Mono notation and Adwaita Sans measurements
match the Shell. Each token keeps the same padded slot: current moves have a
muted blue background, partial turns amber, and finished moves dim. Width follows
actual notation, with a 10px internal gutter and only 4px outer padding; there
is no fixed-width empty tail or outer margin. Measurements use tabular digits.
Offline mode reduces to the status dot; connection controls remain in its menu.

The GNOME companion menu can reconnect the saved cube, scan/select a nearby
cube, cancel a connection attempt, disconnect, and turn Bluetooth on. Commands
are queued to the existing Rust model; the extension never opens its own BLE
connection. Discovery IDs are opaque and resolved against current discovery or
the saved cube, with no MAC address in the panel snapshot. If the service has
been quit, Connect cube starts facet.service in the background first. Connection
controls need both the updated Rust binary and the updated Shell extension.

The private Wayland test also sends real compositor window-close requests,
reopens the timer twice, and starts/stops keyboard timing after reopening.
These checks do not validate physical cube interaction.

GNOME companion rows use a matching 14px pitch (28px stacks) and integer
notation sizes. Active/partial cursors retain regular font metrics instead of
switching font faces. A green check replaces the connection dot only when every
scramble token is complete, the cube is connected, and timing has not started;
partial, recovery, running, stale, and disconnected states clear it. This adds
no text or width to the panel. The Shell workflow test checks row positions and
Pango baselines through partial/second-row cursors and completion transitions.
