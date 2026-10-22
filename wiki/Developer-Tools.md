# Developer Tools

`openflashback <tool> ...` writes PNG files from the original data:

| Tool | Output |
| --- | --- |
| `screen DATA NAME OUT.png` | a full-screen picture: `menu1`, `menu2`, `menu3`, `instru_f` |
| `rooms DATA LEVEL OUT.png` | contact sheet of every room of level 1-5 |
| `shot DATA LEVEL ROOM OUT.png` | one room with Conrad, as the game draws it |
| `sprites DATA BANK OUT.png` | every frame of a sprite bank (`perso`, `junky`, ...) in grey |
| `ct DATA LEVEL` | prints each room's neighbours |

Screenshots for the README and wiki: `openflashback shot DATA 1 27 screenshots/level1.png`.
