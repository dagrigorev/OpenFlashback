# Developer Tools

`openflashback <tool> ...` writes PNG files from the original data:

| Tool | Output |
| --- | --- |
| `screen DATA NAME OUT.png` | a full-screen picture: `menu1`, `menu2`, `menu3`, `instru_f` |
| `rooms DATA LEVEL OUT.png` | contact sheet of every room of level 1-5 |
| `shot DATA LEVEL ROOM OUT.png` | one room with Conrad, as the game draws it |
| `sprites DATA BANK OUT.png` | every frame of a sprite bank (`perso`, `junky`, ...) in grey |
| `ct DATA LEVEL` | prints each room's neighbours |
| `obj DATA LEVEL NODE [STATE]` | prints an object node's script transitions, all or those leaving one state (Conrad is node 1) |
| `cutscene DATA NAME SUB EVERY OUT.png` | plays sub-script SUB of `NAME.cmd` with `NAME.pol` and keeps every EVERY-th frame in a contact sheet, e.g. `cutscene DATA intro1 0 30 intro.png` |
| `music DATA PROFILE SECONDS OUT.wav` | renders the song of `PROFILE.prf` (e.g. `introl3`, `present3`) with its AdLib instruments to a WAV file |
| `film DATA LEVEL INPUT EVERY OUT.png` | plays scripted input and keeps every EVERY-th tick in a contact sheet, printing Conrad's room, position and animation |

`film` input is space-separated `KEYS*TICKS` with keys `l r u d s` (s = run) or `n` (none). A
running jump over the first gap of level 1:

```
openflashback film DATA 1 "n*30 r*10 rs*10 rsu*1 n*40" 4 jump.png
```

Screenshots for the README and wiki: `openflashback shot DATA 1 27 screenshots/level1.png`.
