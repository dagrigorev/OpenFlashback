# File Formats

What OpenFlashback knows about the DOS data files. Multi-byte values are little-endian unless
noted.

## Full-screen pictures: `menuN.map`, `instru_f.map` + `.pal`

57344 bytes: four planes of 64 x 224 bytes. Plane `i` holds the pixels whose x is `i` modulo 4
(VGA "mode X"). The `.pal` is 256 RGB triplets of 8-bit components.

## Level backgrounds: `levelN.map` + `levelN.pal`

A table of 64 entries `(u32 offset, u16 size)`, offset 0 meaning no room. A room is four
palette slot bytes, then four bands of 256 x 56 pixels, top to bottom, each a `u16` packed size
followed by byte run-length data: a control byte `c < 0x80` copies `c + 1` bytes, otherwise the
next byte repeats `257 - c` times.

Pixel bit 7 selects the palette in slot 1 (foreground) rather than slot 0; the low nibble is
the colour. `levelN.pal` is a list of 16-colour palettes of big-endian `0RGB` 12-bit words.

## Bytekiller compression

Used by `.ct` and other level files. Read backwards in big-endian 32-bit words: the last word
is the unpacked size, then a checksum, then the first bit buffer. Bits are taken least
significant first. Codes: `00` + 3 bits: literal run of n + 1 bytes; `01` + 8-bit offset: copy
2 bytes; `111` + 8 bits: literal run of n + 9; `110` + 8-bit count + 12-bit offset: copy
count + 1; `101` + 10-bit offset: copy 4; `100` + 9-bit offset: copy 3. The XOR of all words
read must be zero.

## Room connections: `levelN.ct` (bytekiller)

0x1d00 bytes: 64 bytes each of the room above, below, to the right and to the left of every
room, then a 16 x 7 collision grid per room: columns of 16 pixels; rows 2, 4 and 6 are the ground
of the three floors (feet at y 70, 142, 214), rows 1, 3 and 5 the space above each, where a set
cell is a wall.

## Animations: `levelN.ani` (levels 4 and 5: `level4_1.ani`, ...)

A `u16` count, then a `u16` offset per animation counted from byte 2. An animation is a frame
count byte, five header bytes, then 4 bytes per frame: `u16` sprite number (bit 15 mirrors it),
signed `dx` and `dy` moved when the frame is shown, for a character facing left. Conrad's
animations have the same numbers in every level: 0 turn, 1 stand, 3 and 2 the walking step,
8 start running, 6 / 17 / 7 / 18 the running cycle, 10 stop running, 4 jump up, 5 land from it,
16 pull up onto a ledge, 15 lower down from one, 13 and 21 standing and running take-off, then
22, 23, 24 in the air and landing. Every move goes a whole 16-pixel cell.

## Objects: `levelN.pge`

A `u16` count, then 32-byte records; record 0 is Conrad. Known fields: `u16` starting animation
(0), `i16` x (2), `i16` y (4), `u16` object node (6), `i16` life (8), object type (byte 18),
room (byte 19).

## Character sprites: `perso.spr` + `perso.off` (also `junky`, `mercenai`, `replican`, `glue`)

`.off` lists `(u16 id, u32 offset)` pairs ending with id `0xffff`; offsets count from byte 12 of
the `.spr`. A frame: 4 header bytes, a big-endian `u16` count of packed bytes, then 4-bit
pixels, high nibble first. Nibble 15 starts a run: `15, colour, n` repeats `n + 4` times;
`15, 15, hi, lo, colour` repeats `(hi << 4 | lo) + 4` times. Colour 0 is transparent.

Header bytes 0 and 1 are the anchor: the character's position (his feet) lies that many pixels
right of and below the top-left corner of the picture. Sprites face left; mirrored, the anchor
counts from the right edge. Header bytes 2 and 3 are width and height. If byte 2 has bit
`0x40`, pixels are stored column by column; the height is then `byte2 & 0x3f` and the width
byte 3.

## Conrad's palette: `FB.EXE`

Seven 16-colour palettes of little-endian `0RGB` words, each beginning with black; the fifth
is Conrad's normal colours. OpenFlashback finds the table by that shape.

## Object scripts: `levelN.obj` (levels 4 and 5: `level4_1.obj`, ...)

A `u16` count of node slots and a `u32` offset per slot, counted from byte 2; several slots can
share one node (Conrad's `.pge` record names node 1, which shares node 0's script). A node is a
`u16` record count and 18-byte records, each a transition between states (a state is an
animation number):

| Bytes | Field |
| --- | --- |
| 0-1 | state the transition leaves from |
| 2, 3 | signed `dx`, `dy` moved when it is taken (`dy` -72 = one floor up) |
| 4-5 | next state |
| 6 | condition opcode 2 |
| 7 | condition opcode 1 |
| 8 | flags (0x01 flips the facing after a turn) |
| 9 | action opcode |
| 10-11 | index of the next state's first record |
| 12-17 | three `i16` arguments |

