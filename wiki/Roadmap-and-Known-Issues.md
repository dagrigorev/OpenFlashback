# Roadmap and Known Issues

## Done

- Title and instruction screens, every room of the five levels with their palettes.
- Room connection table; walking through connected rooms.
- Conrad's sprite frames, with his palette read from `FB.EXE`.

## Next

1. Conrad's animations from `.ani` / `.bnq` (the walk cycle is currently a guess: frames 4-11).
2. Collision from the room grid in `.ct`: floors, walls, ledges, falls.
3. Level scripts and objects (`.pge`, `.obj`, `.lev`): moves, items, doors, lifts, enemies.
4. Enemy sprite banks (`junky`, `mercenai`, `replican`, `glue`).
5. Text (`fb_txt.fnt`, `.tbn`), inventory icons (`global.icn`).
6. Cutscenes (`.cmd` / `.pol` polygon animations).
7. Sound effects and music (`.prf`, `.mid`, `.ins`).
8. Comparison against the original running in DOSBox, as OpenBlood does.

## Known issues

- No game logic: Conrad cannot jump, climb, fall or collide with anything.
- The room foreground layer (pixels with bit 7) is drawn under Conrad instead of over him.
- Conrad's palette is the fifth of seven palettes in `FB.EXE`; not yet checked against a
  capture of the original.
