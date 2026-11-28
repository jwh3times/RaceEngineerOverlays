# Race Engineer Overlays

The language of sim-racing overlays: the information shown to a driver on top of the sim while
they race, and the sims it comes from.

## Language

### Sims and their data

**Sim**:
A racing simulator reo reads from, such as iRacing or Le Mans Ultimate.
_Avoid_: game, title

**Tape**:
A recording of a sim's telemetry and session data that can be replayed in place of the live sim.
_Avoid_: replay (iRacing's own replay is a different thing), log, dump

### Race timing

**Gap**:
The time a car is behind the leader of its own class.
_Avoid_: delta (that is the Delta widget's term), behind

**Interval**:
The time a car is behind the car directly ahead of it in its own class.
_Avoid_: gap (for the car ahead), split

### Widgets

**Widget**:
One self-contained display of race information, such as the Relative or the Fuel widget, placed
on screen on its own.
_Avoid_: overlay (for one display), HUD element, panel

**Overlay**:
What reo is as a whole: the set of widgets drawn on top of the sim while racing.
_Avoid_: using it for a single widget

**Standings**:
The whole field in race order, grouped by class in a multiclass race, with each car's
manufacturer emblem, gap and interval, last and best lap, recent laps compared with the driver's
own, stint length, tyre compound and positions gained or lost since the start.
_Avoid_: leaderboard, timing tower

**Relative**:
The cars physically nearest the driver on track, ahead and behind, with the time to each and
whether they are lapping or being lapped.
_Avoid_: proximity list

**Track map**:
An outline of the track with every car's position on it, coloured by class.
_Avoid_: minimap

**Radar**:
A top-down view of the cars within a few car lengths of the driver.
_Avoid_: proximity radar

**Blind spot**:
A marker at the edge of the screen while a car is alongside the driver.
_Avoid_: spotter (the sim's spotter is a voice), radar

**Weather**:
Track and air temperature, track wetness, wind and the forecast.

**Flag**:
The flag currently shown to the driver, displayed prominently.

**Inputs**:
Throttle, brake, clutch and steering as a trace and as bars, with gear and speed.
_Avoid_: pedals, telemetry (too broad)

**Delta**:
The driver's live time difference to a reference lap, shown as a bar.
_Avoid_: delta bar (as the term), lap delta

**Fuel**:
Fuel used per lap, laps of fuel remaining, and the fuel needed to finish.
_Avoid_: fuel calculator

**Pit strategy**:
The pit window, the number of stops, the estimated time lost in the pits and where the driver
rejoins. Fuel amounts belong to the Fuel widget.
_Avoid_: strategy (alone), pit calculator

**Race incidents**:
The driver's incident count against the session's limit, and a feed of incidents involving
other cars.
_Avoid_: incidents (alone), x-count

**Setup cover**:
A panel that hides the car setup screen from viewers of a stream.
_Avoid_: setup hider, privacy screen

### Configuration

**Edit mode**:
The state in which the driver moves, resizes and shows or hides widgets on screen, instead of
racing with them.
_Avoid_: layout mode, settings

**Layout**:
Where each widget sits and how large it is, saved as a whole.

**Theme**:
The look applied to every widget: colours, fonts and shapes.
_Avoid_: skin, style

**Profile**:
The layout and theme used for one sim.
_Avoid_: preset
