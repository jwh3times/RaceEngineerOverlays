# Competitor Standings widgets and manufacturer emblems

Research for #25 on the reo 1.0 map (#21). Researched 2026-11-27 from public material only. No
code or assets were copied. Where a product's own material says nothing, the table says "not
documented". That means the public material is silent, not that the product lacks the feature.

## Question

1. What columns and options does each product's Standings widget offer? Covers RaceLab, irDashies,
   Kapps and iOverlay.
2. How does each one show manufacturer emblems? Does it ship them, download them, ask the user for
   them, or use text? Does it publish a licence or permission statement?
3. What trademark guidance applies to an open-source app that shows car makers' logos?

## Short answer

- **Common columns:** position, car number, driver name, class colour or class grouping, gap and
  interval, last and best lap, iRating and licence badges, a car brand (emblem) column, positions
  gained, and pit status. Every product that documents its Standings offers these.
- **Distinctive columns:**
  - RaceLab has tyre history (laps on the current tyre), stint badges, top speed and last-lap top
    speed, an average of the last 5 laps, an off-track column and "visualized laptimes".
  - irDashies has lap-time deltas over the last 1 to 5 laps, an average lap over 1 to 100 laps,
    iRating change, country flags, driver tags, push-to-pass, LMU virtual energy and track limits.
    Its class headers carry an SoF and per-manufacturer car counts.
  - iOverlay keeps a separate column setup for race, qualifying and practice, and its header shows
    whatever values you turn on.
  - Kapps documents almost nothing about its own columns. Its development is suspended.
- **Emblems:**
  - RaceLab, irDashies and iOverlay all ship bundled manufacturer logos and add new ones per
    release as the sims add cars.
  - None of them downloads logos or asks the user to supply them.
  - None of them publishes a licence, a permission, or a "not affiliated" statement about car
    makers' logos.
  - irDashies is the only one with public source. It commits the logo PNGs, and composed sprite
    sheets built from them, into its MIT repository with no carve-out for the marks.
  - No public evidence was found that Kapps shows emblems at all.
- **Trademark position:** naming a car maker in text to say which car a driver is in is the
  classic nominative or referential use, and is defensible in both the US and the EU.
  - Reproducing the maker's stylised logo is weaker ground. The leading US car case (*Toyota v.
    Tabari*) agreed that the stylised Lexus mark and "L" logo were "more use of the mark than
    necessary". *Volkswagenwerk v. Church* counted it in the repairer's favour that he did not
    display the encircled VW emblem.
  - Makers' own notices (Porsche) forbid any use of their marks, crest included, without written
    consent, "except as permitted by applicable laws".
  - Shipping the logo files inside an AGPL repository would also suggest they are AGPL-licensed,
    which they are not.
  - The text abbreviation reo uses now is the safe default. Logos would need either the maker's
    permission or user-supplied files.

## Comparison of Standings columns and options

Key:

- **Yes**: the product's own material documents it.
- **Yes (community)**: only third-party material documents it.
- **Not documented**: no public source found either way.

Section numbers refer to the source list at the end.

