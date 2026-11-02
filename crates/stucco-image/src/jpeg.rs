use crate::{Error, Limits};

// Strict marker framing, including entropy byte stuffing, restart markers and EOI.
// Only 8-bit baseline grayscale or RGB JPEG is supported. APP/COM
// metadata is preserved but never trusted/interpreted by this API. MPF excluded.
fn envelope(input: &[u8], limits: Limits) -> Result<(), Error> {
    if !input.starts_with(&[0xff, 0xd8]) {
        return Err(Error::Unsupported);
    }
    let mut p = 2;
    let mut frame = None;
    let mut tables = crate::jpeg_entropy::Tables::default();
    let mut quantization_tables = [false; 4];
    let mut restart = 0;
    let mut markers = 0;
    let mut saw_scan = false;
    loop {
        if input.get(p) != Some(&0xff) {
            return Err(Error::Invalid);
        }
        p += 1;
        while input.get(p) == Some(&0xff) {
            p += 1;
        }
        let m = *input.get(p).ok_or(Error::Invalid)?;
        p += 1;

        markers += 1;
        if markers > limits.max_chunks {
            return Err(Error::Limit);
        }
        if m == 0xd9 {
            return if !saw_scan {
                Err(Error::Invalid)
            } else if p != input.len() {
                Err(Error::TrailingData)
            } else {
                Ok(())
            };
        }
        if !matches!(m, 0xc0 | 0xc4 | 0xdb | 0xdd | 0xda | 0xe0..=0xef | 0xfe) {
            return Err(Error::Unsupported);
        }
        let len_bytes = input.get(p..p + 2).ok_or(Error::Invalid)?;
        let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
        if len < 2 {
            return Err(Error::Invalid);
        }
        let end = p.checked_add(len).ok_or(Error::Invalid)?;
        let data = input.get(p + 2..end).ok_or(Error::Invalid)?;
        if m == 0xe2 && data.starts_with(b"MPF\0") {
            return Err(Error::Unsupported);
        }
        if m == 0xc0 && (data.first() != Some(&8) || !matches!(data.get(5), Some(1 | 3))) {
            return Err(Error::Unsupported);
        }
        p = end;
        match m {
            0xc0 => {
                if frame.is_some() {
                    return Err(Error::Invalid);
                }
                frame = Some(data);
            }
            0xc4 => tables.insert(data)?,
            0xdb => {
                // Baseline 8-bit DCT permits only 8-bit, nonzero quantizers.
                // Validate every definition, including unused/redefined tables.
                if data.is_empty() {
                    return Err(Error::Invalid);
                }
                let mut remaining = data;
                while !remaining.is_empty() {
                    let selector = remaining[0];
                    if selector >> 4 != 0 || selector & 0x0f > 3 {
                        return Err(Error::Invalid);
                    }
                    let values = remaining.get(1..65).ok_or(Error::Invalid)?;
                    if values.contains(&0) {
                        return Err(Error::Invalid);
                    }
                    quantization_tables[usize::from(selector)] = true;
                    remaining = &remaining[65..];
                }
            }
            0xdd => {
                if data.len() != 2 {
                    return Err(Error::Invalid);
                }
                restart = u16::from_be_bytes([data[0], data[1]]);
            }
            0xda => {
                if saw_scan {
                    return Err(Error::Unsupported);
                }
                let frame = frame.ok_or(Error::Invalid)?;
                if frame.len() < 6 {
                    return Err(Error::Invalid);
                }
                let components = usize::from(frame[5]);
                if frame.len() != 6 + 3 * components {
                    return Err(Error::Invalid);
                }
                for component in frame[6..].chunks_exact(3) {
                    if quantization_tables.get(usize::from(component[2])) != Some(&true) {
                        return Err(Error::Invalid);
                    }
                }
                let width = u32::from(u16::from_be_bytes([frame[3], frame[4]]));
                let height = u32::from(u16::from_be_bytes([frame[1], frame[2]]));
                if width == 0 || height == 0 {
                    return Err(Error::Invalid);
                }
                if width > limits.max_width
                    || height > limits.max_height
                    || u64::from(width) * u64::from(height) > limits.max_pixels
                {
                    return Err(Error::Limit);
                }
                p += crate::jpeg_entropy::scan(&input[p..], frame, data, &tables, restart)?;
                saw_scan = true;
            }
            _ => {}
        }
    }
}

