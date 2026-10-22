# Installation

1. Download the package for your system from [Releases](Releases) and unpack it.
2. From your installed **DOS** Flashback, copy every file of the `DATA` folder (`level1.map`,
   `perso.spr`, `menu1.map`, ...) next to `openflashback` (`openflashback.exe` on Windows).
   A `data` subfolder works too, or pass `--data <dir>`, or set `OPENFLASHBACK_DATA`.
3. Also copy `FB.EXE`: Conrad's colours are read from it. Without it he is drawn in grey.
4. Run `openflashback` (fullscreen by default).

Command line:

| Option | Meaning |
| --- | --- |
| `--windowed` | start in a window (F11 toggles) |
| `--data <dir>` | where the data files are |
| `--level N` | start in level 1-5, skipping the title screen |
| `--room N` | start room (0-63) with `--level` |

File names are matched without regard to case.
