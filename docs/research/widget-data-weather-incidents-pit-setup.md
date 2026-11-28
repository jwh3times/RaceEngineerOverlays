# Widget data: Weather, Race incidents, Pit strategy and Setup cover

Research for issue #24 on the route-to-1.0 map (#21). Question: what data do the **Weather**,
**Race incidents**, **Pit strategy** and **Setup cover** widgets (see `GLOSSARY.md`) need, and does
iRacing expose it? Le Mans Ultimate is covered only for these four widgets; #22 covers LMU in
general.

Researched 2026-11-27. This branch is never merged.

## Answer in one table

| Widget | Need | iRacing | LMU |
| --- | --- | --- | --- |
| Weather | track and air temperature, wetness, wind | **Exposed** live in telemetry | **Exposed** (scoring info) |
| Weather | forecast | **Not exposed** to the SDK. Only in the sim UI and the web Data API | **Not exposed** in shared memory |
| Race incidents | own count, session limit | **Exposed**: telemetry counts and `WeekendOptions:IncidentLimit` | No incident points; penalties and track-limit points instead |
| Race incidents | other cars' counts | **Hidden in race sessions** (own team only) except to admins; visible in non-race sessions and after the race | Per-car penalty count only |
| Race incidents | feed of the field's incidents | **Derivable, partially**: off-track, tow/reset, repair/black flags per car; no contacts, no points | Derivable from penalties, impacts are own-car only |
| Pit strategy | pit-lane time loss per track | **Derivable**: speed limit and track length in the session string; pit-lane length is not exposed, but entry/exit and time in lane are measurable for every car | Derivable the same way (`mInPits`, `mPitState`) |
| Pit strategy | tyre and fuel service times | **Not exposed** as data. Published per car class in release notes; the planned service and the active rule set are exposed | Not exposed |
| Setup cover | garage/setup screen open | **Exposed**: `IsGarageVisible` | **Derivable**: `mInRealtime`, `mOptionsLocation`, `mOptionsPage` |

## How this was checked

- **The sim's own variable headers.** Every iRacing telemetry variable carries a name, type, unit
  and description written by iRacing into shared memory. These were read with
  `reo vars --irdt tapes/ai-race-10min.irdt`, and the session string's key structure with
  `reo session --raw`. Only names, types, units and descriptions are reported here, never values
  or tape contents (`AGENTS.md` rule 7). The tape carries `TrackWetness`, `Precipitation` and
  `WeatherDeclaredWet`, so it was recorded after iRacing's 2024 Season 2 Patch 1. It has no
  `WeekendInfo:AltAssetTag` key, so it predates 2026 Season 4. It is an offline AI race, so it
  cannot show how other drivers' incident counts behave online.
- **iRacing release notes** on support.iracing.com and iracing.com, which are where iRacing
  announces SDK changes.
- **`irsdk_defines.h`**: the official SDK download is behind the iRacing forum login and no copy
  was found locally, so enum values come from `pyirsdk` (kutu/pyirsdk), a long-standing port
  that mirrors the header's enums. Treat those numbers as medium trust until checked against
  the header.
- **The iRacing forum** (forums.iracing.com) requires a login, so threads there could not be
  read directly; where a forum answer matters it is cited through the page that quotes it, and
  marked as such.
- **LMU**: no local install, so fields come from `TinyPedal/pyLMUSharedMemory`, a ctypes
  transcription of the `SharedMemoryInterface.hpp` that ships in LMU's
  `Support\SharedMemoryInterface` folder. Medium trust; #22 should confirm against the header.

## Weather

### What iRacing exposes live (telemetry)

All at the start/finish line unless noted. Descriptions are iRacing's own, from the variable
headers.

