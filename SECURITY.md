# Security

## Reporting

Please report vulnerabilities privately through GitHub's "Report a vulnerability" button on this
repository's **Security** tab, not in public issues.

## What the overlays trust

- **Sim shared memory is untrusted input.** It is written by another process. The reader copies
  out of it and never holds references into it, validates the header (version, buffer count and
  sizes) before using it, drops variables whose offsets would overrun a row, and re-checks the
  tick count after each copy.
- **Session text is untrusted input.** Driver and team names in iRacing's session YAML are
  chosen by other players. They are decoded as Windows-1252 and quoted before parsing, and are
  only ever displayed, never executed or used as paths.
- **Recordings are private.** `.irdt` and `.lmudt` tapes carry driver names and customer IDs;
  `.gitignore` keeps them out of the repository, and they are never attached to issues.
- **No elevation, no network.** The core needs no administrator rights and opens no sockets.

## Known gaps

- Release binaries aren't signed yet.