When an animation ends, the records of its state are tried in order and the first whose
conditions hold is taken; the last record of a state usually has no condition. Condition
opcodes 1 to 8 test the controls relative to the facing: up, back, down, forward, then the same
with the modifier (Shift). Conrad's script confirms: stand + back turns (0, then 183), stand +
forward + Shift starts running (8, then 18), stand + up jumps (4, then 16 pulling up one floor
when a ledge test holds, else 5), stand + down crouches (64, 66 while held, 65), stand + down +
Shift climbs down (15, then 91 one floor lower), walking alternates 3 and 2 on forward. The
other condition opcodes (about 60, many with an expected value in the arguments, apparently grid
and object tests) and the action opcodes are not decoded yet.

## Cutscene pictures: `NAME.pol`

Big-endian. Five `u32` offsets: shape offset table, palettes, vertex offset table, shape data,
vertex data. Palettes are 16 `0RGB` words each. A shape (`u16` offset from the table, into the
shape data) is a `u16` primitive count, then per primitive a `u16` vertex-set number (bit 15:
an `i16` x and y offset follow; low 14 bits index the vertex offset table) and a colour byte. A
vertex set starts with a count byte `n`: bit 7 set is an ellipse (`i16` centre x, y, radius x,
y), 0 is a point (`i16` x, y), otherwise a polygon: `i16` x, y of the first point and `n` pairs of
signed byte steps to the next. Coordinates are relative to the 240 x 128 cutscene window, which
OpenFlashback centres at (8, 50).

## Cutscene scripts: `NAME.cmd`

Big-endian. A `u16` count of sub-scripts and their `u16` offsets (counted from the end of that
table; count 0 means one script right after it), then byte code: an opcode byte equal to the
opcode times 4 (a byte of `0x80` or more ends the script) and its arguments:

| Opcode | Arguments | Meaning |
| --- | --- | --- |
| 0, 5 | - | show the frame drawn so far |
| 1 | byte mode | 0: draw into the front layer (palette bank 1, cleared after every frame); otherwise draw into the background (bank 0), which is cleared |
| 2 | byte n | hold the frame for n more frames |
| 3 | `u16` shape (bit 15: `i16` x, y follow) | draw a shape |
| 4 | byte palette, byte bank | load a `.pol` palette into bank 0 or 1 |
| 6 | `u16` | caption: text number shown under the window until replaced (`0xffff` clears) |
| 7 | - | nothing |
| 8 | 3 bytes | skipped |
| 9 | - | keep the current picture as the background |
| 10 | `u16` shape (bit 15: x, y), `i16` zoom (+512 = 1:1), byte origin x, y | scaled shape |
| 11 | `u16` shape (bit 15: x, y; 14: zoom; 13, 12: extra angles), byte origin x, y, `u16` angle | rotated shape |
| 12 | - | keep the current picture as the background |
| 13 | `u16` text (unless `0xffff`: byte x, y) | text drawn into the front layer at character cell x, y of the window; low 12 bits text number, top 4 bits colour in bank 1 |
| 14 | ... | interactive menu keys (playback stops) |

All 62 scripts of the DOS version parse with these sizes. Unknown: the meaning of the two extra
rotation angles, the exact frame rate (12 frames per second is a guess), the caption colours
and placement (centred under the window here).

## Font and texts: `fb_txt.fnt`, `fr_cine.txt`, `fr_cine.bin`

The font is 8 x 8 glyphs from the space character on, 32 bytes each: 4 bits per pixel, high
nibble first, values 15 (face), 8 and 2 (shading), 0 transparent. `fr_cine.txt` holds one text
per line (English in this copy, despite the name), `|` breaking it into lines; `fr_cine.bin` is
the big-endian `u16` offset of each of its 117 texts.

## Music: `NAME.prf`, `.mid`, `.ins`

The songs are Standard MIDI Files (format 1). A 753-byte `.prf` profile goes with each: 16
instrument names of 30 bytes, one per MIDI **track** (track 0 is the tempo track; songs often
put every track on channel 0), little-endian `i16` transposes in semitones per track at offset
480, `i16` volume adjustments at 512 (not used yet), and the song's file name at 550. Profiles of
other sizes (`recharge.prf`) or without instruments (`opt.prf`) are for other sound drivers.

An `.ins` file is the 80-byte AdLib instrument format: mode byte (0 melodic, 1 percussive),
voice byte, 13 little-endian words for the modulator and 13 for the carrier (key-scale level,
multiplier, feedback, attack, sustain level, sustaining envelope, decay, release, total level,
tremolo, vibrato, key-scale rate, connection, where connection 1 is FM), then the modulator
and carrier waveforms. The carrier's feedback and connection words are unused and hold junk.

Which song plays with which cutscene is matched by name (`logos` with `present3`, `intro1`
with `introl3`, `memo` with `memory3`, ...).

## Not decoded yet

`.bnq`, the `.obj` condition and action opcodes beyond the controls, the rest of `.pge`, `.lev`, `.mbk`, `.sgd`, `.rp`, `.tbn`, `global.*` (sound effects in `global.fib`), the `.prf` volume and remaining per-track values.