| Column or option | RaceLab | irDashies | Kapps | iOverlay |
|---|---|---|---|---|
| Position, car number, driver name | Yes [R1] | Yes [D1] | Yes (community) [K3] | Yes (implied by "21 driver columns") [O1] |
| Class grouping / class header | Yes. Class name, class SoF, driver count and class driver count options; multiclass ordering [R2] | Yes. Class headers with estimated laps, compact SoF and optional manufacturer counts; iRacing's class order or class-leader order [D1] | SoF shown [K3] | Yes. Class header with a "fill line" setting and a "YOUR CLASS" label [O2] |
| Gap | Yes. Decimals 0 to 3 [R3] | Yes. Precision 1 to 3, also in practice and qualifying [D1] | Not documented | Yes [O1] |
| Interval | Yes. Decimals 0 to 3 [R3] | Yes. Precision 1 to 3 [D1] | Not documented | Not documented (the column list is not public) |
| Relative timing column | Yes ("relative timing" column) [R4] | No (a separate Relative widget) | Not documented | Not documented |
| Last lap / best lap | Yes, with decimal and drop-minute settings [R1][R5] | Yes, several time formats [D1] | Last lap built in; fastest lap added by a community patch [K3][K4] | Not documented |
| Lap history / recent laps | "Delta lap time" column; "visualized laptimes" per session type [R6][R7] | Lap time deltas over 1 to 5 laps [D1] | Not documented | Not documented |
| Average lap | "Average last 5 lap time" [R8] | 1 to 100 laps, can exclude in-laps and out-laps [D1] | Not documented | Not documented |
| Stint length | Stint indicators and pit/stint badges [R6][R9] | Pit lap number or laps since the last pit [D1] | Not documented | Not documented |
| Tyres | Tyre history, with laps completed on the tyre [R9][R10] | Tyre compound [D1] | Not documented | Not documented |
| Positions gained since start | "Positions changed" column, with animation on position change [R11][R7] | Position-change arrows against the qualifying grid [D1] | Not documented | "Positions gained" [O1] |
| Manufacturer emblem | Yes ("car brands", with colour options) [R7][R12] | Yes, can hide in single-make series [D1][D2] | Not documented | Yes ("car brand") [O1][O2] |
| Pit status / badges | Pit/stint badges, OUT, TOW, post-pit indicator [R6][R3][R13] | DNF, TOW, OUT, PIT, last pit lap, meatball, black flag, slowdown; optional pit time [D1] | Not documented | Not documented |
| iRating / licence / safety rating | iRating, iRating gain/loss, SR, WC/PRO licence badge [R14][R15] | Licence and rating badges, full iRating, iRating change with optional estimates [D1] | Not documented | iRating and its gain, licence [O1][O3] |
| Country flag | Driver flags in all sims; nationality and club name [R16][R17] | Country flags, 250+ countries [D1] | Not built in; a community patch adds it [K3][K4] | Not documented |
| Team name | Team and driver name in team races [R18] | Team name, with team or driver as subtext [D1] | Not documented | Not documented |
| Push-to-pass | Yes [R7][R19] | Yes [D1] | Not documented | Not documented |
| Off-track | Off-track column [R20] | Row highlight on off-track; LMU track-limits points [D1] | Not documented | Not documented |
| Top speed | Top speed; last-lap top speed [R7][R21] | Top speed in the header/footer bar only [D1] | Not documented | Not documented |
| Sim-specific extras | Not documented | LMU virtual energy; LMU ranks and penalties [D1][D3] | Not documented | Not documented |
| Driver tags / notes | Driver tags and tag notes [R1][R13] | Driver tags with presets, custom groups and labels [D1] | Not documented | Driver tags in the name or car-number columns [O2] |
| Column order | Drag to reorder [R22] | Custom display order for all columns [D1] | Drag-and-drop column editor (community description) [K3][K4] | Not documented |
| Per-session setups | Visualized laptimes per practice/qualifying/race [R7] | Per-session visibility [D1] | Not documented | Separate setups for race, qualifying and practice [O1] |
| Which drivers are shown | "Minimum visible drivers" [R13] | Drivers around you, other classes, minimum in class, top N, divider [D1] | Not documented | Up to 64 ahead and behind, and 128 at the top [O2] |
| Header / footer values | Not documented | Header and footer bars with ~20 items (session, clock, incidents, weather, fuel and more) [D1] | Headers toggle (community) [K4] | Header values you choose [O1][O2] |
| Compact mode / scaling | Themes (Lumina, classic) [R22] | Compact, Ultra; 50 to 200% scale [D1] | Font-size setting [K2] | Not documented |
| Ordering in practice and qualifying | Best lap (8.7.0) [R3] | Best lap (stated in the LMU notes) [D3] | Not documented | Not documented |

### How each product describes its Standings

- **RaceLab** (Pace Engineering GmbH, Switzerland; free Basic tier and a paid Pro tier; eight
  sims). The product site only names the Standings widget [R1]. Its column set is documented only
  through five years of release notes, cited above, and there is no reference page listing the
  columns. The Standings was rewritten in v1.3.10 (May 2021) [R7].
- **irDashies** (MIT, github.com/tariknz/irdashies). The README has the fullest public feature
  list of the four [D1]. The settings section names 23 selectable columns [D2]:
  - Position, Car Number, Country Flags, Driver Name, Team Name, Driver Tag, Pit Status
  - Car Manufacturer, Tire Compound, Rating, iRating Change, Position Change, Push to Pass
  - Gap, Interval, Best Time, Last Time, Average Lap Time, Lap Time Deltas, Lap Count
  - LMU Virtual Energy, Track Limits