| Variable | Type, unit | Description |
| --- | --- | --- |
| `AirTemp` | float, C | Temperature of air at start/finish line |
| `TrackTempCrew` | float, C | Temperature of track measured by crew around track |
| `TrackTemp` | float, C | "Deprecated set to TrackTempCrew" |
| `TrackWetness` | int, `irsdk_TrackWetness` | How wet is the average track surface |
| `Precipitation` | float, % | Precipitation at start/finish line |
| `WeatherDeclaredWet` | bool | Steward says rain tires can be used |
| `WindVel` | float, m/s | Wind velocity at start/finish line |
| `WindDir` | float, rad | Wind direction at start/finish line |
| `RelativeHumidity` | float, % | Relative humidity at start/finish line |
| `FogLevel` | float, % | Fog level at start/finish line |
| `AirDensity`, `AirPressure` | float, kg/m^3 and Pa | at start/finish line |
| `Skies` | int | 0 clear, 1 partly cloudy, 2 mostly cloudy, 3 overcast |
| `SolarAltitude`, `SolarAzimuth` | float, rad | Sun angle above horizon, clockwise from north |

`irsdk_TrackWetness` values (from pyirsdk): 0 unknown, 1 dry, 2 mostly dry, 3 very lightly wet,
4 lightly wet, 5 moderately wet, 6 very wet, 7 extremely wet. Value 0 is a sentinel and maps to
`None` in the model (rule 1). The `%` variables are fractions to convert to `0.0..=1.0`.

Release-note history (primary):

- **2024 Season 2** added `Precipitation`, extended `WeekendInfo:TrackWeatherType`,
  `WeekendInfo:TrackSkies` and `WeekendOptions:WeatherType`/`Skies` to cover dynamic weather, and
  added `WeekendInfo:TrackPrecipitation`, the initial precipitation at the start of the session.
