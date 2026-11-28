# What Le Mans Ultimate exposes to an external app

Research for [#22](https://github.com/jwh3times/RaceEngineerOverlays/issues/22), on the
wayfinder map [#21](https://github.com/jwh3times/RaceEngineerOverlays/issues/21). Researched
2026-11-27, against LMU v1.4.x (the newest release notes are v1.4.3, 2026-10-02). It lives on the
`research/lmu-shared-memory` branch, which is never merged.

Public material only. No code is copied here, and nothing comes from a recorded tape (`AGENTS.md`
rule 7). Field names are given so the adapter work can find them; layouts and code are described,
not reproduced.

## Short answer

LMU offers an external app **three data surfaces**, and an overlay needs all three:

1. **The built-in shared memory**: a named file mapping, `LMU_Data`. It was added in v1.2
   (December 2025) and extended in v1.3. It carries the rFactor 2 "V01" scoring and telemetry
   structs for up to 104 cars: telemetry at about 100 Hz for every car, scoring at about 5 Hz. It
   also has a cooperative lock and update events, so reads can be synchronised. Without the lock,
   reads can tear.
2. **A local REST API** on `http://127.0.0.1:6397`, the sim's own UI backend. It carries the
   things shared memory does not: the weather forecast, the pit menu and refuel target, pit-stop
   and repair time estimates, the tyre compound catalogue, team names, nationalities, the track
   outline and the in-car / at-the-monitor state.
3. **A local WebSocket** on `ws://127.0.0.1:6398/websocket/ui`, topic `LiveStandings`. It is the
   only source found that gives a penalty's kind (drive-through, stop-go, time) for every car.

There is also the inherited **rFactor 2 plugin interface**: a DLL loaded into the game, which is
what Racelab still uses. Surfaces 2 and 3 are not documented by Studio 397. Everything known
about them comes from open-source overlays reading them.

Per widget, LMU can feed every one of the 13, but four only partly: **Race incidents** (LMU has no
incident points), **Setup cover** (no field for "the setup screen is open"), **Weather** (the
forecast is REST-only) and **Pit strategy** (pit-lane loss must be learned). The table under
[Per widget](#per-widget) gives the detail.

A tape of an LMU session has to record more than shared memory: the REST bodies and the WebSocket
frames too. irDashies' `.lmudt` records shared memory and REST, but not the WebSocket.

## Sources and how far each is trusted

The primary source for the shared-memory layout is S397's own header set, which ships with the
game in `<LMU install>\Support\SharedMemoryInterface\` (`SharedMemoryInterface.hpp`,
`InternalsPlugin.hpp`, `PluginObjects.hpp`) [S1, S10]. **It is not published anywhere public, and
LMU is not installed on this machine, so the header itself was not read for this note.** TRACE
reports that the headers forbid redistributing or modifying them [S10]. The layout facts below
therefore come from public mirrors that say they were built from that header, cross-checked
against each other. Treat field-level detail as "agreed by several independent readers of the
header", not as read from the header.

| Id | Source | Kind | Trust |
| --- | --- | --- | --- |
| S1 | LMU release notes, V1.2 (2025-12-08): "Added SharedMemory Header file to support folder for new shared memory implementation". [guide.lemansultimate.com](https://guide.lemansultimate.com/hc/en-gb/articles/14556121957775-V1-2-Update-2) | Official, Studio 397 / Motorsport Games | High |
| S2 | V1.2.1 (2025-12-16): "Signed PluginAdapter.exe". [link](https://guide.lemansultimate.com/hc/en-gb/articles/14634667416463-V1-2-1-Update-2-Patch-1) | Official | High |
| S3 | V1.2.2.0 (2026-01-19): "Added toggle for Enable Plugins to Settings page". [link](https://guide.lemansultimate.com/hc/en-gb/articles/14928780204303-V1-2-2-0-Update-2-Patch-2) | Official | High |
| S4 | V1.3.0.0 (2026-03-30): "Added various new values to plugin system". [link](https://guide.lemansultimate.com/hc/en-gb/articles/15634539149711-Update-V1-3-0-0). V1.3 announcement: "new and highly requested telemetry parameters have been added to our shared memory file". [lemansultimate.com](https://lemansultimate.com/le-mans-ultimate-releases-v1-3-update-with-final-elms-content-performance-updates/) | Official | High |
| S5 | V1.4.2 (2026-09-22): "Fixed some threading issues with shared memory". [link](https://guide.lemansultimate.com/hc/en-gb/articles/17713037697807-V1-4-2-Update-1-4-Patch-2) | Official | High |
| S6 | Known Issues and Advice: plugins are switched off in `UserData\player\CustomPluginVariables.JSON`. [link](https://guide.lemansultimate.com/hc/en-gb/articles/13240843908623-Known-Issues-and-Advice) | Official | High |
| S7 | TinyPedal's `pyLMUSharedMemory` (MIT), commit `e9b71b6`: `lmu_data.py`, `lmu_mmap.py`. It says it is "based on LMU SharedMemoryInterface header file by S397". [github](https://github.com/TinyPedal/pyLMUSharedMemory) | Header-derived mirror, with the header's field comments | Medium-high |
| S8 | TinyPedal (GPL-3.0), commit `62aa71f`: `tinypedal/adapter/lmu_sharedmemory.py`, `lmu_reader.py`, `lmu_restapi.py`, `tinypedal/process/session.py`. [github](https://github.com/TinyPedal/TinyPedal) | Mature open-source LMU overlay | Medium |
| S9 | irDashies (MIT), commit `f831814`: `docs/LMU_RECORD_REPLAY.md`, `docs/LMU_TRACK_GEOMETRY.md`, `src/app/lmu/**` (struct port, shared-memory source, tape format, REST tasks, WebSocket, mappers). [github](https://github.com/tariknz/irdashies) | Open-source overlay, the one reo replaces | Medium |
| S10 | TRACE, `docs/le-mans-ultimate.md` (MIT), commit `d927b16`. Measured against the installed header. [github](https://github.com/keystroke-tools/TRACE/blob/master/docs/le-mans-ultimate.md) | Open-source telemetry tool | Medium |
| S11 | SimMagicFix, `src/core/lmu_lock.h` (MIT), commit `42feb96`: a lock-compatible reimplementation. [github](https://github.com/SunNathan/SimMagicFix) | Open source | Medium |
| S12 | LMU Pitwall README (MIT): the "Enable Plugins" switch and the rates. [github](https://github.com/Swizzjack/lmu-pitwall) | Open source | Medium-low |
| S13 | goLMUSharedMemory README: pack(4), "supports LMU v1.4.2", the layout changes between versions. [github](https://github.com/LuisGasparGH/goLMUSharedMemory) | Open source | Medium-low |
| S14 | The rFactor 2 Internals Plugin SDK `InternalsPlugin.hpp`, as carried by TheIronWolf's rF2SharedMemoryMapPlugin (commit `48aa12d`), and that plugin's README. [github](https://github.com/TheIronWolfModding/rF2SharedMemoryMapPlugin). The official SDK download is on Studio 397's modding resources page ([studio-397.com/modding-resources](https://www.studio-397.com/modding-resources/)), not read here. | rF2 SDK copy | Medium-high for rF2; LMU's copy may differ |
| S15 | Racelab, LMU first steps: install `RacelabLMUPlugin.dll` and enable it in `CustomPluginVariables.json`. [garage.racelab.app](https://garage.racelab.app/docs/first-steps/lmu) | Competitor public docs | High for what it says |

No forum post by a Studio 397 developer announcing the shared memory could be found: searches of
`community.lemansultimate.com` turned up nothing indexable. The release notes (S1 to S5) are the
only official word.

## The interfaces

### 1. Built-in shared memory (`LMU_Data`)

**Turning it on.** Settings, Gameplay, *Enable Plugins* (a toggle added in V1.2.2 [S3]) has to be
on, followed by a full restart of the game. With it off, no `LMU_Data` mapping appears [S12]. That
the same switch controls the built-in shared memory and the DLL plugins is community-observed
[S12], not stated in the release notes. reo should say so in its "waiting for LMU" message.

**Objects.** All are Windows named kernel objects, created by the game [S10, S11, S9]:

| Name | What it is |
| --- | --- |
| `LMU_Data` | The data mapping: one fixed struct, about 325 KB. |
| `LMU_SharedMemoryLockData` | An 8-byte mapping, `{ waiters, busy }`, for a cooperative spin lock. |
| `LMU_SharedMemoryLockEvent` | An auto-reset event that wakes waiters when the lock is released. |
| `LMU_Data_HoldEvent` | Signalled at the start of an update cycle. |
| `LMU_Data_DataEvent` | Signalled when the next update is ready. |

irDashies opens `LMU_Data` read-only (`FILE_MAP_READ`) and checks that the view's region is at
least the struct size before using it [S9]. The same approach works for a Rust reader.

**Layout.** It uses x64 Windows with `#pragma pack(4)` [S10, S13, S7]. The top-level object
(`SharedMemoryObjectOut`) holds, in order [S7, S9, S10]:

- **generic** (332 B): an event block of 16 `uint32` counters, one per plugin callback
  (`SME_ENTER` ... `SME_UPDATE_SCORING`, `SME_UPDATE_TELEMETRY`, ... `SME_FFB`), plus
  `gameVersion`, the FFB torque, and the rF2 `ApplicationStateV01` (window handle, resolution,
  `mOptionsLocation`, `mOptionsPage`).
- **paths** (1,300 B): five 260-byte Windows paths (`userData`, `customVariables`,
  `stewardResults`, `playerProfile`, `pluginsFolder`).
- **scoring** (starts at byte 1,632): the rF2 `ScoringInfoV01` (session, clocks, phases, flags,
  weather now), a 64 KB text **results stream**, and 104 × `VehicleScoringInfoV01` (584 B each).
- **telemetry** (starts at byte 128,464): `activeVehicles`, `playerVehicleIdx`,
  `playerHasVehicle`, then 104 × `TelemInfoV01` (1,888 B each, four wheels included).

**Total size: 324,820 bytes per irDashies [S9], but 324,824 per TRACE [S10].** The difference
isn't explained; it may be a different game build or a different counting of the outer layout
struct. The layout has already changed once (V1.3 added fields [S4]) and readers pin a version
(goLMUSharedMemory: "supports LMU v1.4.2" [S13]). **The adapter must check the mapping size and
`generic.gameVersion`, and fail closed on a mismatch**, which irDashies and TRACE both do [S9,
S10].

LMU's struct differs from rF2's in its tails. It carves rF2's `VehicleScoringInfoV01`
`mExpansion[48]` into `mSteamID`, `mVehFilename`, `mAttackMode`, `mFuelFraction`, `mDRSState`
and 4 spare bytes [S9]. `TelemInfoV01` gains the LMU fields listed under the widgets below
(electronics levels, virtual energy, the gaps ahead and behind, `mDeltaBest`, `mLapInvalidated`,
track-limit steps, the vehicle model and class). So **rF2 readers do not read LMU correctly**, and
the struct must be LMU's own.

**Rates.** Telemetry is written at about 100 Hz for every car, scoring at about 5 Hz [S9, S12].
irDashies' recorder polls every 10 ms "matching the rate the sim publishes at" [S9]. The rF2 SDK
documents scoring as "approximately five times per second" [S14]. Neither rate is in an official
LMU document.

**Can reads tear? Yes.** The game writes the mapping in place. Three approaches are in use:

- **The official lock** [S10, S11]: spin on a compare-exchange of `busy` from 0 to 1 a bounded
  number of times. Then register as a waiter and wait on `LMU_SharedMemoryLockEvent`. Copy the
  whole mapping, set `busy` back to 0, and signal the event if anyone is waiting. TRACE copies the
  complete mapping under the lock, releases it, then parses the copy, and reports lock contention
  as a retryable miss [S10]. Taking the lock needs write access to `LMU_SharedMemoryLockData`. It
  is cooperative, so a reader that holds it stalls the game's writer: hold it only for the copy.
- **Update counters, no lock** [S9]: read `SME_UPDATE_SCORING` and `SME_UPDATE_TELEMETRY` twice
  back to back and skip if they moved. Then copy, re-read them, and accept the copy only if all
  three readings agree. Retry up to four times. Vehicle counts are deliberately left out of the
  check: scoring's `mNumVehicles` (5 Hz) and telemetry's `activeVehicles` (100 Hz) legitimately
  differ for up to one scoring interval when a car joins or leaves.
- **Update events** (`LMU_Data_HoldEvent`, `LMU_Data_DataEvent`) for an event-driven reader
  instead of polling [S10, S11]. Nobody surveyed relies on them yet.

V1.4.2 "fixed some threading issues with shared memory" [S5], so builds before it may tear more.
This matches `AGENTS.md` rule 3: copy out, never hold references, validate after copying. The LMU
equivalent of iRacing's tick check is the counter comparison or the lock.

**Liveness: the mapping outlives the session.** When a session ends, the block simply stops being
written. The last snapshot stays readable, with `mInRealtime` still true [S9, S8]. TinyPedal and
irDashies both call the data stale after **2 s with no change** in the scoring counter or the
session clock `mCurrentET` [S8, S9]. irDashies also treats the sim as gone when
`generic.appInfo.mAppWindow` is no longer a live window [S9].

**Indexing.** Scoring and telemetry arrays are **not** in the same order. Match them by `mID`
(the slot ID, which multiplayer can reuse after someone leaves). Find the player in scoring by
`mIsPlayer`, and in telemetry by `playerVehicleIdx` / `playerHasVehicle` [S8, S10].

### 2. The local REST API (`127.0.0.1:6397`)

This is a JSON API served by the sim itself, on a fixed loopback port. It is **undocumented**: no Studio 397 page describing it was found, and everything here is from
the overlays that poll it [S8, S9]. The port is not necessarily open when shared memory first
appears, so a poller must retry rather than latch off [S9]. Some paths answer only in some states
(404 outside a game, spectator-only paths) [S9].

The paths in use, with what they give an overlay [S9 `src/app/lmu/rest/tasks.ts` and
`lmu_replay_main.cpp`; S8 `lmu_restapi.py`]:

| Path | Gives |
| --- | --- |
| `/rest/sessions/weather` | The **weather forecast** for practice, qualifying and race |
| `/rest/sessions` | Session settings (time scale, private qualifying) |
| `/rest/sessions/GetGameState` | Where the player is: driving, paused, at the monitor, not in a session (an error body), not in a game (404) |
| `/rest/garage/UIScreen/RepairAndRefuel` | The pit menu's refuel target, tank size, maximum virtual energy, and wear and damage (aero, brakes, suspension) |
| `/rest/strategy/pitstop-estimate` | Estimated stop time, repair time, and penalty seconds to serve |
| `/rest/garage/UIScreen/TireManagement` | The tyre compound catalogue (index to name) |
| `/rest/garage/UIScreen/CarSetupOverview` | Setup values (fuel ratio); the largest payload |
| `/rest/garage/getPlayerGarageData` | Garage setup (TinyPedal only) |
| `/rest/watch/standings` | The whole field with team names and car numbers; tens of KB, changes every poll |
| `/rest/watch/focus` | Which car the spectator camera is on (spectator view only) |
| `/rest/watch/trackmap` | LMU's own racing line, as world points |
| `/rest/multiplayer/teams` | Driver nationalities (online only; filled in a little after a driver joins) |

Cost note: irDashies hashes each body, skips parsing when it hasn't changed, and backs off to 5 s.
It polls `/rest/watch/standings` only every 10 s, because parsing it costs real CPU [S9]. That
matters for the budget: under 10% of a core.

### 3. The UI WebSocket (`127.0.0.1:6398/websocket/ui`)

Subscribe with a `SUB` message for topic `LiveStandings`. The sim then pushes the whole grid, one
entry per car, with **penalties broken out by kind** (drive-through, stop-go, time seconds) and
the track-limits allowance [S9 `liveStandings/constants.ts`, `penalties.ts`]. A penalty
disappearing from a frame means it has been served. irDashies treats the feed as stale after 15 s
of silence and reconnects indefinitely. It is undocumented, like the REST API.

### 4. The rFactor 2 plugin interface (inherited)

LMU still loads rF2-style Internals Plugin DLLs from its `Plugins` folder, configured in
`UserData\player\CustomPluginVariables.JSON` [S6, S15]. It runs them through a `PluginAdapter.exe`
[S2] and keeps extending them ("various new values to plugin system" [S4]). The rF2 SDK's
callbacks are `UpdateScoring` (about 5 Hz), `UpdateTelemetry` (player only or all vehicles),
`UpdateGraphics`, `AccessTrackRules`, `AccessMultiSessionRules`, `AccessPitMenu` and
`AccessWeather` [S14]. Through them a plugin reaches data the built-in shared memory does not
carry: rules, the pit menu and weather control. Racelab ships its own LMU plugin DLL [S15].
TheIronWolf's rF2 shared-memory plugin was the old community route. Its README lists telemetry
at 50 FPS, scoring at 5 FPS, and "weak synchronization by using version variables" against torn
frames [S14]. Tools are leaving it now that `LMU_Data` exists [S12].

For reo, a plugin means shipping a native DLL that runs inside the game's process: a C++ (or
`cdylib`) build, an install step into the game folder, and the user editing a JSON file. The
built-in shared memory plus REST covers every widget without one. **The plugin route is only
worth it if a later need is unreachable otherwise.** Rules and the pit menu are mostly reachable
over REST already.

## Per widget

Key: **available** means a field gives it; **derivable** means reo computes it from available
fields; **REST** or **WS** means it is only on the local API or WebSocket; **missing** means no
surface surveyed has it.

| Widget | Available | Derivable | Missing or REST/WS only |
| --- | --- | --- | --- |
| **Standings** | Overall `mPlace`, class name `mVehicleClass` (32 chars), gaps `mTimeBehindLeader`/`mTimeBehindNext` with laps-behind, best and last lap, best and last sectors, `mNumPitstops`, `mQualification` (grid), per-car front and rear compound index and name, `mVehicleModel` ("brand & model"), `mFinishStatus`, `mDriverName`, `mSteamID` | **Class position** (LMU publishes overall only [S9]); car number and team, parsed from the entry name `"<Team> #<Number>:<Series>"` [S9]; positions gained (grid to now); recent laps and stint length, from lap and pit history kept by reo; interval | Team name (REST `/rest/watch/standings`); nationality (REST `/rest/multiplayer/teams`); **manufacturer emblem**: no emblem id, only the model string, so an abbreviation can be mapped from `mVehicleModel` |
| **Relative** | `mLapDist`, track length `mLapDist` (session), `mTotalLaps`, `mTimeIntoLap`, `mEstimatedLapTime`, `mInPits`, class; the player's `mTimeGapCarAhead`/`Behind` | Time to each nearby car from lap-distance difference and estimated lap time; lapping and being lapped from `mTotalLaps` | none |
| **Track map** | Every car's world position `mPos` and orientation in telemetry at 100 Hz (also in scoring at 5 Hz); `mLapDist`; `mPathLateral`, `mTrackEdge` | Track outline, by learning it from driven laps (irDashies learns road edges from `mPathLateral`/`mTrackEdge` [S9]) | LMU's racing line as an outline: REST `/rest/watch/trackmap` |
| **Radar** | World positions and orientation of every car (`mPos`, `mOri`) at 100 Hz, per-car `mElapsedTime` | Relative positions: rotate into the player's frame. Opponents' telemetry can carry a different timestamp, and irDashies aligns them by `mElapsedTime` [S9] | No sim-side "car left/right" signal; car dimensions are not published (irDashies assumes 2.2 m by 4.6 m [S9]) |
| **Blind spot** | as Radar | Car alongside from the same relative positions [S9] | No spotter signal like iRacing's `CarLeftRight` |
| **Weather** | `mAmbientTemp`, `mTrackTemp` (°C), `mRaining` (0..1), `mDarkCloud`, path wetness min/avg/max (0..1), `mWind` (a vector), `mCloudCoverage` (0..10 sky type), `mTrackGripLevel` (0..4), `mTimeOfDay` | Wetness trend | **Forecast**: REST `/rest/sessions/weather` only [S9, S8] |
| **Flag** | `mGamePhase` (formation, green, full-course yellow or safety car, stopped, over), `mYellowFlagState` (pending, pits closed or open, last lap, resume), `mSectorFlag[3]` (local yellows by sector), per-car `mFlag` (the header says only green or blue), `mUnderYellow`, `mFinishStatus` (DQ), `mIndividualPhase` | Chequered from phase 8 or finish status, red from phase 7, yellow from phase 6, the yellow state or a sector flag, blue from the car's `mFlag` (irDashies' mapping [S9 `mapTelemetry.ts`]); white would have to come from laps or time remaining | No black, meatball or white flag field; a penalty is only a count (`mNumPenalties`); its kind comes from the WS |
| **Inputs** | Throttle, brake and clutch (0..1) and steering (-1..1), filtered and unfiltered; `mGear` (-1 = reverse, 0 = neutral); `mPhysicalSteeringWheelRange`; `mSpeedLimiterActive`, `mABSActive`, `mTCActive` | Speed from the `mLocalVel` magnitude; steering angle as input × range | Range units are not stated in the header; TinyPedal reads it as degrees and falls back to REST when it is ≤ 0 [S8] |
| **Delta** | `mDeltaBest` (LMU's own delta to the best lap, player car) [S7, S10]; `mLapStartET`, `mElapsedTime`, `mLapDist` | A delta to any other reference lap, from a recorded lap-distance trace | none |
| **Fuel** | `mFuel` (L), `mFuelCapacity` (L), `mVirtualEnergy` (fraction), per-car `mFuelFraction` (byte, /255), `mBatteryChargeFraction`, `mStateOfCharge` (%) | Fuel or energy per lap, laps remaining, and amount needed to finish (from `mSessionTimeRemaining`/`mEndET`/`mMaxLaps`); energy-limited cars (Hypercar, LMDh) use virtual energy, the rest use fuel [S9] | Maximum virtual energy and the pit menu's refuel target: REST `RepairAndRefuel` |
| **Pit strategy** | `mPitState` (request, entering, stopped, exiting), `mInPits`, `mInGarageStall`, `mPitLapDist`, `mNumPitstops`, `mScheduledStops`, `mCurrentSector` sign bit (in the pit lane), `mYellowFlagState` (pits open or closed) | Pit-lane time loss, learned from the player's own stops; rejoin position from gaps and the loss | Stop and repair time estimates and penalty time to serve: REST `/rest/strategy/pitstop-estimate`; component wear: REST `RepairAndRefuel` |
| **Race incidents** | `mNumPenalties` per car; `mTrackLimitsSteps` (a telemetry field, read for the player by TRACE [S10]) with the session's `mTrackLimitsStepsPerPoint` and `StepsPerPenalty`; `mLapInvalidated`; per-car `mLastImpactET`/`mLastImpactMagnitude`; dent severity; and the **results stream**, whose lines `<Incident ...>`, `<TrackLimits ... CurrentPoints=...>` and `<Penalty ...>` name the drivers involved [S8 `session.py`] | A feed of incidents involving other cars, from the results stream; track-limit points against the allowance as the driver's "count against a limit" | **No iRacing-style incident points and no incident limit** [S9 `mapTelemetry.ts`: "LMU keeps no incident count"]; penalty kinds: WS `LiveStandings` |
| **Setup cover** | `ApplicationStateV01.mOptionsLocation` (main UI, loading, monitor, on track) and `mOptionsPage` (the page's name); `mInGarageStall`; GetGameState over REST (driving, paused, at the monitor) | "At the monitor or in the garage", which is when the setup screen can be up | **No field found that says the setup screen itself is open.** irDashies uses `mInGarageStall` as the stand-in [S9]. Whether `mOptionsPage` names the setup page in LMU is unverified and needs a live check |

Other gaps worth knowing: there is no display-units preference (LMU is metric throughout, so
irDashies hard-codes metric [S9]); no camera field outside REST `/rest/watch/focus`; no
team-racing or driver-swap field in shared memory.

## Strings, units and sentinels the adapter has to translate

Rule 1 says SI units, `0..=1` fractions, and `None` rather than sentinels. Against that, LMU hands
over:

**Strings.** Fixed-size `char` arrays: driver 32, vehicle 64, track 64, class 32, compound 18,
model 30, server 32, pit group 24, and so on. They are NUL-terminated when shorter, and a reader
must bound them for the case where they are full. **The encoding is not documented.** TinyPedal
decodes as UTF-8 by default and makes the encoding a user setting [S8], which suggests it varies
in practice. rF2 heritage points to the Windows ANSI code page. reo should decode leniently with
a fallback (UTF-8, then Windows-1252, as `reo_iracing` already handles 1252), and test both.
Strings have meaning inside them: the entry name `"<Team> #<Number>:<Series>"` [S9].

**Units that are not SI, or are mixed** [S7 field comments]:

| Field | Raw | Translate to |
| --- | --- | --- |
| Tyre surface, carcass and inner-layer temps | Kelvin | K, or the model's chosen temperature unit |
| `mOptimalTemp`, `mBrakeTemp`, engine water and oil, ambient, track | Celsius | the same unit as the above (two units in one struct) |
| Tyre `mPressure` | kPa | Pa |
| `mBrakePressure` | currently 0..1 (the header says it "will convert to true brake pressure (kPa) in future") | a fraction, but watch for the change |
| `mStaticUndeflectedRadius` | centimetres, in a `u8` | m |
| `mFuelFraction` | `u8`, 0..255 | `/255` to 0..1 |
| `mStateOfCharge` | percent | 0..1 |
| `mRegen` | kW | W |
| `mWear` | 0..1. The header says "fraction of maximum", TRACE reads it as life remaining [S10] | 0..1, **with the direction confirmed live** |
| `mTimeOfDay`, `mStartET` | seconds since midnight | s |
| Steering ranges | not stated (read as degrees) | rad |
| World coordinates | left-handed, y up [S10]; +z points north [S9] | the model's frame |

**Sentinels and odd encodings:**

- Lap and sector times are **0 (or negative) when not set**. irDashies notes 0 is never a real lap
  time [S9]; TinyPedal tests `> 0` [S8]. Translate to `None`.
- `mBestSector2`, `mLastSector2` and `mCurSector2` are **cumulative** (sector 1 + 2). Sector 3
  is the lap minus sector 2. The `mBestLapSector*` fields are the sectors of the best lap, a
  different thing.
- Scoring `mSector` is 0 = sector 3, 1 = sector 1, 2 = sector 2 ("don't ask why", the header
  says). Telemetry `mCurrentSector` is zero-based, with the **sign bit set in the pit lane**.
- `mQualification` is -1 when invalid; `mYellowFlagState` -1 is invalid; `mGear` -1 is reverse.
- `mControl`: -1 nobody, 0 local player, 1 local AI, 2 remote, 3 replay. LMU keeps publishing the
  selected car during a saved replay, marked by `mControl` = 3 [S10], so the adapter must tell a
  replay from a drive.
- `mSteamID` 0 means an AI entry [S9].
- `mSession` packs the session kind into ranges: 0 test day, 1-4 practice, 5-8 qualifying,
  9 warm-up, 10-13 race.
- Virtual energy is **0, not absent**, on a fuel-limited car [S9]. Treat it as absent when the car
  has none.
- Floats can be NaN (TinyPedal filters every read through a NaN guard [S8]). Translate to `None`.
- Lap counters and completed lap times can be published on **different updates**: hold a
  completed lap until its time arrives [S10].
- In multiclass, the global phase goes to "session over" when the overall winner finishes, while
  other classes are still on their last lap [S10]. Use `mFinishStatus` per car.

## Recording: what an LMU tape must capture

To replay a session through the real adapter, the way `.irdt` tapes do for iRacing, a tape has
to hold:

1. **Every shared-memory snapshot**, taken with the lock or the counter check, at the telemetry
   rate (about 100 Hz). It has to include all cars' telemetry, which Radar, Blind spot and Track
   map need. Stamp each frame with a capture timestamp.
2. **The layout identity**: the mapping size and `generic.gameVersion`, so a tape from another
   build is refused rather than misread.
3. **Connection events**: the mapping appearing and going away, and the data freezing (a session
   ending) as distinct from the sim exiting.
4. **REST responses**: path, body and timestamp, written when a body changes. Errors are
   included, because a 404 or an "unavailable" body from GetGameState is itself an answer [S9].
5. **WebSocket `LiveStandings` frames**, with timestamps. Without them, penalty kinds can't be
   replayed.
6. The **results stream** stays inside the snapshot. It is the incident feed, and it carries
   driver names.

**Privacy (rule 7).** An LMU snapshot carries driver names, Steam IDs, the player name, server
name and public IP, the results stream, and five filesystem paths rooted at `C:\Users\<account>`
[S9 `anonymiseSnapshot`]. REST bodies carry names and nationalities. Such tapes stay in `tapes/`,
and the tape-privacy note on map #21 applies doubly.

**Size.** Stored raw, the snapshot comes to about **31 MB/s** (about 325 KB × 100 Hz), or about
19 GB for ten minutes [S9]. Most of it is empty slots and unchanged bytes.

### What irDashies' `.lmudt` records (format version 2)

From irDashies' public docs and headers [S9 `docs/LMU_RECORD_REPLAY.md`,
`src/app/lmu/native/replay/lmu_tape.h`, `lmu_tape.cpp`], described, not copied:

- **File header** (80 bytes, packed): an 8-byte magic (`IRDLMU1`), the format version (2), an
  endianness marker, the header and record-header sizes, **the snapshot size** (the tape is
  refused if it differs from the reader's struct size), the keyframe interval (100), the record
  count, the duration in microseconds, and reserved words.
- **Records**, each with a 32-byte header: kind, header size, encoded payload size, flags,
  microseconds since the first frame (playback keeps the original cadence), a 32-bit FNV-1a
  checksum of the payload, and a reserved word. The kinds are:
  - *Keyframe*: a whole snapshot, run-length encoded. The encoding alternates varint-counted runs
    of zero bytes and of literal bytes.
  - *Delta*: the snapshot XOR the previous one, then run-length encoded. One frame in 100 is a
    keyframe instead, so a tape can be looped, sought, or resumed after a corrupt record.
  - *Disconnect*: the sim went away.
  - *End*.
  - *Rest*: a path length, a body length, then the raw path and body, unencoded. It is written
    only when a body changes. The recorder polls 11 paths every 200 ms on its own thread.
- **Capture**: every 10 ms by default (`--poll`), with the same counter-based torn-read retry as
  its live addon. A torn read is skipped, not stored.
- **Not captured**: the WebSocket penalty feed (nothing in the recorder touches port 6398). The
  design doc says "all five REST endpoints", but the recorder now polls 11. The doc has fallen
  behind the code.
- **Anonymising** (`lmu_replay anonymise`): replaces driver and player names with pseudonyms and
  Steam IDs with obviously fake ones. Zeroes the profile path, server address and port, all five
  paths, and the results stream; renames the server. **Zeroing the results stream also removes
  the incident feed from an anonymised tape**, so a Race incidents replay test needs a synthetic
  stream instead.

For reo: `tapes/lmu-session.lmudt` (from irDashies' test data, MIT) can be read with this
description as a validation input, the way `.irdt` tapes are now. reo's own LMU recorder should
add the WebSocket, and should consider recording only occupied vehicle slots instead of the whole
104-slot arrays.

## Implications for reo

- **The adapter has three inputs, not one**: the `LMU_Data` reader (high rate, `unsafe`
  boundary, a copy under the lock or the counter check) and a REST and WebSocket poller (low
  rate, off the hot path, with backoff). The model stays the only contract: nothing REST-shaped
  leaks past `crates/model`.
- **Layout is per game version.** Pin offsets and sizes in tests built from synthetic images
  (rule 5), and refuse unknown sizes.
- **Licensing.** The S397 headers reportedly forbid redistribution and modification [S10]. Don't
  vendor them. Writing reo's own Rust layout from a reading of the installed header is what every
  open project surveyed does (pyLMUSharedMemory, MIT, says it is "based on" the header). Whether
  that is acceptable for an AGPL project is the maintainer's call, and may be worth an ADR.
- **Budget.** Copying 325 KB at 100 Hz is about 32 MB/s of memcpy. That is cheap, but reo can copy
  only the scoring block and the occupied telemetry slots instead of the whole mapping.

## What this research could not establish

- **The header itself.** It was not read (it isn't public, and LMU isn't installed here). Field
  details are as mirrored by S7, S9, S10 and S13. A one-off check on the rig is needed:
  `SharedMemoryInterface.hpp`'s exact size, the lock protocol, whether the events counters are
  counters, and the licence wording.
- **The 4-byte size disagreement** (324,820 vs 324,824).
- **The string encoding**: UTF-8 or the ANSI code page.
- **Whether `mOptionsPage` names the setup screen**, which would give Setup cover a real signal.
- **The direction of `mWear`**: worn fraction or life remaining.
- **The steering range units** and the **wind vector units**.
- **Official rates.** 100 Hz and 5 Hz come from community measurements, not a Studio 397 statement.
- **Any official word on the REST API or the WebSocket.** Both are undocumented and could change
  without notice.
- **Whether the results stream keeps its format across versions**, and whether it reaches the
  mapping for every incident online.
