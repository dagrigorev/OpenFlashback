# Roadmap and Known Issues

## Done

- Title and instruction screens, every room of the five levels with their palettes.
- Room connection table; walking through connected rooms.
- Conrad's sprite frames, with his palette read from `FB.EXE`.
- Conrad's original animations and start positions; turning, walking, running, jumping,
  falling and climbing on the room collision grid.
- Object script reader (`.obj`) and dump tool; Conrad's moves follow the script's animation
  chains and controls (checked by a test against the original data).
- Cutscenes: polygon pictures and scripts, played in the letterbox; logos at startup, the intro
  before level 1; the room foreground layer is drawn over Conrad.

## Next

1. Measure the game speed in DOSBox (now 30 updates per second, estimated).
2. Decode the remaining `.obj` condition and action opcodes and run the scripts: moves, items,
   doors, lifts, enemies (`.pge`, `.obj`, `.lev`).
3. Enemy sprite banks (`junky`, `mercenai`, `replican`, `glue`).
4. Text (`fb_txt.fnt`, `.tbn`), inventory icons (`global.icn`).
5. Cutscene captions (`fr_cine.txt`), the in-game cutscenes (lifts, teleporters, ...), and the
   menus scripted with opcode 14.
6. Sound effects and music (`.prf`, `.mid`, `.ins`).
7. Comparison against the original running in DOSBox, as OpenBlood does.

## Known issues

- Conrad's moves come from a hand-written state machine that follows the script's chains, not
  from running the script; no rolling, drawing the gun, using objects or lifts.
- Dropping from a ledge cuts from hanging to landing instead of falling.
- Cutscene frame rate (12 per second) and rotation (two of three angles ignored) are guesses.
- Conrad's palette is the fifth of seven palettes in `FB.EXE`; not yet checked against a
  capture of the original.
