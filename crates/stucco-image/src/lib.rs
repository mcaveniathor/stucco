//! Opt-in restricted static image validation. `Validator` is the original in-process
//! PNG API; [`isolated::IsolatedValidator`] adds baseline JPEG and a Linux x86-64
//! resource-limited worker. See the README for metadata and format restrictions.
pub mod isolated;
mod jpeg;
mod jpeg_entropy;

use std::{fmt, io::Cursor, sync::Arc};

use bytes::Bytes;
use flate2::{Decompress, FlushDecompress, Status};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};

/// Animation is never decoded or silently reduced to its first frame.
#[derive(Clone, Copy, Debug)]
pub enum AnimationPolicy {
    /// Reject all APNG chunks, including one-frame animations.
    Reject,
}

/// No ignored payload after the image or its compressed stream.
#[derive(Clone, Copy, Debug)]
pub enum TrailingDataPolicy {
    /// Reject bytes after IEND and after the IDAT zlib stream.
    Reject,
}

/// All bounds are application-supplied; there are no policy defaults.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_input_bytes: usize,
    pub max_width: u32,
    pub max_height: u32,
    pub max_pixels: u64,
    /// Full unfiltered output buffer, not including input or decoder internals.
    pub max_decoded_bytes: usize,
    /// Passed to `png::Limits`; this is not a process-wide allocator limit.
    pub png_allocation_bytes: usize,
    pub max_chunks: usize,
    /// Must be exactly one: this implementation only accepts static images.
    pub max_frames: u32,
    /// Input bytes + twice filtered scanline bytes + unfiltered output bytes.
    /// Accounts for strict zlib validation and the subsequent PNG full decode;
    /// not instructions, wall time or protection against decoder implementation bugs.
    pub max_work_units: u64,
    pub max_concurrent: usize,
    pub animation: AnimationPolicy,
    pub trailing_data: TrailingDataPolicy,
}

/// Validated format/dimensions; the original input is not sanitized or re-encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidatedImage {
    pub width: u32,
    pub height: u32,
}
impl ValidatedImage {
    pub const fn media_type(&self) -> &'static str {
        "image/png"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Configuration,
    Busy,
    Closed,
    Limit,
    Unsupported,
    Invalid,
    TrailingData,
    Worker,
    Deadline,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "image validation: {self:?}")
    }
}
impl std::error::Error for Error {}

/// A bounded decoder pool. Share one clone across a trust boundary; making a new
/// pool per request defeats admission. No unbounded wait queue is maintained.
#[derive(Clone)]
pub struct Validator {
    limits: Limits,
    slots: Arc<Semaphore>,
    changed: Arc<Notify>,
}

struct WorkPermit {
    slot: Option<OwnedSemaphorePermit>,
    changed: Arc<Notify>,
}
impl Drop for WorkPermit {
    fn drop(&mut self) {
        drop(self.slot.take());
        self.changed.notify_waiters();
    }
}
impl Validator {
    pub fn new(limits: Limits) -> Result<Self, Error> {
        if limits.max_input_bytes == 0
            || limits.max_width == 0
            || limits.max_height == 0
            || limits.max_pixels == 0
            || limits.max_decoded_bytes == 0
            || limits.png_allocation_bytes == 0
            || limits.max_chunks == 0
            || limits.max_frames != 1
            || limits.max_work_units == 0
            || limits.max_concurrent == 0
            || limits.max_concurrent > Semaphore::MAX_PERMITS
        {
            return Err(Error::Configuration);
        }
        Ok(Self {
            limits,
            slots: Arc::new(Semaphore::new(limits.max_concurrent)),
            changed: Arc::new(Notify::new()),
        })
    }

    /// Fully decode an admitted image. Requires a Tokio runtime. An abandoned
    /// caller does not stop blocking work or release its slot. Do not wrap this in
    /// a timeout and interpret timeout as decoder settlement; use `close_and_drain`.
    /// Input must already have been read with a bounded, separately admitted read.
    pub async fn validate(&self, input: Bytes) -> Result<ValidatedImage, Error> {
        if input.len() > self.limits.max_input_bytes {
            return Err(Error::Limit);
        }
        let limits = self.limits;
        self.run(move || decode(&input, limits)).await
    }

