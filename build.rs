use std::convert::TryFrom;
use std::ffi::OsStr;
use std::{env, path::PathBuf};

use anyhow::*;

use embuild::bindgen;
use embuild::build;
use embuild::cargo;
use embuild::kconfig;
use embuild::pio;
use embuild::pio::project;

use walkdir::WalkDir;

fn main() -> Result<()> {
    let pio = pio::Pio::install_default()?;

    let resolution = pio::Resolver::new(pio.clone())
        .params(pio::ResolutionParams {
            platform: Some("espressif32".into()),
            frameworks: vec!["espidf".into()],
            target: Some(env::var("TARGET")?),
            ..Default::default()
        })
        .resolve(true)?;

    let mut builder = project::Builder::new(PathBuf::from(env::var("OUT_DIR")?).join("esp-idf"));

    let project_path = builder.generate(&resolution)?;

    pio.exec_with_args(&[
        OsStr::new("lib"),
        OsStr::new("--storage-dir"),
        OsStr::new(&PathBuf::from(".")),
        OsStr::new("install"),
        OsStr::new("esp-homekit-sdk"),
    ])?;

    pio.build(&project_path, env::var("PROFILE")? == "release")?;

    let pio_scons_vars = project::SconsVariables::from_dump(&project_path)?;

    let header = PathBuf::from("src").join("include").join("bindings.h");

    cargo::track_file(&header);

    let d = "esp-homekit-sdk";
    let mut include = Vec::new();

    for entry in WalkDir::new(d).into_iter().filter_map(|e| e.ok()) {
        if entry.path().ends_with("include") {
            include.push(entry.path().display().to_string());
        }
    }

    bindgen::run(
        bindgen::Factory::from_scons_vars(&pio_scons_vars)?
            .builder()?
            .ctypes_prefix("c_types")
            .header(header.to_string_lossy())
            .blacklist_function("strtold")
            .blacklist_function("_strtold_r")
            .clang_args(include),
    )
}