- **Kapps** (kutu, kapps.kutu.ru; activated with a Twitch subscription to the author). The site
  advertises "New Standings V2" and "Classic Standings" and says that "Kapps development is
  suspended" [K1]. The FAQ covers only hiding and sizing the widget [K2]. The column details above
  come from two community patch READMEs:
  - [K3] and [K4] describe the existing column editor, the Last Lap precision options, Show
    Headers and SoF.
  - Both say Kapps never shipped country flags or a fastest-lap column.

  These READMEs are third-party descriptions, not Kapps' own.
- **iOverlay** ("Joeri app development"; Standings is free; iRacing only). Its site promises "21
  driver columns: iRating and its gain, license, car brand, positions gained, gap and more" with
  separate setups per session type [O1]. The full list of 21 is not public. The FAQ and the
  release notes add the rest [O2][O3].

## Manufacturer emblems

| | RaceLab | irDashies | Kapps | iOverlay |
|---|---|---|---|---|
| Shows emblems | Yes | Yes | No evidence found | Yes |
| Where the logos come from | Bundled; each release adds missing brands (Formula Vee, Buick, Acura, Vauxhall, Caterham; LMU Genesis, ADESS, Isotta Fraschini; RaceRoom brands) [R12][R23][R24] | Bundled and committed in the repo: one PNG per brand in `tools/car_logos/logos/`, composed into a sprite sheet [D4][D5] | Not documented | Bundled; "RAM car logo added" in v2.0.1 [O2] |
| Unknown cars | A fallback logo was shown before brand-specific ones were added (e.g. the RaceRoom logo) [R24] | No emblem for an unknown marque; a reserved "unknown" cell [D4][D6] | Not documented | "the Graphite theme shows a generic logo for cars it does not know" [O2] |
| Style options | "Colored car brands" option; "car brands on standings with color options" [R7][R25] | Hide in single-make series; manufacturer counts in the class header [D1] | Not documented | Not documented |
| Downloads or user-supplied | No (release-bundled) | No (build-time; the LMU sheet is built from a local folder the developer supplies, and the composed sheet is committed) [D7] | Not documented | No (release-bundled) |
| Licence or permission statement for the marks | None found on the site or in the terms [R26] | None. The repository LICENSE is plain MIT with no mention of logos or trademarks, and no README disclaimer [D8] | None. The brand page covers only Kapps' own logo [K5]; the terms carry a generic IP clause [K6] | None on the site or in the FAQ [O1][O3] |

### How irDashies does it (public source, MIT)

Described from the repository. No code is reproduced here.

- **Source files.**
  - One PNG per marque lives in `tools/car_logos/logos/`: about 40 marques, plus series marks
    such as FIA, SCCA, Skip Barber and iRacing.
  - A Python/Pillow script composes them into a grid sprite sheet of 128 px cells. It writes the
    sheet to `src/frontend/assets/img/car_manufacturer.png`, and a TypeScript table of grid
    positions beside it.
  - Cell (0,0) is reserved for "unknown" [D4].
- **Mapping.**
  - A hand-maintained table maps each iRacing car ID to a display name and a manufacturer key,
    and the key must match a logo filename [D4].
  - Every new iRacing season needs a PR that adds logos and IDs. For example, irDashies PR #729
    added four Season 4 cars [D9].
- **LMU.**
  - A second sheet with 64 px cells is built by an Electron script from a folder of ~512 px logos
    on the developer's machine.
  - It is kept separate because LMU has marques that iRacing lacks: Oreca, Duqueine, Ginetta,
    Glickenhaus, Vanwall and Isotta Fraschini [D7].
  - LMU cars get synthetic IDs, resolved from the vehicle name, and an unmatched marque draws
    nothing [D6].
- **Rendering.** One CSS background-position per row. Each emblem is one `em` square, scaled by
  1.25 [D6].

What this means for reo: neither sim's data carries a manufacturer field that irDashies uses. It
derives the marque from a car-ID table (iRacing) or from the vehicle name (LMU), so an emblem
feature needs a maintained car-to-marque table whatever the art source. The same table would also
drive a text abbreviation. This is an inference from the irDashies design, not checked against
reo's model.

## Trademark guidance

Not legal advice. These are the primary authorities an open-source project would be measured
against.

### United States: nominative fair use

- **The test.** *New Kids on the Block v. News America Publishing*, 971 F.2d 302, 308 (9th Cir.
  1992), as restated in *Toyota Motor Sales v. Tabari*, 610 F.3d 1171 (9th Cir. 2010) [T1]. A use
  that names another's product is fair unless:
  1. the product was "readily identifiable" without the mark;
  2. the defendant "used more of the mark than necessary"; or
  3. the defendant "falsely suggested he was sponsored or endorsed by the trademark holder".