    async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let slot = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|e| match e {
                tokio::sync::TryAcquireError::Closed => Error::Closed,
                tokio::sync::TryAcquireError::NoPermits => Error::Busy,
            })?;
        let guard = WorkPermit {
            slot: Some(slot),
            changed: self.changed.clone(),
        };
        tokio::task::spawn_blocking(move || {
            let _guard = guard;
            work()
        })
        .await
        .map_err(|_| Error::Worker)?
    }

    /// Stop admission on every clone and wait for all admitted blocking work.
    /// Cancellation of this drain does not reopen admission. Call again to finish.
    pub async fn close_and_drain(&self) {
        self.slots.close();
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.slots.available_permits() == self.limits.max_concurrent {
                break;
            }
            changed.await;
        }
    }
}

struct Shape {
    width: u32,
    height: u32,
    decoded: usize,
    filtered: u64,
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes.try_into().expect("checked four-byte field"))
}

fn decode(input: &[u8], limits: Limits) -> Result<ValidatedImage, Error> {
    if input.len() > limits.max_input_bytes {
        return Err(Error::Limit);
    }
    if !input.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::Unsupported);
    }
    let mut pos = 8usize;
    let mut count = 0usize;
    let mut shape: Option<Shape> = None;
    let mut saw_idat = false;
    let mut inflater = Decompress::new(true);
    let mut stream_end = false;
    let mut scratch = [0u8; 8192];
    loop {
        count = count.checked_add(1).ok_or(Error::Limit)?;
        if count > limits.max_chunks {
            return Err(Error::Limit);
        }
        let header = input
            .get(pos..pos.checked_add(8).ok_or(Error::Invalid)?)
            .ok_or(Error::Invalid)?;
        let len = be32(&header[..4]) as usize;
        let kind = &header[4..8];
        let end = pos
            .checked_add(12)
            .and_then(|n| n.checked_add(len))
            .ok_or(Error::Invalid)?;
        let chunk = input.get(pos..end).ok_or(Error::Invalid)?;
        let data = &chunk[8..8 + len];
        if crc32fast::hash(&chunk[4..8 + len]) != be32(&chunk[8 + len..]) {
            return Err(Error::Invalid);
        }
        match kind {
            b"IHDR" if shape.is_none() && count == 1 && len == 13 => {
                let width = be32(&data[..4]);
                let height = be32(&data[4..8]);
                // Deliberately restricted profile: RGB/RGBA8, no interlace,
                // metadata, palettes, animation, or alternate compression/filter.
                if data[8] != 8 || !matches!(data[9], 2 | 6) || data[10..] != [0, 0, 0] {
                    return Err(Error::Unsupported);
                }
                if width == 0 || height == 0 {
                    return Err(Error::Invalid);
                }
                let pixels = u64::from(width) * u64::from(height);
                let channels = if data[9] == 2 { 3 } else { 4 };
                let decoded = pixels.checked_mul(channels).ok_or(Error::Limit)?;
                let filtered = decoded.checked_add(u64::from(height)).ok_or(Error::Limit)?;
                let work = filtered
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(decoded))
                    .and_then(|n| n.checked_add(input.len() as u64))
                    .ok_or(Error::Limit)?;
                if width > limits.max_width
                    || height > limits.max_height
                    || pixels > limits.max_pixels
                    || decoded > limits.max_decoded_bytes as u64
                    || work > limits.max_work_units
                {
                    return Err(Error::Limit);
                }
                shape = Some(Shape {
                    width,
                    height,
                    decoded: usize::try_from(decoded).map_err(|_| Error::Limit)?,
                    filtered,
                });
            }
            b"IDAT" => {
                let shape = shape.as_ref().ok_or(Error::Invalid)?;
                saw_idat = true;
                let mut offset = 0;
                // Even empty input may need to flush pending decompressed output.
                loop {
                    if stream_end {
                        if offset != data.len() {
                            return Err(Error::TrailingData);
                        }
                        break;
                    }
                    let before_in = inflater.total_in();
                    let before_out = inflater.total_out();
                    let status = inflater
                        .decompress(&data[offset..], &mut scratch, FlushDecompress::None)
                        .map_err(|_| Error::Invalid)?;
                    offset += (inflater.total_in() - before_in) as usize;
                    if inflater.total_out() > shape.filtered {
                        return Err(Error::Limit);
                    }
                    if status == Status::StreamEnd {
                        stream_end = true;
                        continue;
                    }
                    if inflater.total_in() == before_in && inflater.total_out() == before_out {
                        if offset != data.len() {
                            return Err(Error::Invalid);
                        }
                        break;
                    }
                }
            }
            b"IEND" if len == 0 => {
                let shape = shape.ok_or(Error::Invalid)?;
                if !saw_idat || !stream_end || inflater.total_out() != shape.filtered {
                    return Err(Error::Invalid);
                }
                if end != input.len() {
                    return Err(Error::TrailingData);
                }
                let decoder = png::Decoder::new_with_limits(
                    Cursor::new(input),
                    png::Limits {
                        bytes: limits.png_allocation_bytes,
                    },
                );
                let mut reader = decoder.read_info().map_err(png_error)?;
                if reader.output_buffer_size() != Some(shape.decoded) {
                    return Err(Error::Invalid);
                }
                let mut pixels = Vec::new();
                pixels
                    .try_reserve_exact(shape.decoded)
                    .map_err(|_| Error::Limit)?;
                pixels.resize(shape.decoded, 0);
                let output = reader.next_frame(&mut pixels).map_err(png_error)?;
                if output.width != shape.width
                    || output.height != shape.height
                    || output.buffer_size() != shape.decoded
                {
                    return Err(Error::Invalid);
                }
                reader.finish().map_err(png_error)?;
                return Ok(ValidatedImage {
                    width: shape.width,
                    height: shape.height,
                });
            }
            b"acTL" | b"fcTL" | b"fdAT" => return Err(Error::Unsupported),
            b"IHDR" | b"IEND" => return Err(Error::Invalid),
            _ => return Err(Error::Unsupported),
        }
        pos = end;
    }
}

