#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, Instant},
};
use stucco_image::{
    AnimationPolicy, Error, Limits, TrailingDataPolicy,
    isolated::{Format, IsolatedValidator, ProcessLimits},
};
fn limits() -> Limits {
    Limits {
        max_input_bytes: 1 << 20,
        max_width: 1024,
        max_height: 1024,
        max_pixels: 1 << 20,
        max_decoded_bytes: 4 << 20,
        png_allocation_bytes: 8 << 20,
        max_chunks: 100,
        max_frames: 1,
        max_work_units: 32 << 20,
        max_concurrent: 1,
        animation: AnimationPolicy::Reject,
        trailing_data: TrailingDataPolicy::Reject,
    }
}
fn policy() -> ProcessLimits {
    ProcessLimits {
        executable: PathBuf::from(env!("CARGO_BIN_EXE_stucco-image-worker")),
        address_space_bytes: 256 << 20,
        cpu_seconds: 2,
        deadline: Duration::from_secs(5),
    }
}
fn jpeg(progressive: bool) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut out, 90);
    encoder.set_progressive(progressive);
    encoder
        .encode(&[52; 32 * 16 * 3], 32, 16, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    out
}
fn png() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, 32, 16);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().unwrap();
        w.write_image_data(&[52; 32 * 16 * 3]).unwrap();
    }
    out
}
#[tokio::test]
async fn real_worker_full_decodes_png_and_baseline_jpeg() {
    let v = IsolatedValidator::new(limits(), policy()).unwrap();
    for (input, format) in [(png(), Format::Png), (jpeg(false), Format::Jpeg)] {
        let result = v.validate(input.into()).await.unwrap();
        assert_eq!(
            (result.width, result.height, result.format),
            (32, 16, format)
        );
    }
    assert_eq!(v.validate(jpeg(true).into()).await, Err(Error::Unsupported));
    v.close_and_drain().await;
    assert_eq!(v.validate(png().into()).await, Err(Error::Closed));
}
// One valid grayscale block: DC category zero, EOB, then one-bit padding.
// Independent of the encoder, so table mutations leave all other syntax intact.
fn quantized_block(tables: &[Vec<u8>], reference: u8) -> Vec<u8> {
    fn segment(out: &mut Vec<u8>, marker: u8, payload: &[u8]) {
        out.extend([0xff, marker]);
        out.extend(u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
        out.extend(payload);
    }
    let mut out = vec![0xff, 0xd8];
    for table in tables {
        segment(&mut out, 0xdb, table);
    }
    segment(&mut out, 0xc0, &[8, 0, 8, 0, 8, 1, 1, 0x11, reference]);
    let mut huffman = Vec::new();
    for kind in [0, 0x10] {
        huffman.extend([kind, 1]);
        huffman.extend([0; 16]); // Fifteen other code lengths, then symbol zero.
    }
    segment(&mut out, 0xc4, &huffman);
    segment(&mut out, 0xda, &[1, 1, 0, 0, 63, 0]);
    out.extend([0x3f, 0xff, 0xd9]);
    out
}
fn quantizers(destination: u8, value: u8) -> Vec<u8> {
    let mut table = vec![destination];
    table.extend([value; 64]);
    table
}
#[tokio::test]
async fn jpeg_dqt_valid_control_multiple_and_redefined_tables() {
    let v = IsolatedValidator::new(limits(), policy()).unwrap();
    let combined = [quantizers(0, 1), quantizers(3, 255)].concat();
    for (tables, reference) in [
        (vec![quantizers(0, 1)], 0),
        (vec![combined], 3),
        (vec![quantizers(0, 1), quantizers(0, 2)], 0),
    ] {
        let result = v
            .validate(quantized_block(&tables, reference).into())
            .await
            .unwrap();
        assert_eq!(
            (result.width, result.height, result.format),
            (8, 8, Format::Jpeg)
        );
    }
    v.close_and_drain().await;
}
#[tokio::test]
async fn jpeg_dqt_rejects_invalid_definitions_even_when_unused() {
    let v = IsolatedValidator::new(limits(), policy()).unwrap();
    let mut wide = vec![0x10];
    wide.extend([0, 1].repeat(64));
    let mut last_zero = quantizers(0, 1);
    last_zero[64] = 0;
    for bad in [
        quantizers(0, 0),
        last_zero,
        wide,
        quantizers(0x20, 1),
        quantizers(4, 1),
        vec![],
        vec![0],
        vec![0; 64],
        [quantizers(0, 1), vec![1, 1]].concat(),
    ] {
        // Test both a sole malformed definition and a malformed definition
        // following a valid referenced table (in separate and shared segments).
        for tables in [
            vec![bad.clone()],
            vec![quantizers(0, 1), bad.clone()],
            vec![[quantizers(0, 1), bad.clone()].concat()],
        ] {
            // An empty suffix adds no malformed table to a nonempty segment.
            if bad.is_empty() && tables.len() == 1 && !tables[0].is_empty() {
                continue;
            }
            assert_eq!(
                v.validate(quantized_block(&tables, 0).into()).await,
                Err(Error::Invalid)
            );
        }
    }
    // Invalid unused destination-1 table must also be rejected.
    assert_eq!(
        v.validate(quantized_block(&[quantizers(0, 1), quantizers(1, 0)], 0).into())
            .await,
        Err(Error::Invalid)
    );
    v.close_and_drain().await;
}
#[tokio::test]
async fn jpeg_dqt_rejects_missing_and_out_of_range_references() {
    let v = IsolatedValidator::new(limits(), policy()).unwrap();
    for (tables, reference) in [
        (vec![], 0),
        (vec![quantizers(0, 1)], 1),
        (vec![quantizers(0, 1)], 4),
        (vec![quantizers(0, 1)], 255),
    ] {
        assert_eq!(
            v.validate(quantized_block(&tables, reference).into()).await,
            Err(Error::Invalid)
        );
    }
    v.close_and_drain().await;
}
#[tokio::test]
async fn jpeg_malformed_trailing_and_resource_limits_fail_closed() {
    let v = IsolatedValidator::new(limits(), policy()).unwrap();
    let good = jpeg(false);
    let mut trailing = good.clone();
    trailing.push(0);
    assert_eq!(v.validate(trailing.into()).await, Err(Error::TrailingData));
    // Missing EOI, truncated headers, and EOI with most entropy data removed.
    for cut in [2, 50, good.len() - 2] {
        assert!(v.validate(good[..cut].to_vec().into()).await.is_err());
    }
    let sos = good.windows(2).position(|w| w == [0xff, 0xda]).unwrap();
    let scan_start = sos + 2 + u16::from_be_bytes([good[sos + 2], good[sos + 3]]) as usize;
    let mut empty_scan = good[..scan_start].to_vec();
    empty_scan.extend([0xff, 0xd9]);
    assert!(v.validate(empty_scan.into()).await.is_err());
    for which in 0..6 {
        let mut l = limits();
        match which {
            0 => l.max_input_bytes = good.len() - 1,
            1 => l.max_width = 31,
            2 => l.max_pixels = 511,
            3 => l.max_decoded_bytes = 1535,
            4 => l.max_work_units = 1,
            _ => l.max_chunks = 1,
        }
        assert!(
            IsolatedValidator::new(l, policy())
                .unwrap()
                .validate(good.clone().into())
                .await
                .is_err()
        );
    }
    assert!(v.validate(b"not an image".to_vec().into()).await.is_err());
    assert!(v.validate(good.into()).await.is_ok());
}
#[tokio::test]
async fn jpeg_metadata_is_preserved_uninterpreted_and_dimensions_are_encoded() {
    let v = IsolatedValidator::new(limits(), policy()).unwrap();
    let good = jpeg(false);
    // Valid little-endian EXIF orientation 6. Validator never rotates output or
    // reports display-oriented dimensions; input remains byte-for-byte unchanged.
    let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let mut input = good[..2].to_vec();
    input.extend([0xff, 0xe1]);
    input.extend(((exif.len() + 2) as u16).to_be_bytes());
    input.extend(exif);
    input.extend(&good[2..]);
    let info = v.validate(input.into()).await.unwrap();
    assert_eq!((info.width, info.height), (32, 16));
}
fn fixture(script: &str) -> (tempfile::TempDir, ProcessLimits, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let executable = dir.path().join("worker");
    let pid = dir.path().join("pid");
    std::fs::write(
        &executable,
        format!("#!/bin/sh\necho $$ > '{}'\n{}\n", pid.display(), script),
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut p = policy();
    p.executable = executable;
    (dir, p, pid)
}
async fn await_pid(path: &std::path::Path) -> u32 {
    let start = Instant::now();
    loop {
        if let Ok(s) = std::fs::read_to_string(path) {
            if let Ok(pid) = s.trim().parse() {
                return pid;
            }
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        tokio::task::yield_now().await;
    }
}
fn reaped(pid: u32) {
    assert!(
        !PathBuf::from(format!("/proc/{pid}")).exists(),
        "child {pid} remains alive/zombie"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_retains_admission_until_kill_and_reap() {
    let (_dir, p, pidfile) = fixture("while :; do :; done");
    let v = IsolatedValidator::new(limits(), p).unwrap();
    let other = v.clone();
    let task = tokio::spawn(async move { other.validate(png().into()).await });
    let pid = await_pid(&pidfile).await;
    assert_eq!(v.validate(png().into()).await, Err(Error::Busy));
    task.abort();
    let _ = task.await;
    v.close_and_drain().await;
    reaped(pid);
    assert_eq!(v.validate(png().into()).await, Err(Error::Closed));
}
#[tokio::test]
async fn deadline_kills_reaps_and_releases_capacity() {
    let (_dir, mut p, pidfile) = fixture("while :; do :; done");
    p.deadline = Duration::from_millis(150);
    let v = IsolatedValidator::new(limits(), p).unwrap();
    assert_eq!(v.validate(png().into()).await, Err(Error::Deadline));
    let pid = std::fs::read_to_string(&pidfile)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    reaped(pid);
    // Re-admission proves the previous worker's permit was returned.
    assert_eq!(v.validate(png().into()).await, Err(Error::Deadline));
    v.close_and_drain().await;
}
#[tokio::test]
async fn kernel_cpu_and_address_space_limits_are_effective() {
    let (_dir, mut p, pidfile) = fixture("while :; do :; done");
    p.cpu_seconds = 1;
    p.deadline = Duration::from_secs(10);
    let v = IsolatedValidator::new(limits(), p).unwrap();
    assert_eq!(v.validate(png().into()).await, Err(Error::Worker));
    let pid = std::fs::read_to_string(&pidfile)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    reaped(pid);
    let mut p = policy();
    p.address_space_bytes = 1;
    assert_eq!(
        IsolatedValidator::new(limits(), p)
            .unwrap()
            .validate(png().into())
            .await,
        Err(Error::Worker)
    );
}
#[tokio::test]
async fn child_exit_bad_output_and_spawn_failure_release_capacity() {
    for script in ["exit 4", "printf bogus", "printf 12345678901234567890"] {
        let (_dir, p, pidfile) = fixture(script);
        let v = IsolatedValidator::new(limits(), p).unwrap();
        assert_eq!(v.validate(png().into()).await, Err(Error::Worker));
        let pid = std::fs::read_to_string(&pidfile)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        reaped(pid);
        assert_eq!(v.validate(png().into()).await, Err(Error::Worker));
    }
    let mut p = policy();
    p.executable = PathBuf::from("/nonexistent/stucco-image-worker");
    let v = IsolatedValidator::new(limits(), p).unwrap();
    assert_eq!(v.validate(png().into()).await, Err(Error::Worker));
    assert_eq!(v.validate(png().into()).await, Err(Error::Worker));
}

#[test]
fn worker_refuses_missing_or_mismatched_protocol_version() {
    for version in ["stucco-image-worker-v0:0.0.0", "1048576"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_stucco-image-worker"))
            .args([
                version, "1048576", "1024", "1024", "1048576", "4194304", "8388608", "100",
                "33554432",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}
