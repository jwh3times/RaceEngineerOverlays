# Car positions and track outlines for Track map, Radar and Blind spot

Research for [#23](https://github.com/jwh3times/RaceEngineerOverlays/issues/23), on the
[route to reo 1.0](https://github.com/jwh3times/RaceEngineerOverlays/issues/21). Researched
2026-11-27. This file lives on the `research/positions-and-track-outlines` branch, which is never
merged.

## Answer

- **iRacing (live shared memory)** gives every car only its **progress along the lap**
  (`CarIdxLapDistPct`, plus `CarIdxTrackSurface` and `CarIdxOnPitRoad`), at 60 Hz. No car,
  not even the driver's, has a world X/Y/Z or latitude/longitude in live data. The driver alone also
  gets heading (`Yaw`, `YawNorth`), car-local velocity (`VelocityX/Y/Z`), the gaps to the cars
  directly ahead and behind (`CarDistAhead`, `CarDistBehind`) and the spotter's side indicator
  `CarLeftRight`.
- **iRacing disk telemetry (`.ibt`)** adds `Lat`, `Lon` and `Alt` for the driver's car only, and
  carries no per-car (`CarIdx*`) arrays.
- **LMU (built-in `LMU_Data` shared memory)** gives **every vehicle** a world position (`mPos`), an
  orientation matrix (`mOri`) and local velocity, both in the telemetry block and in the scoring block.
  The scoring block also has `mLapDist`, `mPathLateral` (lateral offset from an approximate centre
  path) and `mTrackEdge`. Scoring updates about 5 times per second. Telemetry updates far faster; the
  exact LMU rate is unconfirmed (see the open questions). LMU has no `CarLeftRight`-style indicator.
- **Radar and Blind spot for other cars.**
  - On LMU, a true two-dimensional Radar and a Blind spot are feasible for **any** car, because every
    car has a position and a heading.
  - On iRacing, both are feasible **only around the driver's own car**:
    - Distance along the track is exact for all cars (lap fraction × track length), but there is
      **no lateral position for any car**.
    - Side comes only from `CarLeftRight`, which is for the driver alone and does not say which car
      is alongside. Placing cars left or right is a heuristic, and so is pairing a car with the
      indicator.
    - When the camera is on another car (spectating, replay), only the along-track gaps remain:
      there is no side information at all.
- **Track outlines.**
  - Neither sim exposes track geometry through its SDK.
  - iRacing's own track-map SVGs (Data API track assets) are iRacing's property, with no licence
    granted. They cannot be bundled, and iRacing is not issuing new Data API client IDs at the moment.
  - Community datasets cover only about 20 to 30 real-world F1 and DTM circuits. They don't match sim
    coordinates or sim-only layouts, and their licensing is muddier than their labels: OpenStreetMap
    data is ODbL share-alike, and the datasets derived from it carry other labels.
  - **Recording the outline locally from the user's own laps is the only option that covers every
    track and carries no third-party licence**:
    - On LMU, from `mPos`.
    - On iRacing, by dead reckoning `Yaw` plus `VelocityX/Y`, or by reading `Lat`/`Lon` from the
      user's own `.ibt` files, closed into a loop with `LapDist`.

## iRacing

### What exists, and for whom

From the SDK header and the variable list on a recorded live session. The list was taken with
`reo vars --irdt tapes/ai-race-10min.irdt`: 333 variables, of which only the names, counts, units and
descriptions are used here.

| Variable | Count | Unit | Who | Notes |
|---|---|---|---|---|
| `CarIdxLapDistPct` | 72 (one per car index) | fraction of a lap | every car | The only per-car positional signal. Empty slots and cars not in the world carry a sentinel, commonly `-1` (unverified here). Trust `CarIdxTrackSurface` = `irsdk_NotInWorld` (-1) instead. |
| `CarIdxTrackSurface` | 72 | `irsdk_TrkLoc` | every car | `NotInWorld`, `OffTrack`, `InPitStall`, `AproachingPits`, `OnTrack`. |
| `CarIdxTrackSurfaceMaterial` | 72 | `irsdk_TrkSurf` | every car | Asphalt, grass, sand and so on. |
| `CarIdxOnPitRoad` | 72 | bool | every car | Between the pit cones. |
| `CarIdxEstTime` | 72 | s | every car | "Estimated time to reach current location on track". |
| `CarIdxLap`, `CarIdxLapCompleted` | 72 | – | every car | Combine with the lap fraction for total race distance. |
| `CarIdxSteer`, `CarIdxRPM`, `CarIdxGear` | 72 | rad, rpm, – | every car | No speed and no heading per car. |
| `LapDist` | 1 | m | driver | "Meters traveled from S/F this lap". |
| `LapDistPct` | 1 | fraction | driver | |
| `CarDistAhead`, `CarDistBehind` | 1 | m | driver | The first car ahead or behind. |
| `CarLeftRight` | 1 | `irsdk_CarLeftRight` | driver | The spotter indicator; see below. |
| `Yaw`, `YawNorth`, `Pitch`, `Roll` | 1 | rad | driver | `YawNorth` is "relative to north". |
| `VelocityX/Y/Z` (+ `_ST` at 360 Hz, 6 subsamples) | 1 / 6 | m/s | driver | Car-local frame (see below). |
| `YawRate`, `LatAccel`, `LongAccel` (+ `_ST`) | 1 / 6 | rad/s, m/s² | driver | |
| `Speed` | 1 | m/s | driver | |

The variables that are **not** in live shared memory matter as much:

- No `Lat`, `Lon` or `Alt`: the live tape has none.
- No world X/Y/Z for any car.
- No per-car speed, heading or lateral offset.

The session YAML supplies what turns the lap fraction into metres and orients a map:

- `WeekendInfo` has `TrackLength` (a string with units, such as "x km"), `TrackNorthOffset`,
  `TrackLatitude`, `TrackLongitude`, `TrackAltitude`, `TrackID` and `TrackConfigName`.
- `SplitTimeInfo` has `Sectors` with `SectorStartPct`.

These are key names from `reo session --raw`.

`irsdk_CarLeftRight` (from `irsdk_defines.h`):

```
irsdk_LROff, irsdk_LRClear,          // no cars around us
irsdk_LRCarLeft, irsdk_LRCarRight,   // a car to our left / right
irsdk_LRCarLeftRight,                // cars on each side
irsdk_LR2CarsLeft, irsdk_LR2CarsRight // two cars to our left / right
```

It counts cars beside the driver (up to two on one side), but it does not identify them or measure a
lateral distance.

**Update rate.** `irsdk_defines.h` says variables "are output at a rate of 60 times a second", and
that the sim "writes a new line of variables every 16 ms". The header's `tickRate` field is described
as "ticks per second (60 or 360 etc)". The `_ST` variables pack six 360 Hz subsamples into one 60 Hz
row, for the driver only. Every `CarIdx*` array, `CarLeftRight` included, is a 60 Hz value.

**Disk telemetry (`.ibt`).** The header describes it as "Detailed information about the local drivers
car", written by Alt-L, whose format "directly mirrors the format of the live data". The variable
headers of a local `.ibt` (names only) show what it carries:

- `Lat` ("Latitude in decimal degrees"), `Lon` and `Alt` ("Altitude in meters"), at a 60 Hz tick rate.
- No `CarIdx*` arrays and no `CarLeftRight`.

So GPS-style coordinates exist only after the fact, and only for the driver. The Mu converter
(`.ibt` to MoTeC) documents the same use: "GPS in iRacing allows for … the generation of the
trackmap".

**`VelocityX/Y` are car-local.** The SDK does not state the frame. On a local `.ibt`, over every
sample above 20 m/s:

- `hypot(VelocityX, VelocityY)` equals `Speed` (mean relative error below 0.01%).
- `VelocityX` alone is within 0.02% of `Speed`.
- `VelocityY` averages about 1.4% of `Speed`.

Only aggregate ratios were computed. So X is forward and Y is lateral (slip), and integrating a
position needs `Yaw` to rotate them into a world frame.

### What that allows

- **Track map (cars on an outline).** Feasible for every car. Place each car at its
  `CarIdxLapDistPct` along a recorded centre line. Cars side by side draw on top of one another,
  because there is no lateral position.
- **Radar.**
  - *Longitudinal* gap for every car: `((pct_car − pct_focus + 1.5) mod 1 − 0.5) × TrackLength`, at
    60 Hz, with `CarIdxTrackSurface` and `CarIdxOnPitRoad` to drop pit and off-world cars.
  - *Lateral*: none from the SDK. For the driver's car, `CarLeftRight` says how many cars are on
    each side. The radar then has to guess which of the longitudinally overlapping cars they are,
    and keep their lanes from one tick to the next.
  - irDashies, which is MIT-licensed public source, says this in so many words: "iRacing gives every
    car's lap progress but no world position, so distance is measured along the track. Per-car speed
    comes from progress deltas because the SDK reports speed for the player only."
  - irDashies lists the spotter indicator only for the player's car: "when the camera is on someone
    else, lanes come from memory and pairing alone."
- **Blind spot.** Feasible for the driver: `CarLeftRight` gives the side, and the nearest overlapping
  car by lap distance gives the longitudinal overlap. **Not feasible for another car** beyond "a car
  overlaps longitudinally", because no side information exists for anyone but the driver.

## Le Mans Ultimate

LMU publishes its own shared memory: the mapping `LMU_Data`, with no plugin needed on Windows. It is
laid out by `SharedMemoryInterface.hpp`, which Studio 397 ships in the game's
`Support\SharedMemoryInterface` folder, over the rFactor 2 `InternalsPlugin.hpp` structures.

- The game folder isn't installed on this machine, so the layout was read from the MIT-licensed
  pyLMUSharedMemory mapping, which names that header as its source.
- The `InternalsPlugin.hpp` field comments were read from the copy (© Studio 397) vendored in the
  rF2 shared memory plugin.
- Ticket #22 covers LMU's shared memory in full. This section covers only positions.

Top level (`SharedMemoryObjectOut`): `generic`, `paths`, `scoring`, `telemetry`.

| Block / field | Who | Comment (from `InternalsPlugin.hpp`) |
|---|---|---|
| `telemetry.activeVehicles`, `playerVehicleIdx`, `playerHasVehicle` | – | Index of the driver's car. |
| `telemetry.telemInfo[104].mPos` | every vehicle | "world position in meters" |
| `telemInfo[].mOri[3]` | every vehicle | "rows of orientation matrix" |
| `telemInfo[].mLocalVel`, `mLocalAccel` | every vehicle | "in local vehicle coordinates" |
| `scoring.scoringInfo.mLapDist` | – | "distance around track" (the lap length) |
| `scoring.vehScoringInfo[104].mLapDist` | every vehicle | "current distance around track" |
| `vehScoringInfo[].mPathLateral` | every vehicle | "lateral position with respect to *very approximate* \"center\" path" |
| `vehScoringInfo[].mTrackEdge` | every vehicle | "track edge (w.r.t. \"center\" path) on same side of track as vehicle" |
| `vehScoringInfo[].mPos`, `mOri`, `mLocalVel` | every vehicle | As in telemetry, at the scoring rate. |
| `vehScoringInfo[].mInPits` | every vehicle | "not always accurate for remote vehicles" |

- **Axes.** LMU's world is Y-up with the ground on the X/Z plane. TinyPedal (GPL-3.0) maps
  longitudinal to `mPos.x`, lateral to `-mPos.z` and vertical to `mPos.y`, and derives yaw from
  `mOri[2]` as `atan2(x, z)`. TinyPedal reads `telemInfo[index]` for any vehicle index, which is how
  its Radar sees other cars.
- **Update rates.**
  - `InternalsPlugin.hpp`: `UpdateScoring` arrives "approximately five times per second". The
    `UpdateTelemetry` comment gives no rate.
  - For rFactor 2's plugin interface, the rF2 shared memory plugin's README gives Telemetry at 50 FPS
    and Scoring at 5 FPS.
  - For LMU's built-in memory, the only figure found is a third-party project's claim of 100 Hz
    telemetry and 5 Hz scoring (lmu-pitwall), which is not an official source.
- **No side indicator.** `InternalsPlugin.hpp` has no equivalent of `CarLeftRight`. irDashies
  synthesises its LMU blind-spot channels (`LmuBlindSpotLeft/RightLongitudinal`,
  `LmuCarIdxRelativeLateral/Longitudinal/Heading`) from the positions.
- **No geometry.** `InternalsPlugin.hpp` exposes nothing like a waypoint, spline or outline. The only
  related field is `mPitLaneStartDist` in the track rules ("where the waypoint branch to the pits
  breaks off").

### What that allows

- **Track map.** Feasible for every car, either from `mPos` directly or from `mLapDist` along a
  recorded outline.
- **Radar.** A true top-down radar for **any** focus car. Transform each car's `mPos` into the focus
  car's frame with the focus car's `mOri`. Use telemetry for the fast path and scoring at 5 Hz as a
  fallback. `mPathLateral` gives a coarse lane as a cross-check.
- **Blind spot.** Feasible for any car, computed from those relative positions. A car-width and
  car-length threshold is needed, because LMU supplies no indicator.

## Track outlines

### Sim-provided

- **iRacing SDK:** none. The live data has no geometry and no positions.
- **iRacing Data API track assets.** `/data/track/assets` lists, per track, a `track_map` base URL
  and `track_map_layers`: `background`, `inactive`, `active`, `pitroad`, `start-finish` and `turns`
  SVGs. (Field names are from the Aydsko iRacing Data API client docs, because the official
  `/data/doc` needs a login.)
  - **Licence: none granted.** iRacing's auth documentation says "iRacing does not provide explicit
    authorization to 3rd parties". Use of the SDK, the `/data` API and their data is governed by
    the member Terms of Use, EULA and Conditions of Use, and access can be restricted at any time.
  - irDashies ships drawings derived from these SVGs, but carves them out of its MIT licence: they
    "are the property of iRacing.com Motorsport Simulations, LLC" and "may not be copied, modified,
    distributed, or used separately from this project without permission".
  - Fetching them at run time on the user's behalf needs an OAuth client (Authorization Code with
    PKCE for a distributed app), and "New OAuth client IDs are currently not being created" while
    iRacing reviews third-party use. Contact: auth@iracing.com.
  - **Not AGPL-compatible, and not redistributable.**
- **iRacing track files** (`tracks\` in the install) are proprietary binaries. Unusable.
- **LMU / rF2:** no geometry interface (see above). The track's AI waypoint data sits inside the
  game's packed content, which is proprietary.

### Recording the user's own laps

This covers every track and layout, sim-only ones included, and needs no third-party licence,
because the outline is generated on the user's machine from their own session.

- **LMU.** Sample the driver's `mPos` (X, −Z) against `mLapDist` over one clean lap, resample it to
  a fixed number of points, and close the loop. irDashies does exactly this for LMU: it records
  ground positions keyed by lap distance, rejects laps with coverage gaps or invalid laps, and
  resamples to 512 points.
  - Any car's positions could be recorded, not only the driver's, so a spectator can build the map too.
  - `mTrackEdge` and `mPathLateral` give a rough track width.
- **iRacing, live.** Dead-reckon the driver's path:
  - Rotate the car-local `VelocityX/Y` by `Yaw` and integrate at 60 Hz, or at 360 Hz from the `_ST`
    arrays.
  - Key each point by `LapDist` and close the loop at the start/finish line. Drift has to be
    corrected, for example by spreading the closure error along the lap.
  - Only the driver's car can be recorded.
- **iRacing, from `.ibt`.** Read `Lat`/`Lon` from the user's own disk telemetry for one lap, and
  project them with `TrackLatitude`/`TrackLongitude` from the session. This gives absolute,
  drift-free geometry, but only after a logged session, and it needs an `.ibt` reader.
- **Storage.** Keep outlines per sim, track ID and config in the user's data directory.
  - Whether outlines that the maintainer records may be **shipped** with reo is a separate question,
    not settled here. They are measurements, not copies of iRacing's assets, but they are measured
    from iRacing's track model under its Terms of Use.
  - Until that is answered, record per user and ship none.

### Community datasets

| Dataset | Coverage | Stated licence | Provenance | AGPL-3.0 and redistribution |
|---|---|---|---|---|
| OpenStreetMap (`highway=raceway` ways) | Most real circuits, quality varies | ODbL 1.0 | Crowd-mapped | Can be shipped **as a separate data file** beside AGPL code: "If you alter or build upon our data, you may distribute the result only under the same license", plus the attribution "Provide credit to OpenStreetMap". It cannot be relicensed under AGPL. A derived outline set stays ODbL. |
| TUMFTM/racetrack-database | 20+ F1 and DTM circuits: centre line, widths and race line, in metres | LGPL-3.0 | Centre lines "fetched as GPS points from the OpenStreetMap project"; widths from satellite images | LGPL-3.0 is AGPL-compatible as a code licence, but the data derives from OSM, so ODbL obligations very likely still apply. Treat it as ODbL. |
| bacinger/f1-circuits | F1 circuits only, GeoJSON | MIT | "Initial circuits data source" is a third-party Google My Maps map | MIT is compatible, but provenance is unclear (tracing from Google Maps imagery is restricted by Google's terms). Risky to redistribute. |

**Every real-world dataset shares the same limits:**

- It is in geographic or arbitrary metric coordinates, not sim coordinates.
- It must be aligned to the sim's lap distance (start/finish line, direction, length) before a car
  can be placed on it.
- It covers few of iRacing's or LMU's layouts, and no fictional or sim-specific ones.

They could at most seed a map before the user's first lap.

## Open questions

- **LMU telemetry rate.** The official rate of `LMU_Data` telemetry is unconfirmed: about 50 Hz is
  inherited from rF2, and 100 Hz is claimed by a third party. Measure it live, or read it from
  `SharedMemoryInterface.hpp`, under #22.
- **LMU online.** Whether `telemInfo[]` is fully populated for **remote** vehicles in an online LMU
  race, as it is offline. The header notes `mInPits` is "not always accurate for remote vehicles".
  Check this on a live online session.
- **iRacing.** Whether `CarIdxLapDistPct` for remote cars is smoothed or jumps with net updates. This
  affects radar jitter; measure it on the tape.
- **Shipping outlines.** Whether reo may ship outlines it recorded from iRacing or LMU. That is a
  Terms of Use question for iRacing and Studio 397, not answered by any public licence found.
- **iRacing forums.** The official iRacing forum SDK threads need a login, so they were not read.
  `irsdk_defines.h` was read from a vendored copy, not the forum download.

## Sources

- iRacing SDK header `irsdk_defines.h` (© 2013 iRacing.com Motorsport Simulations, BSD-3-Clause),
  read from the copy vendored in irDashies:
  <https://github.com/tariknz/irdashies/blob/main/src/app/irsdk/native/lib/irsdk_defines.h>. The SDK
  itself is distributed through the iRacing forums (login required):
  <https://forums.iracing.com/discussion/62/iracing-sdk>.
- This repository's `reo vars` and `reo session --raw` against `tapes/ai-race-10min.irdt`, a live
  shared-memory recording (variable and key names only).
- A local `.ibt` file's variable headers (names only), and aggregate velocity ratios from it.
- iRacing auth service documentation: <https://oauth.iracing.com/oauth2/book/print.html>.
- Aydsko iRacing Data API client docs, `TrackAssets` and `MapLayers`:
  <https://adrianjsclark.github.io/aydsko-iracingdata/api/Aydsko.iRacingData.Tracks.MapLayers.html>.
- irDashies track data licence and README:
  <https://github.com/tariknz/irdashies/tree/main/src/frontend/domain/track/tracks>. Its radar,
  blind-spot and LMU track-map sources are in `src/app/processors/` and `src/app/lmu/trackMap.ts`
  (public, MIT; studied, not copied).
- `InternalsPlugin.hpp` (© 2017 Studio 397), vendored in TheIronWolfModding/rF2SharedMemoryMapPlugin,
  together with that plugin's README for the rF2 refresh rates:
  <https://github.com/TheIronWolfModding/rF2SharedMemoryMapPlugin>.
- pyLMUSharedMemory (MIT), a mapping of LMU's `SharedMemoryInterface.hpp`:
  <https://github.com/TinyPedal/pyLMUSharedMemory/blob/master/lmu_data.py>.
- TinyPedal (GPL-3.0), LMU reader: <https://github.com/TinyPedal/TinyPedal/blob/master/tinypedal/adapter/lmu_reader.py>.
- lmu-pitwall (third-party claim of LMU rates): <https://github.com/Swizzjack/lmu-pitwall>.
- Mu (`.ibt` to MoTeC converter) README: <https://github.com/patrickmoore/Mu>.
- OpenStreetMap copyright and licence: <https://www.openstreetmap.org/copyright>; the
  `highway=raceway` tag: <https://wiki.openstreetmap.org/wiki/Tag:highway%3Draceway>.
- TUMFTM/racetrack-database (LGPL-3.0): <https://github.com/TUMFTM/racetrack-database>.
- bacinger/f1-circuits (MIT): <https://github.com/bacinger/f1-circuits>.