- ***Tabari* is about a car maker's logo.** The court said: "Toyota suggests that use of the
  stylized Lexus mark and 'Lexus L' logo was more use of the mark than necessary and suggested
  sponsorship or endorsement by Toyota. This is true: The Tabaris could adequately communicate
  their message without using the visual trappings of the Lexus brand." It added that "those
  visual cues might lead some consumers to believe they were dealing with an authorized Toyota
  affiliate" [T1]. The word "Lexus" in plain text was the defensible part.
- ***Volkswagenwerk v. Church*, 411 F.2d 350 (9th Cir. 1969)** [T2].
  - An independent repairer "may advertise to the public that he repairs appellant's cars".
  - The court weighed in his favour that he "did not use Volkswagen's distinctive lettering style
    or color scheme, nor did he display the encircled 'VW' emblem".

### European Union: referential use

- ***BMW v Deenik*, C-63/97, judgment of 23 February 1999 (ECLI:EU:C:1999:82)** [T3].
  - The maker "may not prohibit a garage business from using the BMW trade mark ... for the
    purpose of informing the public that it carries out repairs and maintenance of BMW cars".
  - That protection is lost if the use "may create the false impression that there is a
    commercial connection between the garage and the trade mark proprietor".
- **Statute.** The same principle is now written into Article 14(1)(c) of Directive (EU)
  2015/2436, use "for the purpose of identifying or referring to goods or services as those of
  the proprietor", subject to honest practices (Article 14(2)). EUR-Lex did not render for this
  research, so this rests on the directive's citation rather than a fetched quote [T4].

### What car makers publish

- **Porsche.** The Porsche Newsroom legal notice lists "Porsche, Porsche Crest, Porsche Design,
  911, Carrera, Targa, Tiptronic" and "other Porsche product names, model numbers, logos". It says
  "You are prohibited from using any of the marks appearing on this website without the express
  prior written consent of PAG", "except as permitted by applicable laws" [T5].
- **Porsche UK.** The UK notice says the marks "may not be used for any purpose whatsoever without
  prior written authorisation" and that "Porsche will vigorously prosecute any unauthorized use"
  [T6].
- **Volvo.** Volvo's legal notice says the "trademarks, company logos and other emblems" on its
  site are subject to Volvo's trademark rights [T7].
- **No permissive policy found.** No car maker was found to publish a policy that permits app
  developers to use their logos.
- **iRacing's licence.** iRacing's in-sim use of the makers' logos rests on its own licences, and
  those licences do not pass to third-party overlay makers. This is inferred: no iRacing
  statement on it was found.

### Where the logos are hosted

- **GitHub.** GitHub's trademark policy treats as a violation the use of a company's "name, logo,
  or other trademark-protected material in a manner that may mislead or confuse others with
  regard to its brand or business affiliation". It acts on reports, and lets an account holder
  resolve unintentional confusion first [T8].
- **How an open-source logo project handles it.** Simple Icons is an open-source (CC0) brand-icon
  project [T9]. Its disclaimer says:
  - the project's licence does not make the icons CC0;
  - it is not responsible for legal action from brands, and users must get any permission they
    need;
  - users should follow each brand's guidelines;
  - brands can ask for removal.

  This is how an open-source project ships marks it does not own: with a disclaimer, and with
  removal on request.

### Position for reo

1. **Text is safe.** Naming the marque in text (an abbreviation or the name) to show which car a
   driver is in meets all three *New Kids* factors and *Deenik*'s honest-practices test, as long
   as reo does not suggest it is endorsed.
2. **Logos are the weak point.** Every precedent above treats the stylised logo as going beyond
   what is needed to identify the car. Shipping the art also needs a copy of each logo file,
   which raises copyright as well as trademark questions.
3. **Shipping logos in the repository would mislabel them.** An AGPL-3.0 repository that
   contained the logo files would imply they are AGPL-licensed. irDashies does exactly this under
   MIT, without a carve-out.
