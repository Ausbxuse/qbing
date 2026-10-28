# csTimer local fork

This repository starts from [cs0x7f/cstimer](https://github.com/cs0x7f/cstimer), retaining its Git history and GPL license. `upstream` points to that repository; local work is on `zhenyu/local`. It has not been published as a GitHub fork.

The timer, GAN Bluetooth protocol, timestamp correction, scorebook, reconstruction and training tools remain csTimer's implementation. The optional interface is live: Maple Mono, Light and Dark appearance modes, a centered clock and animated cube, compact history, and a four-stage CFOP dock. Controls and data share a 14px scale, labels use 12px and scrambles use 18px; notation stays the same size while scrambling. The cube uses GAN-inspired stickerless shells with narrow seams, pale recessed internals, rounded center caps and asymmetric inner cutouts on edge and corner pieces. Its surface profiles are a visual approximation from the official GAN16 ui product photograph, not manufacturer CAD. Face colors are visually matched to GAN16 ui and GAN12 stickerless product photographs: primary red, yellow-green, azure blue, orange, yellow and neutral white. They are photographic approximations, not manufacturer color specifications. Display-linear tone mapping avoids automatic highlight desaturation; the light balance and reduced specular washout were checked against sampled photo colors in an isolated renderer. Checks sample a front-facing patch, so they do not establish color accuracy under every view, light or display. Perspective, studio lighting and supersampling retain shape and clean edges at the small UI size. `local-cube-geometry.js` builds closed indexed caps with a genuinely curved face, rolled shoulder and recessed back. Normals are shared across the rings to keep reflections continuous. `local-cube-shaders.js` adds only procedural roughness variation in cap-local coordinates; it does not fake the cap curvature. Subpixel grain fades out at timer size. Filtered directional shadows add contact depth, and reflection cards describe the coating. The UV-style dielectric clearcoat reflects the studio cards across the curved caps. The GAN center mark is a local blue SVG adaptation of the manufacturer header mark; its source and retained brand rights are recorded in `src/assets/gan-center.svg`. The mark follows validated face turns. The logo spans about half the center width. Center caps use smooth rounded-square outlines without dimples. The renderer still redraws on changes rather than continuously while idle. Record editing and replay call upstream handlers. Average TPS/moves use the same retained solves as the time average; manual records never gain invented move data.

The browser app owns the Bluetooth connection. **Cube Companion** is a separate Rust/eframe desktop widget that reads its live local updates. It shows the scramble and highlighting, connection dot, latest single and TPS, with pin/open controls. It does not connect to the cube independently.

## Current interface revision

The user rejected the earlier palette-only iterations and the Signal study. The current revision changes the composition: a compact session rail, centered scramble/cube/time, and a horizontal CFOP dock. Phase detail opens on selection in a side inspector on desktop and a sheet on phone. The session selector lives in the session rail on desktop and returns to the header on phone. Averages stay below the clock on large screens and move to the side on narrow or short desktops. The four phases use two columns on smaller screens. This revision is implemented; it has not been approved by the user.

Options → Appearance offers Light (`paper`, the default) and Dark (`carbon`). Unsupported retired palette values migrate to Light without modifying solve history or comments. Maple Mono NF and the user's time size, weight and decimal preferences remain intact. Theme changes and timer sizing apply immediately.

The session trace covers the last 32 time results, including penalties. Click/Enter reviews a solve; arrows/Home/End inspect points. DNFs leave a gap. Average TPS/moves use the same retained, time-ranked windows as time averages, with sample coverage. Manual solves never gain invented sensor data.

Full History adds Best/Mean/ao5/ao12 summaries, each with eligible TPS/move metrics and sample coverage. Search matches solve numbers (`#4` is exact), formatted time, scramble, tags and preserved legacy notes. All/Moves/DNF/Favorites plus color/group filters change presentation only, preserving indices and upstream averages. Desktop includes reconstructed CFOP stage/pause times, timestamps and quick tags instead of a note column. Favorites are one-click stars; the tag menu adds one of four colors and a Review/Practice/Milestone group. Metadata lives in `record[5].localReview`, preserving comments and other metadata, and travels in upstream JSON export/import. Rows append in batches of 100. Arrow keys/Home/End navigate rows; Right opens details, T opens tags, Enter reviews the solve. Filters never reconstruct the entire history. Session changes reset search and filters.

The CFOP dock uses upstream reconstructed pause/execution durations and HTM counts. Its proportional strip and stage controls choose the corresponding tool. Pause time is not a separately measured recognition time, and phase times exclude penalties.

The renderer uses the requested cartoon treatment: rounded caps, matte face colors, charcoal seams and toon lighting. Studio reference assets and their original licenses are retained but are not used by the current renderer. Physical GAN16 Bluetooth operation remains untested. The native companion is unchanged.

The action menu remains anchored to its trigger, supports keyboard navigation and focus return, and disables unavailable actions. Classic mode remains available. Statistics, timed replay, scrambling and Bluetooth are upstream-authoritative.

Actual built-app screenshots are at **http://localhost:3328/installed-review.html**. The isolated study at **http://localhost:3328/focus-review.html** uses sample records and never opens Bluetooth or timer storage. `node scripts/capture-focus.mjs` captures 17 desktop/phone workflows. Browser integration tests use port 3330 with an isolated profile and cover responsive transitions after media-query layout has settled, visible cube pixels in saved screenshots, persistence, both appearance modes and retired-theme migration.

## Run

```sh
nix develop --command npm run build
npm start
```

Open **http://localhost:3327** in Chrome or Brave. `node scripts/install-local.mjs` adds **csTimer Local** to the Linux application menu and installs `~/.local/bin/cstimer-local`. That launcher starts the loopback server and opens a dedicated Brave app window with Web Bluetooth's experimental platform support enabled. The main interface uses csTimer in Brave; the companion widget is a native Rust application.

The launcher keeps its browser profile under `~/.local/share/cstimer-local/browser`. Settings and history use csTimer's browser storage, with its existing export/import controls. The older Cube Lab app and milkyway database remain unchanged; this fork does not yet synchronize with that database. A normal browser tab and the dedicated app window have separate profiles/history.

Open **Cube Companion** from the application menu or run `~/.local/bin/cstimer-companion`. Its default window is approximately 210×58; scrambles balance across two rows, with text scaled to fit longer sequences. Drag the scramble area to move it, use the pin to toggle always-on-top, and right-click the pin to close. The open button activates the dedicated main app. On Linux the widget uses X11/Xwayland when available so the pin works. The browser app must stay running for live Bluetooth updates; its launcher disables background timer throttling. Closing the timer clears the indicator while retaining the last solve.

To rebuild/install the companion:

```sh
nix develop --command cargo build --release --locked --manifest-path companion/Cargo.toml
nix develop --command node scripts/install-companion.mjs
```

The main launcher's dedicated profile exposes a loopback-only debugging port (3329 by default) for activating its existing window. The bridge on port 3327 accepts same-origin JSON updates, gives one timer ownership, and expires stale connection state. It is not a persistent database or a network synchronization service.

The current surface can be inspected at **http://localhost:3328/cube-surface.html**: drag to rotate, try R/U turns through upstream cubie math, reset the view, or switch the backdrop. Regenerate that isolated preview with `node scripts/preview-cube-surface.mjs`; it imports copies of the actual renderer and shaders, with no Bluetooth or timer storage.

The original design proposals remain at **http://localhost:3328** and **http://localhost:3328/widget.html** when `npm run preview:interface` is running. They use sample data. The production app at port 3327 and installed companion use actual upstream records and connection state.

## GAN16 UI

Wake the cube, disconnect other cube apps, then click **Connect** in the header and confirm the existing connection dialog. The tool's Connect action also selects Bluetooth timing, avoiding a connected cube with keyboard timing still selected. Defaults use the existing 3D virtual cube, CFOP splits and exact scramble matching. Linux may need the cube's MAC address in the upstream connection flow; csTimer remembers it locally.

The history and CFOP panel use real record data; select a row to review it, double-click for upstream details, and use Replay for timed reconstruction. Practice exposes existing csTimer tools and PLL/OLL case filters. The header menu retains import/export, session management and classic mode. Case transition gaps are not independent measurements of visual recognition.

Upstream GAN v4 does not handle gyro packets; the cube view supports manual orientation and reset. Scramble guidance now retains the original row, inserts verified inverse moves at its cursor, marks a partial half turn with ◐, and accepts opposite-face turns in either order. Unsupported notation or unexplained packet jumps defer to the upstream hint. Readiness and solve timing remain upstream. The HTM counter merges same-face turns within a commuting block, including reversals; R2 counts once.

## Acubemy study

Inspected the public playground, case database, history and training entry points on 2026-10-26 in isolated desktop and phone browser sessions. Computed styles and rendered font inspection confirmed Geist Mono for the timer and scramble, with CSS weight 500 and full-size decimals. The playground clock measured 76px on desktop and 48px on phone; it kept that typography while running. Geist Sans serves controls. This fork adapts those roles to csTimer's layout and size controls rather than copying Acubemy's page geometry.

The most useful interaction is case → breakdown → alternatives → practice. The [PLL case grid](https://acubemy.com/statistics/pll) can sort by total, recognition or execution time. An individual [Aa case page](https://acubemy.com/statistics/pll/aa) puts those components, reference distributions, alternative algorithms and Drill/Learn actions together. Algorithm percentages describe the share of users using an alternative, including speed cohorts; they are not predicted personal time savings. [F2L case pages](https://acubemy.com/statistics/f2l/f2l-1) preserve case identity while changing the selected slot and its alternatives. These observations favor linking existing tools and retaining context over adding another dashboard.

csTimer already supplies reconstruction case statistics and case-filtered scramble/training tools. Its case table currently has no direct drill action. Connecting that row to the exact case filter is follow-up work, not an implemented feature. Reconstruction's stage-start-to-first-move interval is a transition/pause proxy, not an independent measurement of visual recognition. Acubemy documents STM in its [FAQ](https://acubemy.com/faq); do not assume equivalence with csTimer's default HTM counts from GAN outer-face reports.

Study limits: no account, physical cube pairing or completed solve was used, and mutating requests were blocked. The [Training landing](https://acubemy.com/training) displayed Login Required; case-specific Drill/Learn navigation was observed, but drill timing, feedback and scheduling were not tested. Personal insights and completed smart-cube reconstruction were not verified. Empty screens under these restrictions do not establish a product defect or universal login requirement.

## Twisty Timer study

Inspected the original [Twisty Timer v4.7.0 source](https://github.com/aricneto/TwistyTimer/tree/2fcd6f7d2697a0810f37245ac033f1da12448382), pinned to commit `2fcd6f7d2697a0810f37245ac033f1da12448382`, and all seven screenshots on its [official Play listing](https://play.google.com/store/apps/details?id=com.aricneto.twistytimer). This is a source/layout and published-screenshot study, not an Android runtime test. It concerns the original app, not Cubic Timer or a Bluetooth fork. The source snapshot is in ignored `.data/twisty-study/source`; the previously collected store screenshots are in the older Cube Lab repository's `.data/reference-ui/twisty-*.png`.

Twisty's typography differs from Acubemy: its XML specifies a centered Lato Bold clock at 76sp, auto-sized up to 90sp, with smaller fractions. Quicksand Medium serves the interface and Lato 12sp the peripheral summaries. These are configured font roles, not measured rendered fonts. See [timer layout](https://github.com/aricneto/TwistyTimer/blob/2fcd6f7d2697a0810f37245ac033f1da12448382/app/src/main/res/layout/fragment_timer.xml#L59). The transferable idea is a clear size hierarchy; the two apps do not share a single typography recipe.

The [timing state transition](https://github.com/aricneto/TwistyTimer/blob/2fcd6f7d2697a0810f37245ac033f1da12448382/app/src/main/java/com/aricneto/twistytimer/fragment/TimerFragment.java#L1269) coordinates fading/translating the scramble, diagram, summaries and actions away while the centered clock scales to 1.15 times its resting size. Default animation duration is 250ms; navigation is locked during timing. On start it preserves the displayed scramble as `currentScramble` and prepares the next scramble in the background. On stop the recorded solve retains the original scramble and the surrounding controls return. Delete, DNF, +2 and comment are close to the finished result; penalty Undo reverses a penalty only.

Timer, List and Graph share puzzle/category context. History uses a three-column portrait card grid; tapping opens details, long-pressing selects multiple solves, and search filters comments. The detail dialog retains time, date, penalty, comment and scramble, with an expandable diagram. This user's requested long compact history list should stay a table: borrow the contextual actions and navigation, not Twisty's card grid.

Its [trainer](https://github.com/aricneto/TwistyTimer/blob/2fcd6f7d2697a0810f37245ac033f1da12448382/app/src/main/java/com/aricneto/twistytimer/puzzle/TrainerScrambler.java#L75) supports OLL and PLL. A bottom sheet selects cases; tapping toggles selection, long-pressing opens the algorithm detail, and closing the sheet generates a training scramble. Selected sets persist by family and named category. Algorithm-library progress is a manually assigned 0–100 value. It is not inferred mastery. Current-session statistics cover unarchived solves; all-time includes archived solves. Charts emphasize trends, while tables give precise current/session-best/all-time-best values.

Capability limit: the original app has no Bluetooth implementation. Its [solve model](https://github.com/aricneto/TwistyTimer/blob/2fcd6f7d2697a0810f37245ac033f1da12448382/app/src/main/java/com/aricneto/twistytimer/items/Solve.java#L11) stores total time, puzzle/category, scramble, date, penalty, comment and archive state, without case identity or timed moves. It cannot substantiate measured recognition, execution, TPS, automatic per-case improvement or GAN latency correction. The requested smart-cube drill metrics require richer records than this model.

### Consequences for this fork

The implemented timer/history/CFOP interface follows these constraints. Direct case-row-to-drill navigation and personalized algorithm recommendations remain follow-up work:

- Preserve the centered clock and make ready/running/finished states reveal only relevant actions. Keep immediate result actions next to the result.
- Retain compact history and preserve session, method and case context when opening analysis or practice.
- Connect an observed weak case to its breakdown, algorithm evidence and exact drill. Show sample counts and retain F2L slot identity.
- Persist the drill's target case and measured move stream. Separate self-rated familiarity, population popularity and measured improvement.
- Keep chart labels sparse and exact values in tables. Distinguish current, session-best and lifetime-best statistics.
- Keep csTimer responsible for timing, Bluetooth, scramble matching, reconstruction and metric semantics. Validate any interaction changes with workflow screenshots before implementation.

## Changes and checks

- `src/js/local-defaults.js`: fresh-profile defaults and a one-time full-size-decimal default when no decimal-size preference is stored.
- `src/js/local-appearance.js`, `src/css/local.css`, `src/fonts/`: optional typography under Options → Display.
- `src/js/local-metrics.js`, `src/css/local-metrics.css`: optional average TPS/moves under Options → Statistics, with independent sample coverage.
- `src/js/local-interface.js`, `src/js/local-cube-view.js`, `src/js/local-scramble-guide.js`, `src/interface/`: live interface, visual cube and verified guidance.
- `companion/`: native Rust desktop widget.
- `scripts/`: local build, loopback bridge, app/widget launchers and installation. The build uses the upstream source template without production analytics and preserves its core scripts.

```sh
npm test
nix develop --command make check
npm ci
CSTIMER_URL=http://localhost:3330 npm run test:browser
CSTIMER_URL=http://localhost:3330 npm run test:browser:classic
CSTIMER_URL=http://localhost:3330 nix develop --command node scripts/capture-native-companion.mjs
```

Additional browser checks use isolated synthetic sessions; they do not pair with the physical cube or modify personal history. The original project README follows upstream and is retained unchanged.

Browser and native captures use isolated synthetic records on a separate test server (`CSTIMER_PORT=3330 npm start`), never the personal browser profile. Physical GAN pairing, radio behavior and latency still require a hardware test; simulations do not establish that validation.

History includes an equal-width time histogram for the current search/filter across all matching records (not just loaded rows). Bins use upstream scored time including +2; DNFs are counted separately, invalid times excluded. Hover or focus a bin for its half-open range, count and percentage. Empty and single-time sessions are supported without fabricated spread.

On wide History views the histogram shares the overview column grid with a compact statistics block: median, population standard deviation, nearest-rank P90, and DNF rate. These follow the current search/filter, include scored +2 times, exclude DNFs from timed statistics, and do not reconstruct moves. Standard deviation requires at least two timed solves. On phones the statistics form one compact row below the plot.

Finalized History styling uses the approved pair of cool ice-blue fills, with separate Light and Dark colors, contiguous bins, and the aligned side statistics. Bin inspection has a stable single-line header, full accessible ranges, and one keyboard stop; Left/Right/Home/End navigate bins without starting the timer. Review captures reload and verify synthetic data before publishing all screenshots together.

Typography pairs bundled Geist Sans (400/500/600) for UI text with Maple Mono NF (400/600) for measurements and cube notation. Headings, active navigation and primary values carry emphasis; secondary metrics stay regular. Clock size/weight preferences remain upstream-controlled. On phones the Bluetooth control is icon-only with a 44px hit area, status dot, and its existing full device/action accessible label; desktop retains the text. No network fonts are loaded.

The current cube uses a cartoon treatment: three-band toon lighting, matte face colors, charcoal seams and uniformly rounded caps. The studio HDR environment and procedural plastic finish are no longer loaded by the renderer. The existing facelet mapping, verified turn animation, center logo, drag and reset handlers remain authoritative.

The pace plot has a dashed reference at the complete session's non-DNF mean (including scored +2 penalties), with range expanded to keep it visible. A downward marker and faster-than-mean tooltip identify improved solves; DNFs never enter that mean. The bottom CFOP dock shows amber pause + primary execution time, preserving csTimer precision and raw measurements. Phase clicks choose the relevant review tool rather than opening a phase magnifier. The desktop right rail is dedicated to Tools. At shorter/narrower desktop sizes averages move into the left rail; mobile tools open as a dismissible sheet.

Cross/X-cross routes use the selected solve's recorded scramble in a worker running the upstream solvers. Rotation prefixes follow upstream's frame convention. OLL/PLL variants are from Tao Yu's MIT Alg-Trainer dataset (revision e94551529fc0db8dccb11a5257c0e39dbaed2726, notice in src/assets/LICENSE-alg-trainer.txt); each displayed candidate is aligned with and checked against the reconstructed starting state. Counts compare HTM, not predicted personal speed. Technique tips use measured move gaps, U2 counts and trigger patterns; finger placement is not measured. Solvers never replace the active scramble or write time/move data.

`npm run test:browser:review` verifies cross/X-cross goals, aligned OLL/PLL candidates, offline worker/catalog operation, tag persistence/export, session boundaries, and keyboard bookmarking without starting the timer. Workflow screenshots are published at `http://localhost:3328/review-workspace.html`.


Performance revision: the cartoon cube now uses four instanced batches sharing geometry and a single demand-driven frame scheduler. At the normal probe viewport it draws 49,464 triangles in four calls, versus 271,256 triangles in 135 calls before this change. There is no idle animation loop; hidden/offscreen views snap logically to the latest verified state without drawing or replaying a backlog. Incoming turn history is validated, queued with a shared 120ms catch-up budget, and pending same-direction quarters can render as a coherent half turn. Reduced motion and lost/restored WebGL contexts are handled explicitly. Native device pixel ratio is capped at 2 rather than forcing supersampling on 1× screens. Timer timestamps, move counts and scramble readiness remain upstream-owned.

The modern renderer loads on demand; Classic does not initialize it. Statistics refreshes are coalesced to one display frame and per-record sensor metrics are weakly cached. The hidden stock average-metrics addon does no duplicate work while the modern interface is active. Unused HDR, font and shader assets are omitted from generated output, reducing the uncompressed offline bundle from 4,688,907 bytes to about 2.67 MB; their original sources and notices remain in the repository. All required tools and fonts remain available offline.

`npm run test:browser:performance` checks draw/triangle budgets, no idle or hidden GPU draws, rapid-turn correctness, reduced motion, WebGL recovery, batched statistics and Classic's lazy renderer. `npm run profile:interface` writes a synthetic headless-Brave/SwiftShader report under `.data/`. Frame timings in that probe are not a hardware FPS guarantee. Baseline and optimized captures are local synthetic data; no browser profile or physical Bluetooth connection is used.

Builds fingerprint code URLs (including module/worker dependencies and companion assets) and precache those exact URLs. This avoids mixed-version startup through an older cache-first worker. The cache schema is salted so old bare-URL caches are retired. `npm run test:browser:cache` verifies a single navigation can load the current interface and geometry while a simulated old worker serves stale unversioned code.

Run browser probes against a separate loopback server: `CSTIMER_PORT=3330 npm start`. They launch isolated browser contexts with sample records; the installed application uses port 3327.


Native Rust desktop and Kotlin Android clients are documented in [NATIVE.md](NATIVE.md). Their code, storage and artifacts are separate from the web application; the approved web UI is the visual reference.
