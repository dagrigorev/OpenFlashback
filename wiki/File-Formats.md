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
room, then a 16 x 7 collision grid per room.

## Character sprites: `perso.spr` + `perso.off` (also `junky`, `mercenai`, `replican`, `glue`)

`.off` lists `(u16 id, u32 offset)` pairs ending with id `0xffff`; offsets count from byte 12 of
the `.spr`. A frame: 4 header bytes, a big-endian `u16` count of packed bytes, then 4-bit
pixels, high nibble first. Nibble 15 starts a run: `15, colour, n` repeats `n + 4` times;
`15, 15, hi, lo, colour` repeats `(hi << 4 | lo) + 4` times. Colour 0 is transparent.

Header bytes 2 and 3 are width and height. If byte 2 has bit `0x40`, pixels are stored column
by column; the height is then `byte2 & 0x3f` and the width byte 3.

## Conrad's palette: `FB.EXE`

Seven 16-colour palettes of little-endian `0RGB` words, each beginning with black; the fifth
is Conrad's normal colours. OpenFlashback finds the table by that shape.

## Not decoded yet

`.ani`, `.bnq`, `.pge`, `.obj`, `.lev`, `.mbk`, `.sgd`, `.rp`, `.tbn`, `global.*`, `.cmd` / `.pol`
cutscenes, `.prf` / `.mid` / `.ins` audio, `fb_txt.fnt`.
