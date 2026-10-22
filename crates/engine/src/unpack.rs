//! Delphine's "bytekiller" LZ decompression, used for the `.ct`, `.lev`, `.sgd`, `.bnq` and other files.
//!
//! A packed file is read backwards in big-endian 32-bit words. Its last three words are the unpacked
//! size, a checksum and the first bit-buffer word; the output is also written from the end. The
//! checksum is the XOR of every word read, which must come out zero.

use std::fmt;

/// Why a packed buffer could not be expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnpackError {
    /// Shorter than the 12-byte trailer, or the stream ran out early.
    Truncated,
    /// A back-reference pointed past the end of the output.
    BadReference,
    /// The data expanded, but the checksum does not match.
    Checksum,
}

impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            UnpackError::Truncated => "packed data is truncated",
            UnpackError::BadReference => "packed data refers outside the output",
            UnpackError::Checksum => "packed data checksum mismatch",
        };
        f.write_str(s)
    }
}

impl std::error::Error for UnpackError {}

struct Bits<'a> {
    src: &'a [u8],
    /// Byte index just past the next word to read (words are consumed towards the start).
    end: usize,
    word: u32,
    crc: u32,
}

impl Bits<'_> {
    fn read_word(&mut self) -> Result<u32, UnpackError> {
        if self.end < 4 {
            return Err(UnpackError::Truncated);
        }
        self.end -= 4;
        let s = &self.src[self.end..self.end + 4];
        Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn bit(&mut self) -> Result<u32, UnpackError> {
        let mut bit = self.word & 1;
        self.word >>= 1;
        if self.word == 0 {
            let w = self.read_word()?;
            self.crc ^= w;
            bit = w & 1;
            // The top bit marks the end of the buffer: once only it is left, the word is used up.
            self.word = 0x8000_0000 | (w >> 1);
        }
        Ok(bit)
    }

    fn bits(&mut self, count: u32) -> Result<usize, UnpackError> {
        let mut v = 0usize;
        for _ in 0..count {
            v = v << 1 | self.bit()? as usize;
        }
        Ok(v)
    }
}

/// Expands a bytekiller-packed buffer.
pub fn unpack(src: &[u8]) -> Result<Vec<u8>, UnpackError> {
    if src.len() < 12 {
        return Err(UnpackError::Truncated);
    }
    let mut bits = Bits {
        src,
        end: src.len(),
        word: 0,
        crc: 0,
    };
    let size = bits.read_word()? as usize;
    bits.crc = bits.read_word()?;
    bits.word = bits.read_word()?;
    bits.crc ^= bits.word;

    let mut out = vec![0u8; size];
    // Number of bytes still to write; output fills from index `left - 1` downwards.
    let mut left = size;
    while left > 0 {
        let (literal_bits, extra, ref_bits, count) = if bits.bit()? == 0 {
            if bits.bit()? == 0 {
                (3, 1, 0, 0)
            } else {
                (0, 0, 8, 2)
            }
        } else {
            match bits.bits(2)? {
                3 => (8, 9, 0, 0),
                2 => {
                    let n = bits.bits(8)? + 1;
                    (0, 0, 12, n)
                }
                1 => (0, 0, 10, 4),
                _ => (0, 0, 9, 3),
            }
        };
        if ref_bits == 0 {
            let n = (bits.bits(literal_bits)? + extra).min(left);
            for _ in 0..n {
                left -= 1;
                out[left] = bits.bits(8)? as u8;
            }
        } else {
            let n = count.min(left);
            let offset = bits.bits(ref_bits)?;
            for _ in 0..n {
                left -= 1;
                let from = left + offset;
                if from >= size {
                    return Err(UnpackError::BadReference);
                }
                out[left] = out[from];
            }
        }
    }
    if bits.crc != 0 {
        return Err(UnpackError::Checksum);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packs `data` as literal runs of at most 8 bytes, the simplest valid bytekiller stream.
    fn pack_literals(data: &[u8]) -> Vec<u8> {
        // Bits in the order the decoder consumes them.
        let mut stream: Vec<u32> = Vec::new();
        let push = |s: &mut Vec<u32>, v: usize, n: u32| {
            for i in (0..n).rev() {
                s.push((v >> i) as u32 & 1);
            }
        };
        let mut i = data.len();
        while i > 0 {
            let n = i.min(8);
            push(&mut stream, 0, 2);
            push(&mut stream, n - 1, 3);
            for k in 0..n {
                push(&mut stream, data[i - 1 - k] as usize, 8);
            }
            i -= n;
        }
        // The trailer's buffer word holds up to 31 bits below an end marker; every word read after
        // it holds 32 bits, least significant first.
        let head = stream.len().min(31);
        let mut first = 1u32 << head;
        for (k, b) in stream[..head].iter().enumerate() {
            first |= b << k;
        }
        let mut words = Vec::new();
        for chunk in stream[head..].chunks(32) {
            let mut w = 0u32;
            for (k, b) in chunk.iter().enumerate() {
                w |= b << k;
            }
            words.push(w);
        }
        let mut crc = first;
        for w in &words {
            crc ^= w;
        }
        let mut out = Vec::new();
        for w in words.iter().rev() {
            out.extend_from_slice(&w.to_be_bytes());
        }
        out.extend_from_slice(&first.to_be_bytes());
        out.extend_from_slice(&crc.to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out
    }

    #[test]
    fn literal_stream_round_trips() {
        let data: Vec<u8> = (0..100u32).map(|i| (i * 7 + 3) as u8).collect();
        assert_eq!(unpack(&pack_literals(&data)).unwrap(), data);
    }

    #[test]
    fn corrupt_checksum_is_reported() {
        let mut packed = pack_literals(b"flashback");
        let n = packed.len();
        packed[n - 5] ^= 1;
        assert_eq!(unpack(&packed), Err(UnpackError::Checksum));
    }

    #[test]
    fn short_input_is_truncated() {
        assert_eq!(unpack(&[0; 8]), Err(UnpackError::Truncated));
    }
}
