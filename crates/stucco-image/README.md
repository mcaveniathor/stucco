# stucco-image

Opt-in server-side full decoding for restricted static PNG and baseline JPEG.
Not a MIME sniffer, sanitizer, universal phone-image decoder, or filesystem/network
sandbox. Add this crate explicitly; the facade enables no dependency or route.

## Two APIs

- `Validator`: existing in-process restricted-PNG pool, unchanged. Byte-work and
  codec allocation budgets are **not** hard CPU/RSS limits. Canceling a caller
  does not stop decoding; its permit remains owned until actual completion.
- `isolated::IsolatedValidator`: PNG and baseline JPEG, **Linux x86-64 only**.
  Install the matching `stucco-image-worker` binary and supply its absolute path
  in trusted `ProcessLimits`. Unsupported platforms return `Unsupported`; resource
  setup, seccomp or executable failures return an error. No in-process fallback.

Build worker: `cargo build -p stucco-image --bin stucco-image-worker --release`.
The worker rejects mismatched crate/protocol versions before reading input.
Deploy it alongside the application, in an operator-owned immutable location.
The executable is a trusted matching-version dependency, not a plugin and never
selected from request data. The public hidden `worker_main` is only an executable
entrypoint; calling it inside the host process is not supported (it installs a
process-wide syscall filter). See `examples/isolated.rs` for operator configuration.

## Accepted formats and metadata policy

**PNG:** unchanged: only non-interlaced RGB/RGBA8 with IHDR/contiguous IDAT/IEND.
All chunk CRCs and exact zlib size/end/checksum are checked, then png reconstructs
all pixels and finishes the stream. Palettes, grayscale, 16-bit, interlace,
metadata/ancillary chunks and APNG are rejected. Trailing file/zlib bytes fail.

**JPEG:** only 8-bit baseline Huffman grayscale or three-component JPEG, one scan
containing all components, with normal sampling/restart markers. A strict envelope
and canonical Huffman entropy pass checks the declared MCU count, coefficient
runs, restart order, padding and exact entropy consumption before zune-jpeg does
full RGB pixel reconstruction in strict mode. A header alone never succeeds.
Progressive/multiscan, CMYK, arithmetic/lossless/hierarchical JPEG, MPF multi-picture,
extra entropy, missing/early EOI and trailing bytes fail closed. We explicitly do
not rely on a decoder's strict option to reject early EOI: tested codec behavior
can otherwise synthesize missing pixels. The extra pass does not implement IDCT.

JPEG APP/COM metadata (including EXIF/XMP/ICC and GPS) is **preserved in the
original input**, not stripped, authenticated or returned as trusted fields.
Dimensions refer to the encoded raster; EXIF orientation is **not applied** and
can swap display width/height in clients. Pixel decoding is a validation pass,
not a color-managed or orientation-normalized derivative. Store/serve originals
privately under application authorization, never advertise them as metadata-free.
Creating sanitized normalized variants is separate work. PNG's stricter metadata
policy is unchanged. HEIC/WebP and all other formats remain unsupported; a phone
may need to export a baseline JPEG. This is not universal camera-file support.

`ImageInfo` supplies dimensions and fixed verified MIME, not an unforgeable token.
Downstream code must trust its own validator invocation, not client-supplied fields.

## Bounds and process ownership

All `Limits` fields and `ProcessLimits` values are explicit operator configuration,
with no product quota/retention defaults. Share clones of one pool; per-request
pools defeat admission. `Busy` is immediate, not an unbounded wait queue.

- Input bytes, width/height, pixels, decoded bytes, chunks/markers and byte-work
  estimates are bounded. PNG work: input + twice filtered bytes + decoded bytes.
  JPEG work: input + three times RGB output bytes; this is an estimate, not CPU
  instructions. JPEG marker count excludes entropy restart markers (bounded by
  input/MCU limits). `png_allocation_bytes` is only png's tracked allocation cap.
- Parent retains only the bounded admitted input and a **10-byte** response buffer;
  valid worker protocol is exactly 9 bytes. Pipes are nonblocking and kernel-bounded;
  stderr is discarded. Child receives bounded stdin, checks max+1, and sends only
  status/format/dimensions. Original pixels never travel back through stdout.
  A small `Bytes` slice can retain a larger backing allocation: separately bound
  ingress allocation and body admission. Child read/allocation overhead is covered
  by its OS address-space ceiling, not a claim that Vec capacity equals input len.
- Before exec: Linux `RLIMIT_CPU` hard/soft whole seconds, `RLIMIT_AS` hard/soft bytes,
  and no core dumps. Infinite limits are rejected. Address space is **not RSS**;
  configure enough for loader, stack, input, decoder state and output. Memory
  exhaustion, signals and worker crashes fail closed. CPU/kernel scheduling and
  the 2 ms supervisor poll are not real-time guarantees.
- On worker entry, before input parsing, no-new-privileges and an architecture-
  checked seccomp filter deny fork/vfork/clone/clone3 and further exec. This makes
  the supplied sequential decoder unable to leave descendants. It is **not** a
  general syscall, filesystem or network sandbox; no container claim is made.
- A blocking supervisor owns the process, permit and pipes. Deadline includes
  spawn/transfer/decode/exit after the supervisor starts; Tokio blocking-pool queue
  time precedes it and is not a wall-clock decode budget. Admission bounds queued
  plus running supervisors. Cancel sets an atomic flag, checked during pipe work.
  Deadline/cancellation/error/panic drops a child guard which kills and waits/reaps;
  only then can admission return. Spawn/OS waits are subject to normal kernel
  scheduling; a user-space wall timer cannot interrupt a stalled kernel syscall.
- `close_and_drain()` closes all clones and waits for actual worker reaping; it
  does not cancel live caller-owned validation, which still has its deadline.
  Canceling drain does not reopen admission. Runtime/process forced termination
  is not a successful drain. Stop producers, drain application operations, then
  drain the validator before shutting down the runtime.

## Downstream integration boundary

Grout should hold one `IsolatedValidator` in its trusted `ProofImageValidator`
adapter, call it inside the existing runtime-owned operation, map verified MIME
and dimensions, and retain post-decode session/digest/immutable-attempt checks.
Distinguish Busy for safe retry, fail closed on other errors, and drain on shutdown.
This crate installs no adapter, HTTP route, retention policy, or product feature.
Do not expose uploads before independent review and downstream acceptance.

## Verification

Tests cover full PNG/baseline JPEG pixels; RGB/grayscale subsampling and restart
intervals; every shortened test JPEG entropy stream even with replacement EOI;
metadata/orientation semantics; successful PNG inflation >=8 KiB including an exact
boundary and byte-fragmented IDAT; malformed/limits; actual subprocess success,
nonzero/short/oversized output, OS CPU/address-space failures, deadline and canceled
waiters, closed/drained admission and `/proc` proof of reaping; seccomp denial of
process/thread creation/exec. Fixture executables are test-only and never used by
production decoding. No external photo corpus, fuzz campaign, non-x86 Linux or
whole-workspace/browser verification is claimed here. Rust MSRV is 1.85.