pub(crate) fn decode(input: &[u8], limits: Limits) -> Result<(u32, u32), Error> {
    if input.len() > limits.max_input_bytes {
        return Err(Error::Limit);
    }
    envelope(input, limits)?;
    let options = zune_core::options::DecoderOptions::default()
        .set_strict_mode(true)
        .set_max_width(limits.max_width as usize)
        .set_max_height(limits.max_height as usize)
        .jpeg_set_max_scans(limits.max_chunks)
        .jpeg_set_out_colorspace(zune_core::colorspace::ColorSpace::RGB);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(
        zune_core::bytestream::ZCursor::new(input),
        options,
    );
    decoder.decode_headers().map_err(|_| Error::Invalid)?;
    let info = decoder.info().ok_or(Error::Invalid)?;
    let width = u32::from(info.width);
    let height = u32::from(info.height);
    let pixels = u64::from(width) * u64::from(height);
    let bytes = pixels.checked_mul(3).ok_or(Error::Limit)?;
    let work = bytes
        .checked_mul(3)
        .and_then(|v| v.checked_add(input.len() as u64))
        .ok_or(Error::Limit)?;
    if width == 0 || height == 0 {
        return Err(Error::Invalid);
    }
    if width > limits.max_width
        || height > limits.max_height
        || pixels > limits.max_pixels
        || bytes > limits.max_decoded_bytes as u64
        || work > limits.max_work_units
    {
        return Err(Error::Limit);
    }
    let mut output = vec![0; usize::try_from(bytes).map_err(|_| Error::Limit)?];
    decoder
        .decode_into(&mut output)
        .map_err(|_| Error::Invalid)?;
    if output.len() as u64 != bytes {
        return Err(Error::Invalid);
    }
    Ok((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn baseline_sampling_restart_grayscale_and_strict_entropy() {
        use jpeg_encoder::{ColorType, Encoder, SamplingFactor};
        for (width, height) in [(1, 1), (17, 19), (64, 48)] {
            for color in [ColorType::Rgb, ColorType::Luma] {
                for sampling in [
                    SamplingFactor::F_1_1,
                    SamplingFactor::F_2_1,
                    SamplingFactor::F_2_2,
                ] {
                    let mut jpeg = Vec::new();
                    let mut e = Encoder::new(&mut jpeg, 85);
                    e.set_sampling_factor(sampling);
                    e.set_restart_interval(2);
                    let channels = if color == ColorType::Luma { 1 } else { 3 };
                    let data = (0..usize::from(width) * usize::from(height) * channels)
                        .map(|i| (i * 73) as u8)
                        .collect::<Vec<_>>();
                    e.encode(&data, width, height, color).unwrap();
                    assert_eq!(
                        decode(&jpeg, crate::tests::limits()).unwrap(),
                        (u32::from(width), u32::from(height))
                    );
                    let sos = jpeg.windows(2).position(|v| v == [0xff, 0xda]).unwrap();
                    let start =
                        sos + 2 + usize::from(u16::from_be_bytes([jpeg[sos + 2], jpeg[sos + 3]]));
                    // Every shortened entropy stream is rejected even if an attacker
                    // supplies a clean EOI marker instead of simply truncating the file.
                    for cut in start..jpeg.len() - 2 {
                        let mut bad = jpeg[..cut].to_vec();
                        bad.extend([0xff, 0xd9]);
                        assert!(
                            decode(&bad, crate::tests::limits()).is_err(),
                            "cut {cut} of {}",
                            jpeg.len()
                        );
                    }
                }
            }
        }
    }
}
