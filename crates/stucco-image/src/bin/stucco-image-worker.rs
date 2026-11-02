fn main() {
    if stucco_image::isolated::worker_main().is_err() {
        std::process::exit(1);
    }
}