4. **Options if emblems are wanted later**, roughly in order of legal comfort:
   - Get each maker's written permission (impractical for ~50 marques).
   - Let the user supply the logo files from a folder, with reo shipping none (the art then never
     passes through reo's distribution).
   - Ship marks with a clear "not affiliated, marks belong to their owners" notice and a removal
     path, as the commercial tools effectively do, though without publishing a notice. That last
     option relies on nobody objecting, not on a right.

## What could not be established

- **Kapps' own column list, and whether it shows emblems at all.** The official site and FAQ say
  nothing. The Twitch video linked as the "Standings V2" showcase [K1] could not be read, so the
  Kapps column is mostly "not documented".
- **iOverlay's full list of 21 columns.** It is not published, and only five are named [O1].
  iOverlay's release notes are only reachable as three recent entries; older IDs return 404.
- **RaceLab's full current column list.** There is no reference page. The table is reconstructed
  from release notes, so a column added without a release note would be missing. The release-note
  pages also serve as RaceLab's documentation, so features may since have been renamed.
- **Where any of the three commercial products got its logo art, and whether it holds licences.**
  None publishes this, so it is unknown rather than known to be unlicensed.
- **The Directive 2015/2436 Article 14 text** was not fetched (EUR-Lex rendered empty). The
  citation is standard, but the quote is not verified here.

## Sources

### RaceLab

- [R1] RaceLab, iRacing overlays page: https://racelab.app/iracing
- [R2] RaceLab release notes v1.16.5, 2022-01-19, class SoF, class name, driver counts:
  https://garage.racelab.app/release-notes/page/17
- [R3] RaceLab release notes v8.6.0 (2026-10-25), v8.7.0 and v8.7.1:
  https://garage.racelab.app/release-notes
- [R4] RaceLab release notes v5.5.0, 2024-05-18, relative timing column:
  https://garage.racelab.app/release-notes/page/10
- [R5] RaceLab release notes, laptime decimals settings (search result summary of
  docs.racelab.app release notes): https://docs.racelab.app/release-notes
- [R6] RaceLab release notes v1.4.0 and v1.1.3, 2021, pit/stint badges and visualized laptimes:
  https://garage.racelab.app/release-notes/page/20
- [R7] RaceLab release notes v1.3.10, v1.4.0, v1.2.4 and v1.5.2, 2021:
  https://garage.racelab.app/release-notes/page/20
- [R8] RaceLab release notes v2.6.3, 2022-07-23: https://garage.racelab.app/release-notes/page/16
- [R9] RaceLab release notes, stint indicators and tyre history (search result summaries):
  https://docs.racelab.app/release-notes/page/18
- [R10] RaceLab release notes v3.7.2, 2023-02-27, tyre history column:
  https://garage.racelab.app/release-notes/page/14
- [R11] RaceLab release notes v7.15.0, 2026-07-18, positions changed column:
  https://garage.racelab.app/release-notes/page/2
- [R12] RaceLab release notes v8.2.0 and v8.1.0, 2026, brand logos:
  https://garage.racelab.app/release-notes/page/2
- [R13] RaceLab release notes v7.6.0, 2026-04-07:
  https://garage.racelab.app/release-notes/2026/04/07/2026/v7.6.0/
- [R14] RaceLab release notes v5.9.1, 2024-10-26, iRating gain/loss, licence badge:
  https://garage.racelab.app/release-notes/page/9
- [R15] RaceLab release notes v6.13.1 and v6.3.6, WC/PRO licence, safety rating:
  https://garage.racelab.app/release-notes/page/6 and
  https://garage.racelab.app/release-notes/page/8
- [R16] RaceLab release notes v6.15.1 and v6.18.0, driver flags across all games:
  https://garage.racelab.app/release-notes/page/6 and
  https://garage.racelab.app/release-notes/page/5
- [R17] RaceLab release notes v1.15.3, 2021-12-24, nationality and club name:
  https://garage.racelab.app/release-notes/page/18
- [R18] RaceLab release notes v7.4.0, 2026-03-25, team and driver name:
  https://garage.racelab.app/release-notes/page/4
- [R19] RaceLab release notes v5.8.1, 2024-10-16, reworked P2P column:
  https://garage.racelab.app/release-notes/page/10
- [R20] RaceLab release notes v6.0.6, 2025-02-16, off-track column:
  https://garage.racelab.app/release-notes/page/8
- [R21] RaceLab release notes v4.0.3, 2024-01-17, last-lap top speed:
  https://garage.racelab.app/release-notes/page/12
- [R22] RaceLab release notes v6.18.0, 2025-12-13, column reorder, Lumina theme:
  https://garage.racelab.app/release-notes/page/5
- [R23] RaceLab release notes, Buick (v2.6.3), Acura (v3.19.0), Vauxhall (v3.8.4) and Caterham
  (v8.4.1) logos: https://garage.racelab.app/release-notes/page/16,
  https://garage.racelab.app/release-notes/page/12,
  https://garage.racelab.app/release-notes/page/14 and https://garage.racelab.app/release-notes
- [R24] RaceLab release notes v8.7.1, 2026-11-13, RaceRoom brand logos replacing the RaceRoom
  logo: https://garage.racelab.app/release-notes
- [R25] RaceLab release notes v1.5.2, 2021-05-29, "Colored car brands option":
  https://garage.racelab.app/release-notes/page/20
- [R26] RaceLab home and iRacing pages (no trademark or affiliation statement):
  https://racelab.app/ and https://racelab.app/iracing

### irDashies

- [D1] irDashies README, Standings section: https://github.com/tariknz/irdashies#standings
- [D2] irDashies Standings settings (column labels):
  https://github.com/tariknz/irdashies/blob/main/src/frontend/components/Settings/sections/StandingsSettings.tsx
- [D3] irDashies README, LMU notes: https://github.com/tariknz/irdashies#readme
- [D4] irDashies `tools/car_logos/README.md`:
  https://github.com/tariknz/irdashies/tree/main/tools/car_logos
- [D5] irDashies logo folder: https://github.com/tariknz/irdashies/tree/main/tools/car_logos/logos
- [D6] irDashies `CarManufacturer.tsx`:
  https://github.com/tariknz/irdashies/blob/main/src/frontend/components/shared/CarManufacturer/CarManufacturer.tsx
- [D7] irDashies `tools/build-lmu-manufacturer-sprite.cjs`:
  https://github.com/tariknz/irdashies/blob/main/tools/build-lmu-manufacturer-sprite.cjs
- [D8] irDashies LICENSE (MIT): https://github.com/tariknz/irdashies/blob/main/LICENSE
- [D9] irDashies PR #729, "feat: add season 4 car manufacturer logos":
  https://github.com/tariknz/irdashies/pull/729

### Kapps

- [K1] Kapps home page: https://kapps.kutu.ru/
- [K2] Kapps FAQ: https://kapps.kutu.ru/faq/
- [K3] Community patch "Kapps Country Flags" (third party):
  https://github.com/ssg22/kapps-country-flags
- [K4] Community patch "Kapps Mods for iRacing" (third party):
  https://github.com/Canedaaa/KAPPS-UPDATE
- [K5] Kapps brand assets: https://kapps.kutu.ru/brand/
- [K6] Kapps terms of service: https://kapps.kutu.ru/terms/

### iOverlay

- [O1] iOverlay home page: https://ioverlay.app/
- [O2] iOverlay release notes v2.0.1 (2026-11-23) and v2.0.0 (2026-11-14):
  https://ioverlay.app/release-notes/latest and https://ioverlay.app/release-notes/67
- [O3] iOverlay FAQ/help: https://ioverlay.app/help

### Trademark

- [T1] *Toyota Motor Sales, U.S.A., Inc. v. Tabari*, 610 F.3d 1171 (9th Cir. 2010):
  https://caselaw.findlaw.com/court/us-9th-circuit/1530600.html
- [T2] *Volkswagenwerk Aktiengesellschaft v. Church*, 411 F.2d 350 (9th Cir. 1969):
  https://hallapproved.com/us/cases/ca9/1969/284952/
- [T3] Court of Justice press release 7/99 on C-63/97 *BMW v Deenik*:
  https://curia.europa.eu/en/actu/communiques/cp99/cp9907en.htm
- [T4] Directive (EU) 2015/2436, Article 14:
  https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32015L2436 (not rendered for this
  research)
- [T5] Porsche Newsroom legal notice: https://newsroom.porsche.com/en_US/legal-notice.html
- [T6] Porsche UK legal notice: https://www.porsche.com/uk/legal-notice/
- [T7] Volvo Cars legal notice: https://www.volvocars.com/en-gt/legal/terms/terms-legal-notice
  (search-result summary only; not fetched in full)
- [T8] GitHub trademark policy:
  https://docs.github.com/en/site-policy/content-removal-policies/github-trademark-policy
- [T9] Simple Icons disclaimer:
  https://github.com/simple-icons/simple-icons/blob/develop/DISCLAIMER.md