fn png_error(error: png::DecodingError) -> Error {
    match error {
        png::DecodingError::LimitsExceeded => Error::Limit,
        _ => Error::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    pub(crate) fn limits() -> Limits {
        Limits {
            max_input_bytes: 1 << 20,
            max_width: 1024,
            max_height: 1024,
            max_pixels: 1 << 20,
            max_decoded_bytes: 4 << 20,
            png_allocation_bytes: 8 << 20,
            max_chunks: 20,
            max_frames: 1,
            max_work_units: 32 << 20,
            max_concurrent: 1,
            animation: AnimationPolicy::Reject,
            trailing_data: TrailingDataPolicy::Reject,
        }
    }
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = (data.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&crc32fast::hash(&out[4..]).to_be_bytes());
        out
    }
    fn compressed(raw: &[u8]) -> Vec<u8> {
        let mut w = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        w.write_all(raw).unwrap();
        w.finish().unwrap()
    }
    fn make(width: u32, height: u32, color: u8, raw: &[u8]) -> Vec<u8> {
        with_zlib(width, height, color, &compressed(raw))
    }
    fn with_zlib(width: u32, height: u32, color: u8, zlib: &[u8]) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = width.to_be_bytes().to_vec();
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, color, 0, 0, 0]);
        out.extend(chunk(b"IHDR", &ihdr));
        out.extend(chunk(b"IDAT", zlib));
        out.extend(chunk(b"IEND", &[]));
        out
    }
    fn valid() -> Vec<u8> {
        make(1, 1, 6, &[0, 255, 0, 0, 255])
    }

    #[tokio::test]
    async fn real_full_decode_rgb_rgba_and_filters() {
        let validator = Validator::new(limits()).unwrap();
        for color in [2, 6] {
            let mut raw = Vec::new();
            for filter in 0..=4 {
                raw.push(filter);
                raw.extend(vec![42; if color == 2 { 6 } else { 8 }]);
            }
            let image = validator
                .validate(make(2, 5, color, &raw).into())
                .await
                .unwrap();
            assert_eq!(
                image,
                ValidatedImage {
                    width: 2,
                    height: 5
                }
            );
            assert_eq!(image.media_type(), "image/png");
        }
        validator.close_and_drain().await;
        assert_eq!(validator.validate(valid().into()).await, Err(Error::Closed));
    }

    #[test]
    fn rejects_every_truncation_and_corrupt_crc() {
        let image = valid();
        for end in 0..image.len() {
            assert!(decode(&image[..end], limits()).is_err(), "truncation {end}");
        }
        let mut bad = image;
        bad[29] ^= 1;
        assert_eq!(decode(&bad, limits()), Err(Error::Invalid));
    }

    #[test]
    fn rejects_invalid_filter_despite_valid_chunk_and_zlib_checksums() {
        assert_eq!(
            decode(&make(1, 1, 6, &[5, 1, 2, 3, 4]), limits()),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn rejects_trailing_file_and_zlib_data() {
        let mut trailing = valid();
        trailing.extend_from_slice(b"payload");
        assert_eq!(decode(&trailing, limits()), Err(Error::TrailingData));
        let mut zlib = compressed(&[0, 1, 2, 3, 4]);
        zlib.extend_from_slice(b"payload");
        assert_eq!(
            decode(&with_zlib(1, 1, 6, &zlib), limits()),
            Err(Error::TrailingData)
        );
        let mut concatenated = compressed(&[0, 1, 2, 3, 4]);
        concatenated.extend(compressed(&[]));
        assert_eq!(
            decode(&with_zlib(1, 1, 6, &concatenated), limits()),
            Err(Error::TrailingData)
        );
    }

    #[test]
    fn rejects_animation_and_compressed_metadata() {
        for kind in [b"acTL", b"fcTL", b"fdAT", b"iCCP", b"zTXt", b"tEXt"] {
            let mut image = valid();
            image.splice(33..33, chunk(kind, &[0; 8]));
            assert_eq!(decode(&image, limits()), Err(Error::Unsupported));
        }
    }

    #[test]
    fn dimension_and_decompression_bombs_fail_closed() {
        assert_eq!(
            decode(&make(u32::MAX, u32::MAX, 6, &[]), limits()),
            Err(Error::Limit)
        );
        assert_eq!(
            decode(&make(1, 1, 6, &vec![0; 1 << 20]), limits()),
            Err(Error::Limit)
        );
        assert_eq!(
            decode(&make(1, 1, 6, &[0; 4]), limits()),
            Err(Error::Invalid)
        );
        assert_eq!(decode(&make(0, 1, 6, &[]), limits()), Err(Error::Invalid));
    }

    #[test]
    fn rejects_bad_zlib_checksum_and_incomplete_zlib() {
        let mut zlib = compressed(&[0, 1, 2, 3, 4]);
        *zlib.last_mut().unwrap() ^= 1;
        assert_eq!(
            decode(&with_zlib(1, 1, 6, &zlib), limits()),
            Err(Error::Invalid)
        );
        zlib.pop();
        assert_eq!(
            decode(&with_zlib(1, 1, 6, &zlib), limits()),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn explicit_limits_are_enforced_independently() {
        let image = make(2, 2, 6, &[0; 18]);
        for field in 0..8 {
            let mut l = limits();
            match field {
                0 => l.max_input_bytes = image.len() - 1,
                1 => l.max_width = 1,
                2 => l.max_height = 1,
                3 => l.max_pixels = 3,
                4 => l.max_decoded_bytes = 15,
                5 => l.png_allocation_bytes = 1,
                6 => l.max_chunks = 2,
                _ => l.max_work_units = 1,
            }
            assert_eq!(decode(&image, l), Err(Error::Limit), "field {field}");
        }
    }

    #[test]
    fn exact_work_boundary_and_fragmented_idat() {
        let raw = [0, 1, 2, 3, 4];
        let zlib = compressed(&raw);
        let mut image = valid()[..33].to_vec();
        for byte in &zlib {
            image.extend(chunk(b"IDAT", &[*byte]));
        }
        image.extend(chunk(b"IDAT", &[]));
        image.extend(chunk(b"IEND", &[]));
        let mut l = limits();
        l.max_chunks = zlib.len() + 3;
        l.max_work_units = image.len() as u64 + 2 * 5 + 4;
        assert!(decode(&image, l).is_ok());
        l.max_work_units -= 1;
        assert_eq!(decode(&image, l), Err(Error::Limit));
    }

    #[test]
    fn configuration_requires_explicit_nonzero_limits_and_static_frames() {
        let mut l = limits();
        l.max_frames = 2;
        assert!(matches!(Validator::new(l), Err(Error::Configuration)));
        l = limits();
        l.max_concurrent = 0;
        assert!(matches!(Validator::new(l), Err(Error::Configuration)));
        l = limits();
        l.max_concurrent = usize::MAX;
        assert!(matches!(Validator::new(l), Err(Error::Configuration)));
    }

    #[tokio::test]
    async fn cancellation_retains_admission_and_drain_joins_work() {
        let validator = Validator::new(limits()).unwrap();
        let clone = validator.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let waiter = tokio::spawn(async move {
            clone
                .run(move || {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    decode(&valid(), limits())
                })
                .await
        });
        entered_rx.await.unwrap();
        waiter.abort();
        let _ = waiter.await;
        assert_eq!(validator.validate(valid().into()).await, Err(Error::Busy));
        let drain = validator.close_and_drain();
        tokio::pin!(drain);
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(drain.as_mut().poll(cx).is_pending()))
                .await
        );
        assert_eq!(validator.validate(valid().into()).await, Err(Error::Closed));
        release_tx.send(()).unwrap();
        drain.await;
        assert_eq!(validator.slots.available_permits(), 1);
    }

    #[tokio::test]
    async fn panic_and_invalid_input_release_capacity() {
        let validator = Validator::new(limits()).unwrap();
        assert_eq!(
            validator
                .run::<()>(|| panic!("injected worker failure"))
                .await,
            Err(Error::Worker)
        );
        assert_eq!(
            validator
                .validate(Bytes::from_static(b"not an image"))
                .await,
            Err(Error::Unsupported)
        );
        assert!(validator.validate(valid().into()).await.is_ok());
        validator.close_and_drain().await;
    }
    #[test]
    fn successful_large_inflater_boundaries() {
        for (width, height, color) in [(341, 8, 2), (683, 4, 2), (512, 9, 6)] {
            for noisy in [false, true] {
                let channels = if color == 2 { 3 } else { 4 };
                let row = width as usize * channels + 1;
                let mut raw = vec![0; row * height as usize];
                let mut state = 1234567u32;
                for (index, b) in raw.iter_mut().enumerate() {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    if index % row != 0 {
                        *b = if noisy { state as u8 } else { 42 };
                    }
                }
                assert!(raw.len() >= 8192);
                let data = compressed(&raw);
                let original = make(width, height, color, &raw);
                // Split every byte, including the zlib trailer, into a separate IDAT.
                let mut fragmented = original[..33].to_vec();
                for b in &data {
                    fragmented.extend(chunk(b"IDAT", &[*b]));
                }
                fragmented.extend(chunk(b"IEND", &[]));
                let mut l = limits();
                l.max_chunks = data.len() + 2;
                l.max_decoded_bytes = raw.len() - height as usize;
                l.max_work_units =
                    fragmented.len() as u64 + 2 * raw.len() as u64 + l.max_decoded_bytes as u64;
                assert_eq!(
                    decode(&fragmented, l).unwrap(),
                    ValidatedImage { width, height }
                );
                l.max_work_units -= 1;
                assert_eq!(decode(&fragmented, l), Err(Error::Limit));
            }
        }
    }
}
