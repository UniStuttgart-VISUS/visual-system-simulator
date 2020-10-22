use std::path::PathBuf;
use std::process;

fn varjo_dirs() -> Result<(Vec<PathBuf>, Vec<PathBuf>), std::env::VarError> {
    let include_paths = {
        let dir = std::path::PathBuf::from(std::env::var("VARJO_INCLUDE_DIR")?);
        if dir.is_dir() {
            vec![dir]
        } else {
            Vec::new()
        }
    };
    let link_paths = {
        let dir = std::path::PathBuf::from(std::env::var("VARJO_LIB_DIR")?);
        if dir.is_dir() {
            vec![dir]
        } else {
            Vec::new()
        }
    };

    Ok((include_paths, link_paths))
}

fn link(lib: &str, mode: &str) {
    println!("cargo:rustc-link-lib={}={}", mode, lib);
}

fn main() {
    if cfg!(feature = "varjo") {
        let (include_dirs, lib_dirs) = varjo_dirs().unwrap_or_else(|_| {
            eprintln!(
                "Unable to find Varjo SDK. \
                Please set the environment variables \
                VARJO_INCLUDE_DIR and VARJO_LIB_DIR."
            );
            process::exit(1);
        });

        println!("cargo:rerun-if-changed=src/varjo.cpp");

        cc::Build::new()
            .cpp(true)
            .warnings(true)
            .includes(include_dirs)
            .file("src/varjo.cpp")
            .compile("vss-desktop-cc");

        for dir in lib_dirs {
            println!("cargo:rustc-link-search=native={}", dir.to_str().unwrap());
        }
        link("vss-desktop-cc", "static");
        link("VarjoLib", "dylib")
    }
}
