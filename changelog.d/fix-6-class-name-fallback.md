### Fixed

- Car classes iRacing leaves unnamed now get a name where one can be inferred. iRacing leaves
  `CarClassShortName` empty in AI and hosted sessions, multi-class ones included, and every class
  came out with an empty name. A class now takes a name any of its drivers carries, or its car's
  name when it holds a single model, such as `Dallara P217 LMP2`. A class of several models, such
  as GT3, has no name, and its class ID still groups it (#6)
- `reo check` lists the session's classes by ID with their names, such as
  `2523 Dallara P217 LMP2 | 4011 (unnamed)`, instead of a single empty name.
