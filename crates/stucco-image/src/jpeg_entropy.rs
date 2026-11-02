//! Strict baseline entropy consumption. No coefficient reconstruction or IDCT here;
//! the maintained JPEG decoder still reconstructs every pixel after this pass.
use crate::Error;
#[derive(Clone, Default)]
pub(crate) struct Huffman {
    counts: [u8; 16],
    symbols: Vec<u8>,
}
impl Huffman {
    fn symbol(&self, bits: &mut Bits<'_>) -> Result<u8, Error> {
        let mut code = 0u32;
        let mut first = 0u32;
        let mut offset = 0;
        for n in self.counts {
            code = (code << 1) | bits.bit()?;
            if code >= first && code - first < u32::from(n) {
                return self
                    .symbols
                    .get(offset + (code - first) as usize)
                    .copied()
                    .ok_or(Error::Invalid);
            }
            offset += usize::from(n);
            first = (first + u32::from(n)) << 1;
        }
        Err(Error::Invalid)
    }
}
#[derive(Default)]
pub(crate) struct Tables {
    dc: [Huffman; 4],
    ac: [Huffman; 4],
}
impl Tables {
    pub(crate) fn insert(&mut self, mut data: &[u8]) -> Result<(), Error> {
        while !data.is_empty() {
            if data.len() < 17 {
                return Err(Error::Invalid);
            }
            let kind = data[0] >> 4;
            let index = usize::from(data[0] & 15);
            if kind > 1 || index > 3 {
                return Err(Error::Invalid);
            }
            let counts: [u8; 16] = data[1..17].try_into().unwrap();
            let count: usize = counts.iter().map(|v| usize::from(*v)).sum();
            let symbols = data.get(17..17 + count).ok_or(Error::Invalid)?;
            if count == 0 || count > 256 {
                return Err(Error::Invalid);
            }
            // Canonical codes must be neither oversubscribed nor all-ones codes
            // (all-ones bits are reserved for byte-boundary padding).
            let mut available = 1i32;
            for n in counts {
                available = available * 2 - i32::from(n);
                if available <= 0 {
                    return Err(Error::Invalid);
                }
            }
            let table = Huffman {
                counts,
                symbols: symbols.to_vec(),
            };
            if kind == 0 {
                self.dc[index] = table;
            } else {
                self.ac[index] = table;
            }
            data = &data[17 + count..];
        }
        Ok(())
    }
}
struct Bits<'a> {
    input: &'a [u8],
    p: usize,
    byte: u8,
    left: u8,
}
impl Bits<'_> {
    fn bit(&mut self) -> Result<u32, Error> {
        if self.left == 0 {
            self.byte = *self.input.get(self.p).ok_or(Error::Invalid)?;
            self.p += 1;
            if self.byte == 0xff {
                if self.input.get(self.p) != Some(&0) {
                    return Err(Error::Invalid);
                }
                self.p += 1;
            }
            self.left = 8;
        }
        self.left -= 1;
        Ok(u32::from((self.byte >> self.left) & 1))
    }
    fn skip(&mut self, n: u8) -> Result<(), Error> {
        for _ in 0..n {
            self.bit()?;
        }
        Ok(())
    }
    fn align(&mut self) -> Result<(), Error> {
        while self.left > 0 {
            if self.bit()? != 1 {
                return Err(Error::Invalid);
            }
        }
        Ok(())
    }
    fn marker(&mut self, expected: u8) -> Result<(), Error> {
        self.align()?;
        if self.input.get(self.p) != Some(&0xff) {
            return Err(Error::Invalid);
        }
        self.p += 1;
        while self.input.get(self.p) == Some(&0xff) {
            self.p += 1;
        }
        if self.input.get(self.p) != Some(&expected) {
            return Err(Error::Invalid);
        }
        self.p += 1;
        Ok(())
    }
}
pub(crate) fn scan(
    input: &[u8],
    frame: &[u8],
    header: &[u8],
    tables: &Tables,
    restart: u16,
) -> Result<usize, Error> {
    if frame.len() < 6 {
        return Err(Error::Invalid);
    }
    let components = usize::from(frame[5]);
    // Deliberate narrow contract: one interleaved baseline scan containing all
    // components, or one grayscale scan. Multiscan/progressive are unsupported.
    if header.first().copied().map(usize::from) != Some(components)
        || header.len() != 1 + 2 * components + 3
        || header[header.len() - 3..] != [0, 63, 0]
    {
        return Err(Error::Unsupported);
    }
    if frame.len() != 6 + 3 * components {
        return Err(Error::Invalid);
    }
    let width = usize::from(u16::from_be_bytes([frame[3], frame[4]]));
    let height = usize::from(u16::from_be_bytes([frame[1], frame[2]]));
    let mut blocks = Vec::new();
    let mut max_h = 0;
    let mut max_v = 0;
    let mut seen = Vec::new();
    for c in header[1..1 + 2 * components].chunks_exact(2) {
        if seen.contains(&c[0]) {
            return Err(Error::Invalid);
        }
        seen.push(c[0]);
        let f = frame[6..]
            .chunks_exact(3)
            .find(|v| v[0] == c[0])
            .ok_or(Error::Invalid)?;
        let h = usize::from(f[1] >> 4);
        let v = usize::from(f[1] & 15);
        if !(1..=4).contains(&h) || !(1..=4).contains(&v) || f[2] > 3 {
            return Err(Error::Invalid);
        }
        max_h = max_h.max(h);
        max_v = max_v.max(v);
        let dc = tables
            .dc
            .get(usize::from(c[1] >> 4))
            .ok_or(Error::Invalid)?;
        let ac = tables
            .ac
            .get(usize::from(c[1] & 15))
            .ok_or(Error::Invalid)?;
        if dc.symbols.is_empty() || ac.symbols.is_empty() {
            return Err(Error::Invalid);
        }
        blocks.push((if components == 1 { 1 } else { h * v }, dc, ac));
    }
    if components == 1 {
        max_h = 1;
        max_v = 1;
    }
    let mcus = width.div_ceil(8 * max_h) * height.div_ceil(8 * max_v);
    let mut bits = Bits {
        input,
        p: 0,
        byte: 0,
        left: 0,
    };
    let mut restart_index = 0;
    for mcu in 0..mcus {
        if mcu != 0 && restart != 0 && mcu % usize::from(restart) == 0 {
            bits.marker(0xd0 + restart_index)?;
            restart_index = (restart_index + 1) % 8;
        }
        for &(count, dc, ac) in &blocks {
            for _ in 0..count {
                let category = dc.symbol(&mut bits)?;
                if category > 11 {
                    return Err(Error::Invalid);
                }
                bits.skip(category)?;
                let mut coefficient = 1;
                while coefficient < 64 {
                    let symbol = ac.symbol(&mut bits)?;
                    if symbol == 0 {
                        break;
                    }
                    let zeros = usize::from(symbol >> 4);
                    let category = symbol & 15;
                    if category == 0 {
                        if zeros != 15 || coefficient + 16 > 64 {
                            return Err(Error::Invalid);
                        }
                        coefficient += 16;
                    } else {
                        coefficient += zeros;
                        if category > 10 || coefficient >= 64 {
                            return Err(Error::Invalid);
                        }
                        bits.skip(category)?;
                        coefficient += 1;
                    }
                }
            }
        }
    }
    bits.align()?;
    // Leave the next marker to the envelope parser, but disallow extra entropy.
    if input.get(bits.p) != Some(&0xff) || input.get(bits.p + 1) == Some(&0) {
        return Err(Error::Invalid);
    }
    Ok(bits.p)
}
