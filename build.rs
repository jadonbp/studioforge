use librojo::cli;

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::PathBuf::from(&out_dir).join("MCPStudioPlugin.rbxm");
    eprintln!("Rebuilding plugin: {dest_path:?}");
    let options = cli::Options {
        global: cli::GlobalOptions {
            verbosity: 1,
            color: cli::ColorChoice::Always,
        },
        subcommand: cli::Subcommand::Build(cli::BuildCommand {
            project: std::path::PathBuf::from("plugin"),
            output: Some(dest_path),
            plugin: None,
            watch: false,
        }),
    };
    options.run().unwrap();

    // Watch all plugin source files individually so Cargo detects new/changed files
    println!("cargo:rerun-if-changed=plugin/default.project.json");
    for entry in walkdir("plugin/src") {
        println!("cargo:rerun-if-changed={}", entry.display());
    }
}

/// Recursively collect all file paths under a directory.
fn walkdir(dir: &str) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path.to_string_lossy()));
            } else {
                files.push(path);
            }
        }
    }
    files
}