- **2024 Season 2 Patch 1** removed the `WeatherType` and `WeatherVersion` telemetry variables
  ("already encapsulated in the session string WeekendInfo:TrackWeatherType"), added the
  `irsdk_TrackWetness` enum on `TrackWetness`, and added `WeatherDeclaredWet` ("set to true when
  the marshal allows use of rain tires").

### Session string (`WeekendInfo`)

`TrackWeatherType` (values seen in iRacing's notes: Realistic, Static, Timeline, Unknown, plus
the older "Classic ..." values), `TrackSkies`, `TrackSurfaceTemp`, `TrackSurfaceTempCrew`,
`TrackAirTemp`, `TrackAirPressure`, `TrackAirDensity`, `TrackWindVel`, `TrackWindDir`,
`TrackRelativeHumidity`, `TrackFogLevel`, `TrackPrecipitation`, and under `WeekendOptions`:
`WeatherType`, `Skies`, `WindDirection`, `WindSpeed`, `WeatherTemp`, `RelativeHumidity`,
`FogLevel`, `TimeOfDay`, `Date`. These are the session's starting or configured conditions, not a
forecast. Values carry unit suffixes as strings (`C`, `m/s`, `rad`, `%`, and `km/h` for
`WeekendOptions:WindSpeed`), which the adapter parses to SI.

### The forecast

**Not exposed to the SDK.** Evidence:

- The tape's session uses forecasted weather (`TrackWeatherType: Realistic`) and its session
  string has no forecast section; every weather key is a current or initial value.
- No iRacing release note from 2024 Season 2 (when forecasts arrived, with the "View Forecast"
  button) through 2026 Season 4 adds a forecast to telemetry or the session string. The weather
  notes since then are about the in-sim forecast display and the forecast model.
- A SimHub forum post of 2024-03-04 reports iRacing's answer in the iRacing forum thread "Will 3rd
  party apps/spotters have access to forecast data?" (forums.iracing.com/discussion/57318): "As of
  today, they don't publish that data via the API/SDK." The iRacing thread itself is behind a
  login and was not read, so this is second-hand.

**Where a forecast does exist:** iRacing's web Data API (members-ng, authenticated) carries a
`weather` object on series schedules with `forecast_options`, `weather_summary` and a
`weather_url` that resolves to hourly key frames (`air_temp`, `precip_chance`, `precip_amount`,
`cloud_cover`, `wind_speed`, `wind_dir`, `rel_humidity`, `pressure`, `time_offset` from the race
start). This is documented by the Aydsko.iRacingData library, not by iRacing directly (iRacing's
own `/data/doc` needs a login). The 2024 S2 Patch 1 notes say each official series uses one
forecast per race week, viewable in the Race Schedule, and that the real weather "may shift as time
progresses". Using it would mean OAuth against the member's account and a network call, which is a
product decision, not a data question.

**Derivable:** a short-range trend (rain arriving or drying) from the rate of change of
`Precipitation`, `TrackWetness` and `Skies`; that is a trend, not a forecast.

## Race incidents

### The driver's own count and the limit: exposed

| Source | Name | Meaning |
| --- | --- | --- |
| telemetry | `PlayerCarMyIncidentCount` | own incident count for this session |
| telemetry | `PlayerCarDriverIncidentCount` | the current team driver's count |
| telemetry | `PlayerCarTeamIncidentCount` | the whole team's count |
| telemetry | `PlayerIncidents` | bitfield `irsdk_IncidentFlags`: "Log incidents that the player received" |
| session | `WeekendInfo:WeekendOptions:IncidentLimit` | maximum incidents before ejection |
| session | `WeekendOptions:IncidentWarningInitialLimit`, `IncidentWarningSubsequentLimit` | thresholds for incident penalties (key names; meaning inferred, not documented) |
| session | `DriverInfo:DriverIncidentCount`; `Drivers[].CurDriverIncidentCount`, `Drivers[].TeamIncidentCount` | per-entry counts (see visibility below) |

The three `PlayerCar...IncidentCount` variables and the `IncidentLimit` session-string entry
arrived in iRacing's 2016 Season 3 release ("the maximum incidents you can receive before being
ejected from the session"). In a solo session all three counts are equal.

`PlayerIncidents` decodes per incident (pyirsdk): the low byte is the report (1 out of control,
2 off track, 3 off track ongoing, 4 contact with world, 5 collision with world, 6 collision with
world ongoing, 7 contact with car, 8 collision with car) and the high byte the penalty (0x100 0x,
0x200 1x, 0x300 2x, 0x400 4x). iRacing's support article "Incident Points" gives the scoring:
0x light contact, 1x off track, 2x wall contact or loss of control, 4x heavy contact on paved
tracks (2x on dirt), and in quick succession only the highest counts. That is enough for a feed of
the **driver's own** incidents with type and points.

The limit can be absent or unlimited in practice sessions; the adapter should treat a missing or
non-numeric `IncidentLimit` as `None`. This was not verified beyond the one tape, where it is
numeric.

### Other cars' counts: hidden during a race

iRacing's 2016 Season 3 release notes, describing the INCS column: "In race sessions, everyone is
only able to see their own team's incident point counts." Once the race is complete, or in a
non-race session, everyone sees everyone's counts; admins see everyone's counts during a race;
the same restrictions apply to replays.

Those notes describe the UI. The session string's per-driver `CurDriverIncidentCount` and
`TeamIncidentCount` are understood to follow the same rule, and a third-party SimHub plugin's data
notes (simsteward/simhub-plugin, `docs/IRACING-DATA-AVAILABILITY.md`) report no live per-car
count for other drivers and that `SessionInfo:Sessions[].ResultsPositions[].Incidents` does not
move until results are official. **Unconfirmed against a live online race**: the tape is an
offline AI race. A live check in an official race, a non-race session and as a hosted-session
admin would settle it.

### A feed of the field's incidents: partly derivable

Telemetry gives, for every car index:

- `CarIdxTrackSurface` (`irsdk_TrkLoc`: -1 not in world, 0 off track, 1 in pit stall,
  2 approaching pits, 3 on track): an off-track excursion, and -1 mid-session for a tow or reset.
- `CarIdxSessionFlags` (`irsdk_Flags`): black (0x010000), disqualify (0x020000), furled
  (0x080000) and repair, the meatball (0x100000).
- `CarIdxFastRepairsUsed`: a fast repair means the car was damaged.
- Speed (from `CarIdxLapDistPct` over time) and `CarIdxLapCompleted`: a car stopped or slow on
  track.

Not observable for other cars: contacts, which cars were involved, and points. In non-race
sessions and after the race, the per-driver counts in the session string can be diffed when it
updates. Admins can see counts live.

## Pit strategy

### Time lost in the pits: derivable, not given

| Source | Name | Notes |
| --- | --- | --- |
| session | `WeekendInfo:TrackPitSpeedLimit` | string with unit, `"<n> kph"`: parse to m/s |
| session | `WeekendInfo:TrackLength` | `"<n> km"` |
| session | `WeekendInfo:TrackNumPitStalls` | |
| session | `DriverInfo:DriverPitTrkPct` | the player's pit stall as a fraction of the lap |
| telemetry | `OnPitRoad`, `CarIdxOnPitRoad` | "on pit road between the cones", every car |
| telemetry | `CarIdxTrackSurface` | 1 in pit stall, 2 approaching pits |
| telemetry | `PlayerCarInPitStall`, `PitstopActive` | the player is in the stall, being serviced |
| telemetry | `CarIdxLapDistPct`, `CarIdxEstTime`, `CarIdxLastLapTime` | position and timing per car |
| telemetry | `dcPitSpeedLimiterToggle` | the player's limiter |

**Pit-lane length is not exposed**, nor are the pit entry and exit positions. They are
measurable: the `CarIdxLapDistPct` at which any car's `CarIdxOnPitRoad` turns on and off gives the
entry and exit points, and the elapsed time between them, less the stationary time
(`CarIdxTrackSurface` = in pit stall with no movement), gives the drive-through time at the limit.
Pit loss is that drive-through time less the time the same stretch of track takes at racing speed,
which `CarIdxEstTime` (iRacing's estimated time to reach a point on track) or the driver's own laps
give. Every car's stops can be measured, so one session gives many samples; storing a measured
value per track and configuration is the practical route.

Community data: no maintained per-track pit-loss table for iRacing was found. HH Timing publishes
per-track pit-loss settings for IndyCar, which is a different series and timing tool. The iRacing
Data API's lap data (authenticated, post-race) marks laps on which a car pitted, which could seed
values but needs the member's OAuth.

### Service times: published, not exposed

The SDK does not report how long a tyre change or refuel takes. iRacing publishes it per car
class in release notes, and it changes from season to season:

- **2023 Season 1**: for GT3-class cars and the Dallara P217, pit stops changed to take fuel and
  tyres simultaneously instead of sequentially ("Full refuel time is 34 seconds" for the P217).
- **2026 Season 3**: for the default regulations, GT3, GT4, GTP and TCR went back to sequential
  (fuel then tyres). The IMSA regulations keep simultaneous service; NEC is simultaneous at a much
  slower fuel rate; DTM has extra crew for very fast tyre changes. "If no value is present, the
  default regulations are in effect."
- **2026 Season 4**: most classes now use 2 wheel guns, so any pair of tyres changes at once;
  repairs cannot run during fueling under the default rules. Per-class figures include GT3 "Total
  time to change 4 tires is unchanged for all rulesets (20 seconds, except DTM where it is 7
  seconds)", and other classes quote "A full tank of fuel will take 60 seconds (120 seconds for NEC
  rules) and tire changes approximately 40 seconds" and "34 seconds to fill the tank". The same
  release added the session-string value **`WeekendInfo:AltAssetTag`** (blank, IMSA, NEC, NOTC,
  PMNA, INDY, NOABSTC, NOABS, ROOKIE, WSC2017, DTM), which names the active rule set.

What the SDK does expose is the **planned service** and its progress:

- `PitSvFlags` (`irsdk_PitSvFlags`: 0x01 LF, 0x02 RF, 0x04 LR, 0x08 RR tyre change, 0x10 fuel,
  0x20 tear-off, 0x40 fast repair), `PitSvFuel`, `PitSvLFP`..`PitSvRRP`, `PitSvTireCompound`,
  and the `dp...` pit-service variables (`dpFuelAddKg`, `dpLFTireChange`.., `dpFastRepair`,
  `dpFuelAutoFillEnabled`).
- `PlayerCarPitSvStatus` (`irsdk_PitSvStatus`: 0 none, 1 in progress, 2 complete, 100-105 the
  car is badly placed or the damage cannot be fixed).
- `PitRepairLeft`, `PitOptRepairLeft`: seconds of mandatory and optional repair left.
- Tyre sets: `TireSetsUsed`/`TireSetsAvailable` and the per-corner and per-axle variants
  (255 = unlimited, a sentinel), `PlayerCarDryTireSetLimit`, `FastRepairAvailable`,
  `FastRepairUsed` (255 = unlimited), and `WeekendOptions:FastRepairsLimit`.
- `FuelLevel`, `DriverInfo:DriverCarFuelMaxLtr`, `DriverCarMaxFuelPct`.

So the stationary time can be estimated as rule set (`AltAssetTag`) × planned service
(`PitSvFlags`, fuel to add) × per-class rates, with the rates kept as data and refreshed from
release notes. They can also be measured on the player's own stops (`FuelLevel` rising during
`PitstopActive` gives the fill rate). Sequential versus simultaneous service changes the total,
so the rule set matters.

## Setup cover

**Exposed.** `IsGarageVisible` (bool): "1=Garage screen is visible". The garage screen is where
iRacing shows the car setup, so the cover shows while it is true. Related: `IsInGarage` ("car in
garage physics running"), `IsOnTrack`, `IsOnTrackCar`, and `CamCameraState`, whose
`irsdk_CameraState` bit 0x0001 is `IsSessionScreen`. The session string also carries the setup
itself under `CarSetup` (with `UpdateCount`) and `DriverInfo:DriverSetupName`,
`DriverSetupIsModified`, `DriverSetupLoadTypeName` and `DriverSetupPassedTech`, which is why
streamers hide it. Not verified here: whether `IsGarageVisible` is set when the garage is
opened while spectating or in a replay.

## LMU, for these four widgets

From the `pyLMUSharedMemory` transcription of LMU's `SharedMemoryInterface.hpp` (the `LMU_Data`
mapping). #22 should confirm against the header itself.

- **Weather: exposed, no forecast.** `ScoringInfo`: `mAmbientTemp` and `mTrackTemp` (C), `mWind`
  (a 3-vector), `mRaining` (0..1), `mDarkCloud` (0..1), `mCloudCoverage` (0 clear .. 10 storm),
  `mMinPathWetness`, `mAvgPathWetness`, `mMaxPathWetness` (0..1), `mTrackGripLevel`. No forecast
  field.
- **Race incidents: no incident points.** Per vehicle, `mNumPenalties` (outstanding penalties),
  `mFlag`, `mFinishStatus`; own car only, `mTrackLimitsSteps` with
  `mTrackLimitsStepsPerPoint`/`mTrackLimitsStepsPerPenalty`, `mLapInvalidated`, and impacts
  (`mLastImpactET`, `mLastImpactMagnitude`). The Race incidents widget on LMU would show penalties
  and track-limit points, or say it is unavailable.
- **Pit strategy: derivable.** Per vehicle, `mInPits` (between pit entrance and exit; may be
  inaccurate for remote cars), `mPitState` (0 none, 1 request, 2 entering, 3 stopped, 4 exiting),
  `mPitLapDist` (the pit location), `mNumPitstops`, `mInGarageStall`. No pit speed limit, no pit
  lane bounds and no service times.
- **Setup cover: derivable.** `ScoringInfo:mInRealtime` is false at the monitor (and in the garage,
  per LmuOverlay's notes); `ApplicationState:mOptionsLocation` (0 main UI, 1 track loading,
  2 monitor, 3 on track) and `mOptionsPage` (the name of the open options page) could tell the setup
  page apart. Which page names mean "setup" needs a live check.

## Open questions

1. Do `Drivers[].CurDriverIncidentCount` and `TeamIncidentCount` read 0 for other cars during an
   online race, and do they fill in for an admin and after the checkered flag? Needs a live check.
2. What does `IncidentLimit` hold in a session without a limit?
3. The exact `irsdk_defines.h` enum values, checked against the official SDK download rather than
   pyirsdk.
4. Whether `IsGarageVisible` is set while spectating or in a replay.
5. LMU: which `mOptionsPage` values are the setup screen.

## Sources

Primary:

- iRacing variable headers (name, unit, description) as read by `reo vars` and the session-string
  keys as read by `reo session --raw`, from a local tape (contents not reproduced).
- iRacing, 2016 Season 3 Release Notes (2016-06-03):
  https://www.iracing.com/2016-season-3-release-notes/
- iRacing, 2024 Season 2 Release Notes [2024.03.05.01]:
  https://support.iracing.com/support/solutions/articles/31000172630-2024-season-2-release-notes-2024-03-05-01-
- iRacing, 2024 Season 2 Patch 1 Release Notes [2024.03.13.01]:
  https://support.iracing.com/support/solutions/articles/31000172730-2024-season-2-patch-1-release-notes-2024-03-13-01-
- iRacing, 2023 Season 1 Release Notes [2022.12.06.01]:
  https://support.iracing.com/support/solutions/articles/31000168823-2023-season-1-release-notes-2022-12-06-01-
- iRacing, 2026 Season 3 Initial Release Notes [2026.06.09.01]:
  https://support.iracing.com/support/solutions/articles/31000179016-2026-season-3-initial-release-notes-2026-06-09-01-
- iRacing, 2026 Season 4 Initial Release Notes [2026.09.09.01]:
  https://support.iracing.com/support/solutions/articles/31000179517-2026-season-4-initial-release-notes-2026-09-09-01-
- iRacing support, "Incident Points":
  https://support.iracing.com/support/solutions/articles/31000173709-incident-points
- iRacing forum, "Will 3rd party apps/spotters have access to forecast data?" (login required, not
  read): https://forums.iracing.com/discussion/57318

Secondary (ports and transcriptions of primary material, medium trust):

- kutu/pyirsdk, `irsdk.py` (enum values mirroring `irsdk_defines.h`):
  https://github.com/kutu/pyirsdk/blob/master/irsdk.py
- TinyPedal/pyLMUSharedMemory, `lmu_data.py` (transcription of LMU's `SharedMemoryInterface.hpp`):
  https://github.com/TinyPedal/pyLMUSharedMemory/blob/master/lmu_data.py
- Aydsko.iRacingData API docs, `Series.Weather` and `Series.WeatherForecast` (iRacing Data API
  forecast fields): https://adrianjsclark.github.io/aydsko-iracingdata/api/Aydsko.iRacingData.Series.WeatherForecast.html

Low trust (community reports, used only where marked):

- SimHub forum, "iRacing - Rain Update: more details and black box info" (2024-03-04):
  https://www.simhubdash.com/community-2/simhub-support/iracing-rain-update-more-details-and-black-box-info/
- simsteward/simhub-plugin, `docs/IRACING-DATA-AVAILABILITY.md`:
  https://github.com/simsteward/simhub-plugin/blob/main/docs/IRACING-DATA-AVAILABILITY.md
- Pagh/LmuOverlay README ("`mInRealtime` is 0 in the garage"): https://github.com/Pagh/LmuOverlay
- HH Timing, IndyCar recommended settings (per-track pit loss, IndyCar only):
  https://help.hhtiming.com/series-specific-info/indycar/recommended-settings/
