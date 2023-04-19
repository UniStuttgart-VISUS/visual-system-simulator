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

fn vulkan_dirs() -> Result<(Vec<PathBuf>, Vec<PathBuf>), std::env::VarError> {
    let include_paths = {
        let dir = std::path::PathBuf::from("D:/Coding/Libraries/Vulkan-SDK/Include");
        if dir.is_dir() {
            vec![dir]
        } else {
            Vec::new()
        }
    };
    let link_paths = {
        let dir = std::path::PathBuf::from("D:/Coding/Libraries/Vulkan-SDK/Lib");
        if dir.is_dir() {
            vec![dir]
        } else {
            Vec::new()
        }
    };

    Ok((include_paths, link_paths))
}

fn openxr_dirs() -> Result<(Vec<PathBuf>, Vec<PathBuf>), std::env::VarError> {
    let include_paths = {
        let dir = std::path::PathBuf::from(std::env::var("OpenXR_INCLUDE_DIR")?);
        if dir.is_dir() {
            vec![dir]
        } else {
            Vec::new()
        }
    };
    let link_paths = {
        let dir = std::path::PathBuf::from(std::env::var("OpenXR_LIB_DIR")?);
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
        let target = std::env::var("TARGET").unwrap();
        let (mut include_dirs, mut lib_dirs) = varjo_dirs().unwrap_or_else(|_| {
            eprintln!(
                "Unable to find Varjo SDK. \
                Please set the environment variables \
                VARJO_INCLUDE_DIR and VARJO_LIB_DIR."
            );
            process::exit(1);
        });

        let (mut vk_include_dirs, mut vk_lib_dirs) = vulkan_dirs().unwrap_or_else(|_| {
            eprintln!(
                "Unable to find Vulkan SDK. \
                Please set the environment variable \
                VULKAN_SDK."
            );
            process::exit(1);
        });

        include_dirs.append(&mut vk_include_dirs);
        lib_dirs.append(&mut vk_lib_dirs);

        println!("cargo:rerun-if-changed=src/varjo.cpp");

        let mut build = cc::Build::new();
        build.cpp(true).warnings(true);
        if target.contains("msvc") {
            build.flag("-EHsc");
        }
        build
            .includes(include_dirs)
            .file("src/varjo.cpp")
            .compile("vss-desktop-cc");

        for dir in lib_dirs {
            println!("cargo:rustc-link-search=native={}", dir.to_str().unwrap());
        }
        link("vss-desktop-cc", "static");
        link("VarjoLib", "dylib");
        //TODO only do this on windows
        link("vulkan-1", "static")
    }
    if cfg!(feature = "openxr") {
        let target = std::env::var("TARGET").unwrap();
        let (include_dirs, lib_dirs) = openxr_dirs().unwrap_or_else(|_| {
            eprintln!(
                "Unable to find OpenXR Includes. \
                Please set the environment variable \
                OpenXR_INCLUDE_DIR."
            );
            process::exit(1);
        });

        println!("cargo:rerun-if-changed=src/openxr.cpp");

        let mut build = cc::Build::new();
        build.cpp(true).warnings(true);
        if target.contains("msvc") {
            build.flag("-EHsc");
        }
        build
            .includes(include_dirs)
            .file("src/openxr.cpp")
            .compile("vss-desktop-cc");


        for dir in lib_dirs {
            println!("cargo:rustc-link-search=native={}", dir.to_str().unwrap());
        }

        println!("cargo:rustc-link-lib=pathcch");

        link("vss-desktop-cc", "static");
        link("openxr_loader", "static");
    }
}
