# Installing, auto-updating and code-signing reo on Windows

Research for [#26](https://github.com/jwh3times/RaceEngineerOverlays/issues/26), on the route-to-1.0
map [#21](https://github.com/jwh3times/RaceEngineerOverlays/issues/21). Gathered 2026-11-27. These
are findings, not a decision: a later ticket chooses.

**Question.** reo 1.0 needs an installer, auto-update and a code-signed binary, so that Windows
SmartScreen does not warn. Which signing route (SignPath Foundation, Azure Artifact Signing, a CA's
OV/EV certificate) and which installer and updater (Velopack, WiX, MSIX, Inno Setup and the like)
fit an AGPL-3.0 Rust workspace that releases from `.github/workflows/release.yml` on
`windows-latest`?

Every source is named inline with the date it carries. "Page date" is the page's own `ms.date` or
last-updated stamp; where a page has none, the date given is the day it was read (2026-11-27).
Statements marked **inference** are this file's reasoning, not a source's claim.

## Short answer

- **No signing route removes the first-download SmartScreen warning for a new app.** Since 2024 EV
  certificates get no instant reputation. OV, EV, Artifact Signing and SignPath all start with a
  warning, and reputation then builds from file-hash history and the signing identity. Only an
  MSIX package distributed through the Microsoft Store is shown without a warning, because the
  Store re-signs it.
- **Signing routes:**
  - **SignPath Foundation** is free for OSI-licensed projects. The certificate names "SignPath
    Foundation" as publisher, not the project, and every release needs a manual approval.
  - **Azure Artifact Signing** (formerly Trusted Signing) costs US$9.99 a month. Individuals must
    be in the USA or Canada and pass ID verification. It has a first-party GitHub Action, and
    Velopack supports it natively.
  - **A CA certificate** costs from €69 (Certum's open-source card) to about US$970 a year
    (DigiCert's upper tier), with the key on a hardware token or a paid cloud HSM. Since 2026-03-01 a certificate lasts at most 460 days.
- **Installers and updaters:** Velopack is the only option that is MIT-licensed, installs per user
  without admin, ships delta updates, publishes to and updates from GitHub Releases, and has a
  first-party Rust crate. WiX and Inno Setup are installer builders with no updater. MSIX has
  per-user installs and block-level differential updates, but needs a trusted signature even to
  sideload. Its web one-click install has been disabled by default since December 2023.
- **Leading combination on these findings:** Velopack for the installer and updater, signed with
  Azure Artifact Signing if the maintainer qualifies as an individual (USA or Canada), with
  SignPath Foundation as the free fallback. SignPath does not fit Velopack cleanly (see
  [Fit between signing and Velopack](#fit-between-signing-and-velopack)), and a spike should
  settle that before anyone commits to it. Choosing is the next ticket's job.

## How SmartScreen reputation works

Sources: Microsoft Learn,
[SmartScreen reputation for Windows app developers](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation)
(page date 2026-05-04, updated 2026-08-17);
[Code signing options for Windows app developers](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
(page date 2026-10-29, updated 2026-11-10);
[Current status of Windows app distribution features](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/distribution-feature-status)
(page date 2026-08-24).

- SmartScreen weighs two signals: **publisher reputation** (is the file signed, and is the
  certificate's publisher known?) and **file-hash reputation**. "Even when signed, a newly created
  binary could still show a SmartScreen warning until its hash or publisher certificate accumulates
  sufficient evidence of positive reputation."
- **EV no longer bypasses SmartScreen.** "EV certificates no longer bypass SmartScreen... Paying a
  premium for EV solely to avoid SmartScreen warnings is no longer justified." The status page
  dates the change to 2024, when the Trusted Root Program removed the EV-specific OIDs.
- First-download behaviour by route: Microsoft Store (MSIX) shows no warning. A valid OV or EV
  certificate, Artifact Signing among them, shows a warning that names the verified publisher until
  reputation accumulates. An unsigned or self-signed file gets "Windows protected your PC" and needs
  "Run anyway".
- Reputation carries across versions only with a consistent signing identity: "Signing files using
  a trusted certificate can allow certificate reputation to build, potentially avoiding warnings on
  new files signed by the same trusted certificate. Unsigned files must build reputation anew with
  every update." Changing certificate affects the publisher signal.
- There is no threshold: it "can take several weeks and hundreds of clean installs from a wide
  audience". Consumers have no way to submit a file for review. The Microsoft Security Intelligence
  portal is for enterprise administrators, and the Artifact Signing FAQ also points there for
  persistent prompts.
- On Windows 11, **Smart App Control** "will block execution of unsigned files unless the file has
  a positive reputation", and it applies to every executable, not only downloads. That makes
  signing matter beyond the download prompt.

**Inference for reo:** sim racing is a small audience, so the hash route alone may never clear the
warning on a given build. A signing identity that stays the same across releases matters more than
the certificate type, and a shared identity with existing reputation (SignPath Foundation's
certificate, or the Store) may clear sooner than a new personal certificate. No source measures
this.

## Signing routes

### SignPath Foundation (free for open source)

Sources: [SignPath Foundation terms (OSS conditions)](https://signpath.org/terms) (no date; the page
says the code of conduct is "still in a draft version"; read 2026-11-27);
[signpath.org](https://signpath.org/) and [About](https://signpath.org/about) (no dates);
SignPath docs, [Trusted build systems: GitHub](https://docs.signpath.io/trusted-build-systems/github)
(no date) and
[Artifact configuration reference](https://docs.signpath.io/artifact-configuration/reference) (no
date); Microsoft Learn, Code signing options (above).

- **Cost:** "For OSS projects, our services are free of charge."
- **Eligibility:**
  - Every component must be under an OSI-approved licence, with no commercial dual-licensing and no
    proprietary components (system libraries are allowed). AGPL-3.0-only is OSI-approved.
  - The project must be actively maintained and already released in the form to be signed.
  - It must be documented on its download page.
- **Publisher name:** certificates are issued to **"SignPath Foundation"**. Projects cannot get a
  certificate in their own name, so users see SignPath Foundation, not "Race Engineer Overlays", as
  the publisher.
- **Obligations:**
  - Publish a "Code signing policy" on the home and download pages, stating free signing by
    SignPath.io with a certificate from SignPath Foundation, and list the team roles (Authors,
    Reviewers, Approvers) and their members.
  - Provide a privacy policy that discloses any data transfers.
  - Set product name and version metadata on the signed binaries.
  - Use MFA on both SignPath and the source repository.
  - Announce system configuration changes and provide uninstallation.
- **Restrictions:** no malware or PUPs, no hacking or vulnerability-scanning tools, and only the
  project's own binaries may be signed (unsigned upstream libraries can be inside the package).
- **Process:** "Signed binaries must come from source code built in a verifiable way", and **each
  release needs manual approval** by an Approver. Build scripts and CI configuration count as source
  code under review.
- **In CI (GitHub Actions):**
  - The workflow uploads the unsigned files with `actions/upload-artifact` (v4+) and passes the
    artifact ID to `signpath/github-action-submit-signing-request`.
  - SignPath's origin verification confirms that a GitHub workflow built the artifact.
  - For open-source projects, "all jobs leading up to the signing request must run on
    GitHub-hosted agents". `windows-latest` qualifies.
  - The action waits up to 600 s by default, and the signed artifact comes back as a download URL.
- **Formats:** SignPath signs PE, MSI, MSIX/AppX, NuGet (`nupkg-file`), ZIP and others, and can
  "deep sign" files nested inside composite formats (MSI, CAB, MSIX, NuGet, ZIP).
- **Operator:** the Foundation "is currently operated by" SignPath GmbH and hopes to become
  independent.
- **SmartScreen:** neither signpath.org nor its docs say anything about reputation. Microsoft
  describes the programme as "OV-level certificate signing through a managed pipeline", so it gets
  the OV behaviour above.
- **Not established:** the application form's criteria (the [Apply](https://signpath.org/apply)
  page renders only a title outside a browser), how long approval takes, and whether a project
  needs a release history before applying beyond "already released".

### Azure Artifact Signing (formerly Trusted Signing)

Sources: Microsoft Learn,
[Quickstart: Set up Artifact Signing](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart)
(page date 2026-05-21, updated 2026-11-20);
[Artifact Signing FAQ](https://learn.microsoft.com/en-us/azure/artifact-signing/faq) (page date
2026-11-17, updated 2026-11-20);
[What is Artifact Signing?](https://learn.microsoft.com/en-us/azure/artifact-signing/overview)
(page date 2026-01-02); Azure
[Retail Prices API](https://prices.azure.com/api/retail/prices?$filter=serviceName%20eq%20'Trusted%20Signing')
(prices effective 2024-06-01, read 2026-11-27);
[Artifact Signing pricing page](https://azure.microsoft.com/en-us/pricing/details/artifact-signing/)
(the amounts render client-side; it confirms 5,000 and 100,000 signatures a month);
[Azure/artifact-signing-action](https://github.com/Azure/artifact-signing-action) (MIT, last pushed
2026-11-16).

- **Price:**

  | SKU     | Price a month | Signatures a month | Certificate profiles |
  |---------|---------------|--------------------|----------------------|
  | Basic   | US$9.99       | 5,000              | 1 of each type       |
  | Premium | US$99.99      | 100,000            | 10 of each type      |

  An overage signature costs US$0.005 (all from the Retail Prices API, which still lists the meter
  under "Trusted Signing"). Billing is not pro-rated and starts when the account is created. Free,
  trial and sponsored Azure subscriptions are not supported: it needs pay-as-you-go or an
  enterprise agreement.
- **Eligibility for an individual** (quickstart prerequisites): "Individual developers must be
  located in the United States or Canada."
  - The Azure billing account must have Account Type **Individual**, and its legal name and sold-to
    address must match a government-issued ID.
  - Verification runs through Microsoft Entra Verified ID with AU10TIX: an ID document, a Face
    Check selfie and Microsoft Authenticator.
  - The certificate shows the validated name and the city, state or province, and country. Email
    and street address are left out unless chosen.
  - A custom CN or O is not possible: the CN is the validated legal name.
- **Organisations:** Public Trust is open to organisations in the US, Canada, the EU, the UK,
  Australia, New Zealand, Japan, South Korea, Singapore, Switzerland, Norway and Israel (quickstart).
  The Code signing options page gives a shorter list (USA, Canada, EU, UK), so the two Microsoft
  pages disagree. Validating an organisation takes 1 to 30 business days.
- **Identity validation expires**, and renewal reminders start 60 days before. Deleting or letting
  it lapse stops certificate renewal.
- **Certificates:**
  - Microsoft keeps them in FIPS 140-3 Level 3 HSMs and never hands one out.
  - They are short-lived. The FAQ's ELAM guidance says an Artifact Signing certificate "is valid for
    three days", so timestamping (`http://timestamp.acs.microsoft.com`) is essential.
  - Artifact Signing does not issue EV certificates and has "no plan" to.
- **SmartScreen:** "SmartScreen reputation builds up automatically. The prompt stops appearing once
  the file hash has sufficient download history." The Code signing options page adds that signing
  consecutive releases with a consistent identity lets publisher reputation build. That works even
  though the certificates rotate, because the identity, not the certificate, carries the
  reputation (that is how Microsoft words it). It also calls Artifact Signing "functionally
  equivalent" to OV for SmartScreen.
- **In CI:**
  - `azure/artifact-signing-action@v2` runs on Windows runners (windows-2022 and windows-2025 are
    listed, not Arm).
  - It authenticates with OIDC federated credentials through `azure/login` (`id-token: write`),
    signs a folder of files, and timestamps them.
  - SignTool with the dlib, PowerShell and an SDK are the alternatives.

### A CA's OV or EV certificate

Sources: CA/Browser Forum,
[Ballot CSC-31: Maximum Validity Reduction](https://cabforum.org/2025/11/17/ballot-csc-31-maximum-validity-reduction/)
(2025-11-17) and the
[Code Signing Baseline Requirements](https://cabforum.org/working-groups/code-signing/requirements/);
[Sectigo code signing](https://sectigo.com/products/signing-certificates/code-signing) (read
2026-11-27); [DigiCert code signing](https://www.digicert.com/signing/code-signing-certificates)
(read 2026-11-27); [SSL.com code signing](https://www.ssl.com/certificates/code-signing/) and
[eSigner pricing](https://www.ssl.com/guide/esigner-pricing-for-code-signing/) (updated 2026-03-25);
[Certum Open Source Code Signing](https://shop.certum.eu/open-source-code-signing.html) (read
2026-11-27); Microsoft Learn, Code signing options (above).

- **Validity:** a code-signing certificate issued on or after 2026-03-01 may last at most 460 days
  (CSC-31). Sectigo's page says 459 days from 2026-02-23. Multi-year orders still exist, with the
  certificate reissued during the term.
- **Key storage:** since June 2023 the private key must live in an HSM or a hardware token
  (Microsoft and Sectigo). Sectigo: "software-only key storage isn't permitted." In CI that means a
  cloud HSM service (DigiCert KeyLocker, SSL.com eSigner, Google Cloud KMS and the like). A USB
  token cannot be used from a GitHub-hosted runner.
- **Prices** as the vendors' own pages show them:

  | Vendor | What it sells | Price shown | Individuals |
  |--------|---------------|-------------|-------------|
  | Microsoft's guide | OV | "typically $150–300/year" | n/a |
  | Microsoft's guide | EV | "$400+/year" | n/a |
  | Sectigo | Code signing | "starts at $536.25 per year when customers choose the five-year option"; more for shorter terms and for EV | The page is ambiguous |
  | DigiCert | Two subscription tiers, not labelled OV or EV | US$696 and US$972 a year | Says it is "designed for individual developers and small teams" |
  | SSL.com | IV (individual) and EV Sole Proprietor; OV and EV need an organisation | Not shown on the page | Yes |
  | SSL.com eSigner (cloud HSM for CI), OV/IV | Subscription on top of the certificate | US$20 a month for 20 signings, US$85 for 100 | n/a |
  | SSL.com eSigner, EV | Subscription on top of the certificate | US$100 a month for 10 signings | n/a |
  | Certum | Open Source Code Signing set (card and reader) | €69 | Yes: the certificate carries the developer's name prefixed "Open Source Developer" |

  Certum's validity and its cloud option (SimplySign) are not on that page.
- **SmartScreen:** the same as Artifact Signing. Reputation builds over time. Sectigo: "Neither
  guarantees instant SmartScreen reputation". SSL.com says EV and OV have been treated equally
  since March 2024.
- **Inference:** a hardware-only Certum card cannot sign on a GitHub-hosted runner. Signing would
  move to the maintainer's machine, which breaks the "a tag is exactly the code that merge built"
  property that `release.yml` keeps today.

### Microsoft Store

Sources: Microsoft Learn,
[Open a developer account](https://learn.microsoft.com/en-us/windows/apps/publish/partner-center/open-a-developer-account)
(page date 2026-10-29);
[App package requirements for MSI/EXE](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/app-package-requirements)
(page date 2022-10-30, updated 2026-08-24); Code signing options (above).

- An individual developer account is **free** through storedeveloper.microsoft.com, with
  government-ID and selfie verification. It is meant for developers publishing "not in relation to
  their business, trade, or profession".
- **MSIX through the Store:** Microsoft re-signs the package, with "free Microsoft code signing and
  CDN hosting", and users "never see a SmartScreen warning". It is the only route with no
  first-download warning. Updates then come from the Store, not from GitHub Releases.
- **MSI or EXE through the Store:** Microsoft does **not** re-sign. The installer and every PE file
  in it must be signed by a Trusted Root Program CA, so it needs one of the routes above. The
  installer must be silent, offline and served from a versioned HTTPS URL the developer hosts.
- **Not established:** whether Store certification accepts an always-on-top overlay that reads
  another process's shared memory. MSIX packaging of reo would also need the checks in
  [MSIX](#msix).

## Installer and updater options

| Option | Licence (AGPL fit) | Per-user, no admin | Updater | Delta updates | GitHub Releases | Rust fit |
|--------|--------------------|--------------------|---------|---------------|-----------------|----------|
| **Velopack** | MIT (fine) | Yes, `%LocalAppData%\{packId}` | Built in (`UpdateManager`) | Yes (zstd per-file patches) | Yes: `vpk upload github` / `download github`, client `GithubSource` | First-party `velopack` crate (MIT); `vpk` CLI needs the .NET 8 SDK at build time |
| **WiX Toolset** v7 | MS-RL (reciprocal per file; the tool only, not the output); OSMF EULA must be accepted | Per-user MSI possible; v7 adds dual-purpose per-user or per-machine | None. Burn's `Update` element only reports a feed location | No (MSI patches are a separate craft) | Hosting only | `cargo-wix` (Apache-2.0) wraps it |
| **MSIX** (+ App Installer) | Windows SDK / WinApp CLI (MIT) / msix-packaging SDK (MIT) | Yes, by design | `.appinstaller` file: on launch or background checks | Yes, 64 KB block-map differential | `.appinstaller` must be reachable at a stable URI | No first-party Rust packager; WinApp CLI or MakeAppx |
| **Inno Setup** 7 | Custom permissive licence; commercial users (above US$5,000 a year) are asked to buy a licence | Yes, `PrivilegesRequired=lowest`, HKCU uninstall | None | No | Hosting only | None (an `.iss` script) |
| **NSIS** | zlib/libpng (bzip2 and CPL-1.0 modules) | Possible (script-defined) | None | No | Hosting only | Through `cargo-packager` |
| **cargo-packager** + `cargo-packager-updater` | Apache-2.0 / MIT | Per NSIS or WiX config | Yes: signed full-installer download from a JSON endpoint | No | A static JSON on Releases can serve as the endpoint (**inference**) | Native cargo subcommand; last release 2025-11-27, updater 2025-07-21 |
| **cargo-dist** (`dist`) + axoupdater | Apache-2.0 | MSI (bundling) or a fetching PowerShell `irm \| iex` script; the MSI's scope is not stated | Experimental `install-updater`, shell and PowerShell installers only; queries the GitHub API | No | Built around GitHub Releases | Native; v0.34.0 on 2026-11-02 ([installers](https://axodotdev.github.io/cargo-dist/book/installers/index.html), [updater](https://axodotdev.github.io/cargo-dist/book/installers/updater.html), read 2026-11-27) |
| **winget** (channel, not an installer) | MIT client; manifests by PR to winget-pkgs | Depends on the installer | `winget upgrade` | No | Manifests point at Release URLs | Any of the above |

Licence and release facts come from the GitHub API, read 2026-11-27:

- `velopack/velopack`: MIT, latest stable 1.2.161 (2026-09-29), prerelease 1.3.39 (2026-10-17).
- `wixtoolset/wix`: MS-RL, v7.0.1 (2026-10-26).
- `jrsoftware/issrc`: custom licence, `is-7_1_1` (2026-10-28).
- `crabnebula-dev/cargo-packager`: Apache-2.0.
- `volks73/cargo-wix`: Apache-2.0, 0.3.9 (2025-03-13).
- `axodotdev/cargo-dist`: Apache-2.0.
- `microsoft/WinAppCli`: MIT, v0.8.0 (2026-11-07).
- `microsoft/winget-cli`: MIT.
- `Squirrel/Squirrel.Windows`: MIT, but its last release was 2.0.1 in 2020. Velopack is its
  successor and has a Squirrel migration guide.

An installer tool's licence binds the tool, not reo's code: none of these makes reo's output
AGPL-incompatible. MS-RL's reciprocity applies only to files that contain WiX code (the WiX
runtime pieces inside a built MSI) and asks for their source, which WiX publishes.

### Velopack

Sources: [docs.velopack.io](https://docs.velopack.io/) (no page dates; read 2026-11-27):
[Installer](https://docs.velopack.io/packaging/installer),
[Deltas](https://docs.velopack.io/packaging/deltas),
[Signing](https://docs.velopack.io/packaging/signing),
[GitHub Actions](https://docs.velopack.io/distributing/github-actions),
[Rust quick start](https://docs.velopack.io/getting-started/rust),
[vpk windows CLI reference](https://docs.velopack.io/reference/cli/content/vpk-windows) (for 1.2.161);
[docs.rs velopack::sources](https://docs.rs/velopack/latest/velopack/sources/index.html) (1.2.161);
crates.io API (`velopack`, MIT, max stable 1.2.161).

- **Install:**
  - `Setup.exe` is a one-click installer with no wizard. It extracts to `%LocalAppData%\{packId}`,
    adds Start-menu and Desktop shortcuts (configurable with `--shortcuts`) and launches the app.
  - An optional `--msi` builds an MSI with a Velopack fork of WiX v7. Its `--instLocation` can be
    `PerUser` (HKCU, no elevation), `PerMachine` (Program Files, elevated) or `Either` (the
    default).
  - A portable zip is built unless `--noPortable` is passed.
- **Rust:**
  - Add the `velopack` crate and call `VelopackApp::build().run()` first in `main` ("it may
    terminate or restart the process").
  - Updating is `UpdateManager::new(source, ..)`, then `check_for_updates()`, then
    `download_updates()`, then `apply_updates_and_restart()`.
  - The sources include `GithubSource` (GitHub and GitHub Enterprise releases), `HttpSource`,
    `FileSource` and `AutoSource`.
  - Packaging is `vpk pack -u <id> -v <version> -p target/release -e reo.exe`.
- **Deltas:**
  - `vpk pack` writes a delta when the previous release is in the output folder. In CI,
    `vpk download github` fetches it first.
  - A delta is a set of zstd binary patches, one per file. `--delta` can be `BestSpeed` (the
    default), `BestSize` or `None`.
  - The client chains deltas from the package it has stored, or downloads the full package when a
    heuristic says that is cheaper.
- **GitHub Releases:**
  - `vpk upload github --repoUrl .. --token .. [--publish] [--merge] [--tag ..] [--releaseName ..]
    [--pre]`. Without `--publish` it leaves a draft. `--merge` adds files to an existing release.
  - **Inference:** `--merge` fits `release.yml`, where the `tag` job creates the Release and the
    `package` job uploads to it. The job would need `vpk download github` before `vpk pack` to
    produce deltas.
  - Velopack's GitHub Actions example uses `permissions: contents: write`.
- **Signing:**
  - `--signParams` passes arguments to signtool. `--signTemplate "<cmd> {{file}}"` runs a custom
    tool per file, or per batch with `{{file...}}`. `--azureTrustedSignFile metadata.json` handles
    Artifact Signing ("Since Velopack 1.3, `vpk` signs with Azure Artifact Signing directly (no
    `signtool.exe`)").
  - The docs say "the Velopack binaries (such as Update and Setup) need to be signed at different
    points in the package build process", so signing has to happen inside `vpk pack`. Files that
    are already signed are left as they are.
- **Build-time dependency:** `vpk` installs as a .NET tool (`dotnet tool install -g vpk`, .NET 8
  SDK). The shipped `Update.exe` and `Setup.exe` are Velopack's own binaries, MIT-licensed.
- **Open issues to know about** (GitHub, read 2026-11-27): #1145, updates take about 3 min on
  machines with EDR. #1045 (closed), 1.2.x failed to sign `Setup.exe` with Artifact Signing.

### MSIX

Sources: Microsoft Learn, [What is MSIX?](https://learn.microsoft.com/en-us/windows/msix/overview)
(page date 2026-09-30);
[Auto-update and repair apps](https://learn.microsoft.com/en-us/windows/msix/app-installer/auto-update-and-repair--overview)
(page date 2026-04-10);
[MSIX containerization overview](https://learn.microsoft.com/en-us/windows/msix/msix-containerization-overview)
(page date 2026-04-15); Current status of Windows app distribution features (above).

- MSIX packages install per user, with a clean uninstall. "Only changed 64 KB blocks are downloaded
  on update" (through `AppxBlockMap.xml`). "All MSIX packages must be signed before installation",
  so sideloading still needs a trusted certificate from one of the signing routes, and a
  self-signed one fails for public users.
- Auto-update outside the Store runs through an `.appinstaller` file: on-launch checks,
  `HoursBetweenUpdateChecks`, `ShowPrompt` and `UpdateBlocksActivation`. The last three need the
  2021 schema; Visual Studio writes the 2017/2 schema by default, and those settings are then
  "silently ignored". It works on Windows 10 2004 and later and all of Windows 11.
- The `ms-appinstaller:` one-click web install has been **disabled by default since December 2023**
  (App Installer 1.21.3421.0), after malware abused it. Users download the `.appinstaller` or
  `.msix` and open it.
- A full-trust packaged desktop app runs at medium integrity with "some file/registry
  virtualization". Writes to `AppData` pass through for full-trust apps.
- **Not established:** whether named kernel objects, such as iRacing's shared-memory file mapping,
  behave the same for a packaged full-trust process. The Microsoft pages read do not say. A spike
  would settle it.

### WiX Toolset

Sources: [`wixtoolset/wix` LICENSE.TXT and OSMFEULA.txt](https://github.com/wixtoolset/wix) (read
2026-11-27); FireGiant docs, [Open Source Maintenance Fee](https://docs.firegiant.com/wix/osmf/),
[What's new](https://docs.firegiant.com/wix/whatsnew/) and
[Update element](https://docs.firegiant.com/wix/schema/wxs/update/) (no dates; read 2026-11-27).

- The licence is MS-RL, an OSI-approved reciprocal licence that applies per file.
- WiX v6 introduced the Open Source Maintenance Fee. The fee applies to users "that use the
  Software as part of revenue-generating activities and have an annual gross revenue greater than
  or equal to US$10,000". It "is not a license fee", and self-compiled binaries are outside it.
- **WiX v7 enforces acceptance of the EULA:** a build must pass `-acceptEula wix7` or set the
  `AcceptEula` MSBuild property. The page says the direct option "is designed for use in build
  scripts and CI/CD".
- WiX has no application updater. A Burn bundle's `Update` element only hands a feed location to
  the bootstrapper application ("In the future the engine will be able to acquire an update
  bundle").
- v7 adds "dual-purpose MSI packages in bundles": per-user or per-machine, chosen at install time.

### Inno Setup

Sources: [`jrsoftware/issrc` license.txt](https://github.com/jrsoftware/issrc/blob/main/license.txt)
(copyright 1997-2026; read 2026-11-27); [Inno Setup ordering page](https://jrsoftware.org/isorder.php)
(read 2026-11-27);
[PrivilegesRequired](https://jrsoftware.org/ishelp/topic_setup_privilegesrequired.htm) and
[Non-administrative install mode](https://jrsoftware.org/ishelp/topic_admininstallmode.htm) help
pages.

- The licence is permissive (use, alter and redistribute with notices kept). Separately, the
  ordering page asks commercial users, meaning organisations or individuals with more than US$5,000
  a year in for-profit revenue, to buy a perpetual licence. Donations to open-source projects count
  toward that threshold. Non-commercial users are not asked.
- `PrivilegesRequired=lowest` always runs Setup in non-administrative mode: per-user Start-menu
  group, "auto" constants mapped to the user form, and uninstall information under HKCU.
- Inno Setup has no updater and no deltas. An updater would have to be written into reo.

## Fit between signing and Velopack

- **Velopack and Artifact Signing:** native (`--azureTrustedSignFile`). It signs the app binaries,
  `Setup.exe` and the MSI, and timestamps each. In CI it needs an Azure login with OIDC before
  `vpk pack` on the Windows job. **Inference:** this is the shortest path from today's `package`
  job.
- **Velopack and SignPath Foundation:** awkward. **Inference** from the two sets of docs:
  - Velopack signs during `vpk pack`, at several points.
  - SignPath's GitHub flow takes an uploaded workflow artifact and returns a signed one, with a
    manual approval per request.
  - Signing the Velopack output afterwards would change files whose hashes Velopack has already
    recorded in its release feed.
  - One way through is to have SignPath sign `reo.exe` (and anything else reo builds) first, then
    let `vpk pack` skip the already-signed files. That still leaves Velopack's own `Setup.exe` and
    `Update.exe`, which need signing inside `pack` (through `--signTemplate`, which calls a
    SignPath submission per file or batch, each needing approval).
  - Neither project documents this combination, and no Velopack issue mentions SignPath (GitHub
    search, 2026-11-27). It needs a spike.
- **Velopack and a CA certificate:** `--signParams` with signtool and a cloud-HSM KSP, or
  `--signTemplate` with the CA's signing tool. It works in CI only with a cloud-HSM product such as
  eSigner or KeyLocker, at extra cost.

## Fit with today's release workflow

`release.yml` (read 2026-11-27) builds `reo.exe` on `windows-latest` from the tag and uploads a zip
and `SHA256SUMS.txt` to the Release the `tag` job created. The `package` job holds only
`contents: write`. **Inference** about what each route would add:

- **Artifact Signing:** `id-token: write`, an `azure/login` step, and a `metadata.json` (endpoint,
  account and profile names, none of them secret). With Velopack: install `vpk` (the .NET SDK is
  on the runner image), run `vpk download github`, then `vpk pack --azureTrustedSignFile ..`, then
  `vpk upload github --merge --tag $TAG`.
- **SignPath:** an `actions/upload-artifact` step, the SignPath action (API token secret,
  organisation and project slugs) and a manual approval, so the job waits on a person each release.
  The repository would also need a published code signing policy page and a privacy policy.
- **Either way,** the binaries are built clean from the tag as today, and the version check on
  `reo --version` still applies before signing.

## Open questions this research could not settle

1. Where the maintainer is based and whether an Azure billing account of type Individual is
   acceptable. Artifact Signing for individuals is limited to the USA and Canada.
2. SignPath Foundation's application criteria and turnaround. The Apply page did not render, and
   the terms are marked draft.
3. Whether SignPath and Velopack can be combined without several manual approvals per release (the
   spike above).
4. Whether a packaged full-trust MSIX process sees iRacing's and LMU's shared memory like an
   unpackaged one, and whether the Store would certify an overlay.
5. How long SmartScreen reputation takes for a niche app under each identity. No source gives
   numbers, and a shared identity (SignPath Foundation's) may differ from a new personal one.
6. Certum's certificate validity and whether its cloud key (SimplySign) is available for the Open
   Source product. The product page names only a card and reader.
