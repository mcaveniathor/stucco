//! Linux process resource boundary. This is resource isolation, not a filesystem
//! or network sandbox. Install the matching trusted worker binary separately.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::{AnimationPolicy, TrailingDataPolicy};
use crate::{Error, Limits, Validator, WorkPermit};
use bytes::Bytes;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::io::{Read, Write};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: Format,
}
impl ImageInfo {
    pub const fn media_type(&self) -> &'static str {
        match self.format {
            Format::Png => "image/png",
            Format::Jpeg => "image/jpeg",
        }
    }
}
/// Immutable operator policy. Limits apply per process; admission applies across clones.
#[derive(Clone, Debug)]
pub struct ProcessLimits {
    /// Absolute path to the matching, trusted stucco-image-worker binary. Never request data.
    pub executable: PathBuf,
    /// RLIMIT_AS: address space, not resident-set accounting.
    pub address_space_bytes: u64,
    /// RLIMIT_CPU hard/soft whole seconds, including process startup.
    pub cpu_seconds: u64,
    /// Wall time includes spawn, input transfer, decode and exit.
    pub deadline: Duration,
}
#[derive(Clone)]
pub struct IsolatedValidator {
    pool: Validator,
    process: ProcessLimits,
}
struct Cancel(Arc<AtomicBool>);
impl Drop for Cancel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
impl IsolatedValidator {
    pub fn new(limits: Limits, process: ProcessLimits) -> Result<Self, Error> {
        if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            return Err(Error::Unsupported);
        }
        if !process.executable.is_absolute()
            || process.address_space_bytes == 0
            || process.address_space_bytes == u64::MAX
            || process.cpu_seconds == 0
            || process.cpu_seconds == u64::MAX
            || process.deadline.is_zero()
            || Instant::now().checked_add(process.deadline).is_none()
        {
            return Err(Error::Configuration);
        }
        Ok(Self {
            pool: Validator::new(limits)?,
            process,
        })
    }
    /// Immediate admission. Dropping this future cancels; capacity remains held
    /// by the supervisor until the child has been killed and synchronously reaped.
    pub async fn validate(&self, input: Bytes) -> Result<ImageInfo, Error> {
        if input.len() > self.pool.limits.max_input_bytes {
            return Err(Error::Limit);
        }
        let slot = self
            .pool
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|e| match e {
                tokio::sync::TryAcquireError::Closed => Error::Closed,
                _ => Error::Busy,
            })?;
        let guard = WorkPermit {
            slot: Some(slot),
            changed: self.pool.changed.clone(),
        };
        let cancel = Cancel(Arc::new(AtomicBool::new(false)));
        let flag = cancel.0.clone();
        let limits = self.pool.limits;
        let process = self.process.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _guard = guard;
            supervise(input, limits, process, flag)
        })
        .await
        .map_err(|_| Error::Worker)?;
        drop(cancel);
        result
    }
    /// Close admission globally and await actual worker settlement, including reap.
    pub async fn close_and_drain(&self) {
        self.pool.close_and_drain().await;
    }
}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn wire_limits(l: Limits) -> Vec<String> {
    std::iter::once(format!(
        "stucco-image-worker-v1:{}",
        env!("CARGO_PKG_VERSION")
    ))
    .chain(
        [
            l.max_input_bytes as u64,
            u64::from(l.max_width),
            u64::from(l.max_height),
            l.max_pixels,
            l.max_decoded_bytes as u64,
            l.png_allocation_bytes as u64,
            l.max_chunks as u64,
            l.max_work_units,
        ]
        .iter()
        .map(ToString::to_string),
    )
    .collect()
}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn parse_limits() -> Result<Limits, Error> {
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref()
        != Some(concat!(
            "stucco-image-worker-v1:",
            env!("CARGO_PKG_VERSION")
        ))
    {
        return Err(Error::Configuration);
    }
    let args = arguments
        .map(|v| v.parse::<u64>().map_err(|_| Error::Configuration))
        .collect::<Result<Vec<_>, _>>()?;
    if args.len() != 8 {
        return Err(Error::Configuration);
    }
    let l = Limits {
        max_input_bytes: args[0].try_into().map_err(|_| Error::Configuration)?,
        max_width: args[1].try_into().map_err(|_| Error::Configuration)?,
        max_height: args[2].try_into().map_err(|_| Error::Configuration)?,
        max_pixels: args[3],
        max_decoded_bytes: args[4].try_into().map_err(|_| Error::Configuration)?,
        png_allocation_bytes: args[5].try_into().map_err(|_| Error::Configuration)?,
        max_chunks: args[6].try_into().map_err(|_| Error::Configuration)?,
        max_work_units: args[7],
        max_frames: 1,
        max_concurrent: 1,
        animation: AnimationPolicy::Reject,
        trailing_data: TrailingDataPolicy::Reject,
    };
    Validator::new(l)?;
    Ok(l)
}
/// Entrypoint for the matching executable; not an in-process sandbox API.
#[doc(hidden)]
pub fn worker_main() -> Result<(), Error> {
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    return Err(Error::Unsupported);
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        deny_descendants()?;
        let limits = parse_limits()?;
        let mut input = Vec::new();
        std::io::stdin()
            .take((limits.max_input_bytes as u64).saturating_add(1))
            .read_to_end(&mut input)
            .map_err(|_| Error::Worker)?;
        let result = if input.len() > limits.max_input_bytes {
            Err(Error::Limit)
        } else if input.starts_with(&[0xff, 0xd8]) {
            crate::jpeg::decode(&input, limits).map(|(width, height)| ImageInfo {
                width,
                height,
                format: Format::Jpeg,
            })
        } else {
            crate::decode(&input, limits).map(|v| ImageInfo {
                width: v.width,
                height: v.height,
                format: Format::Png,
            })
        };
        let mut out = [0u8; 9];
        match result {
            Ok(v) => {
                out[0] = match v.format {
                    Format::Png => 1,
                    Format::Jpeg => 2,
                };
                out[1..5].copy_from_slice(&v.width.to_be_bytes());
                out[5..9].copy_from_slice(&v.height.to_be_bytes());
            }
            Err(e) => {
                out[0] = match e {
                    Error::Limit => 10,
                    Error::Unsupported => 11,
                    Error::TrailingData => 12,
                    _ => 13,
                }
            }
        }
        std::io::stdout().write_all(&out).map_err(|_| Error::Worker)
    }
}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn deny_descendants() -> Result<(), Error> {
    // Resource ownership requires no child processes or threads. The decoder is
    // sequential (zune-jpeg is built without optional SIMD features). Deny subsequent
    // exec too. This is intentionally not represented as a general seccomp sandbox.
    let mut filter = vec![
        libc::sock_filter {
            code: 0x20,
            jt: 0,
            jf: 0,
            k: 4,
        }, // seccomp_data.arch
        libc::sock_filter {
            code: 0x15,
            jt: 1,
            jf: 0,
            k: 0xc000003e,
        }, // AUDIT_ARCH_X86_64
        libc::sock_filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: libc::SECCOMP_RET_KILL_PROCESS,
        },
        libc::sock_filter {
            code: 0x20,
            jt: 0,
            jf: 0,
            k: 0,
        },
        libc::sock_filter {
            code: 0x35,
            jt: 0,
            jf: 1,
            k: 0x40000000,
        }, // reject x32 ABI
        libc::sock_filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: libc::SECCOMP_RET_KILL_PROCESS,
        },
    ];
    for nr in [
        libc::SYS_clone,
        libc::SYS_clone3,
        libc::SYS_fork,
        libc::SYS_vfork,
        libc::SYS_execve,
        libc::SYS_execveat,
    ] {
        filter.push(libc::sock_filter {
            code: 0x15,
            jt: 0,
            jf: 1,
            k: nr as u32,
        });
        filter.push(libc::sock_filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: libc::SECCOMP_RET_ERRNO | libc::EPERM as u32,
        });
    }
    filter.push(libc::sock_filter {
        code: 0x06,
        jt: 0,
        jf: 0,
        k: libc::SECCOMP_RET_ALLOW,
    });
    let program = libc::sock_fprog {
        len: filter.len() as u16,
        filter: filter.as_mut_ptr(),
    };
    // SAFETY: pointers address live, correctly-sized kernel BPF structs. No new
    // privileges and filter are installed only in the single-threaded worker.
    let ok = unsafe {
        libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) == 0
            && libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &program) == 0
    };
    if ok { Ok(()) } else { Err(Error::Worker) }
}
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn supervise(
    _: Bytes,
    _: Limits,
    _: ProcessLimits,
    _: Arc<AtomicBool>,
) -> Result<ImageInfo, Error> {
    Err(Error::Unsupported)
}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn supervise(
    input: Bytes,
    limits: Limits,
    policy: ProcessLimits,
    cancel: Arc<AtomicBool>,
) -> Result<ImageInfo, Error> {
    use std::{
        os::{fd::AsRawFd, unix::process::CommandExt},
        process::{Child, Command, Stdio},
    };
    struct Reap(Child);
    impl Drop for Reap {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let deadline = Instant::now()
        .checked_add(policy.deadline)
        .ok_or(Error::Configuration)?;
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Worker);
    }
    let mut command = Command::new(&policy.executable);
    command
        .args(wire_limits(limits))
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // SAFETY: pre_exec only invokes async-signal-safe libc resource setters;
    // no allocation, locks or formatting are used in the post-fork child.
    unsafe {
        command.pre_exec(move || {
            for (resource, value) in [
                (libc::RLIMIT_AS, policy.address_space_bytes),
                (libc::RLIMIT_CPU, policy.cpu_seconds),
                (libc::RLIMIT_CORE, 0),
            ] {
                let limit = libc::rlimit {
                    rlim_cur: value,
                    rlim_max: value,
                };
                if libc::setrlimit(resource, &limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    let mut child = Reap(command.spawn().map_err(|_| Error::Worker)?);
    let mut stdin = Some(child.0.stdin.take().ok_or(Error::Worker)?);
    let mut stdout = child.0.stdout.take().ok_or(Error::Worker)?;
    for fd in [stdin.as_ref().unwrap().as_raw_fd(), stdout.as_raw_fd()] {
        // SAFETY: valid owned pipe descriptors; F_GETFL and F_SETFL do not retain pointers.
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            if flags == -1 || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) == -1 {
                return Err(Error::Worker);
            }
        }
    }
    let mut sent = 0;
    let mut output = [0u8; 10];
    let mut received = 0;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Worker);
        }
        if Instant::now() >= deadline {
            return Err(Error::Deadline);
        }
        if let Some(pipe) = stdin.as_mut() {
            match pipe.write(&input[sent..]) {
                Ok(n) => sent += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => {
                    stdin = None;
                }
            }
            if sent == input.len() {
                stdin = None;
            }
        }
        match stdout.read(&mut output[received..]) {
            Ok(n) => received += n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => return Err(Error::Worker),
        }
        if received == output.len() {
            return Err(Error::Worker);
        }
        if let Some(status) = child.0.try_wait().map_err(|_| Error::Worker)? {
            // Final bounded drain after exit (the supplied worker cannot fork).
            loop {
                match stdout.read(&mut output[received..]) {
                    Ok(0) => break,
                    Ok(n) => {
                        received += n;
                        if received == output.len() {
                            return Err(Error::Worker);
                        }
                    }
                    Err(_) => return Err(Error::Worker),
                }
            }
            if !status.success() || received != 9 || sent != input.len() {
                return Err(Error::Worker);
            }
            return match output[0] {
                1 | 2 => {
                    let width = u32::from_be_bytes(output[1..5].try_into().unwrap());
                    let height = u32::from_be_bytes(output[5..9].try_into().unwrap());
                    if width == 0
                        || height == 0
                        || width > limits.max_width
                        || height > limits.max_height
                        || u64::from(width) * u64::from(height) > limits.max_pixels
                    {
                        return Err(Error::Worker);
                    }
                    Ok(ImageInfo {
                        width,
                        height,
                        format: if output[0] == 1 {
                            Format::Png
                        } else {
                            Format::Jpeg
                        },
                    })
                }
                10 => Err(Error::Limit),
                11 => Err(Error::Unsupported),
                12 => Err(Error::TrailingData),
                13 => Err(Error::Invalid),
                _ => Err(Error::Worker),
            };
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests {
    #[test]
    fn descendant_filter_denies_fork_clone_and_exec() {
        const FLAG: &str = "STUCCO_IMAGE_FILTER_TEST_CHILD";
        if std::env::var_os(FLAG).is_some() {
            super::deny_descendants().unwrap();
            for syscall in [
                libc::SYS_fork,
                libc::SYS_vfork,
                libc::SYS_clone,
                libc::SYS_clone3,
                libc::SYS_execve,
                libc::SYS_execveat,
            ] {
                // SAFETY: the installed filter denies these syscalls before any
                // arguments are inspected; zero arguments cannot create a child.
                assert_eq!(unsafe { libc::syscall(syscall, 0, 0, 0, 0, 0, 0) }, -1);
                assert_eq!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(libc::EPERM)
                );
            }
        } else {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "isolated::tests::descendant_filter_denies_fork_clone_and_exec",
                ])
                .env(FLAG, "1")
                .status()
                .unwrap();
            assert!(status.success());
        }
    }
}
