//! Values below are example operator budgets, not product policy defaults.
use std::{path::PathBuf, time::Duration};
use stucco_image::{
    AnimationPolicy, Limits, TrailingDataPolicy,
    isolated::{IsolatedValidator, ProcessLimits},
};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let worker = PathBuf::from(args.next().ok_or("expected absolute worker path")?);
    let file = args.next().ok_or("expected image path")?;
    let limits = Limits {
        max_input_bytes: 8 << 20,
        max_width: 4096,
        max_height: 4096,
        max_pixels: 16 << 20,
        max_decoded_bytes: 64 << 20,
        png_allocation_bytes: 64 << 20,
        max_chunks: 1024,
        max_frames: 1,
        max_work_units: 256 << 20,
        max_concurrent: 2,
        animation: AnimationPolicy::Reject,
        trailing_data: TrailingDataPolicy::Reject,
    };
    let validator = IsolatedValidator::new(
        limits,
        ProcessLimits {
            executable: worker,
            address_space_bytes: 512 << 20,
            cpu_seconds: 5,
            deadline: Duration::from_secs(10),
        },
    )?;
    // This CLI reads an operator-selected local file; a server must also admit
    // and bound network/body allocation before calling the validator.
    use std::io::Read;
    let mut input = Vec::new();
    std::fs::File::open(file)?
        .take(limits.max_input_bytes as u64 + 1)
        .read_to_end(&mut input)?;
    let result = validator.validate(input.into()).await;
    validator.close_and_drain().await;
    println!("{:?}", result?);
    Ok(())
}
