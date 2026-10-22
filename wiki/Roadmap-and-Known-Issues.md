# Roadmap and Known Issues

## Done

- Title and instruction screens, every room of the five levels with their palettes.
- Room connection table; walking through connected rooms.
- Conrad's sprite frames, with his palette read from `FB.EXE`.
- Conrad's original animations and start positions; turning, walking, running, jumping,
  falling and climbing on the room collision grid.

## Next

1. Measure the game speed in DOSBox (now 30 updates per second, estimated).
2. Level scripts and objects (`.pge`, `.obj`, `.lev`): moves, items, doors, lifts, enemies.
3. Enemy sprite banks (`junky`, `mercenai`, `replican`, `glue`).
4. Text (`fb_txt.fnt`, `.tbn`), inventory icons (`global.icn`).
5. Cutscenes (`.cmd` / `.pol` polygon animations).
6. Sound effects and music (`.prf`, `.mid`, `.ins`).
7. Comparison against the original running in DOSBox, as OpenBlood does.

## Known issues

- Conrad's moves come from a hand-written state machine, not yet from the level scripts; no
  crouching, rolling, drawing the gun, using objects or lifts.
- Dropping from a ledge cuts from hanging to landing instead of falling.
- The room foreground layer (pixels with bit 7) is drawn under Conrad instead of over him.
- Conrad's palette is the fifth of seven palettes in `FB.EXE`; not yet checked against a
  capture of the original.
