fn main() {
    let mut opts = built::Options::default();
    opts.set_git(true);

    let src = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dst = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("built.rs");
    std::fs::create_dir_all("dashboard-ui/build").unwrap();
    built::write_built_file_with_opts(&opts, src.as_ref(), &dst)
        .expect("Failed to acquire build-time information");
}
