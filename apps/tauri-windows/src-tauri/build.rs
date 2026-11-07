use std::path::PathBuf;

fn main() {
    embed_obs_companion();
    tauri_build::build()
}

/// The OBS companion DLL is built in its own repository and staged by
/// scripts/stage-obs-companion.ps1. Source checkouts without it still compile:
/// they embed an empty payload, and installing the companion reports that it
/// is not included in this build.
fn embed_obs_companion() {
    let staged =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/plugins/vorti-obs-plugin.dll");
    println!("cargo:rerun-if-changed={}", staged.display());
    let payload = if staged.exists() {
        staged
    } else {
        println!("cargo:warning=OBS companion DLL not staged; this build cannot install the OBS companion. Run scripts/stage-obs-companion.ps1 to include it.");
        let placeholder =
            PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("vorti-obs-plugin.dll");
        std::fs::write(&placeholder, []).expect("write OBS companion placeholder");
        placeholder
    };
    println!(
        "cargo:rustc-env=VORTIDECK_OBS_COMPANION_DLL={}",
        payload.display()
    );
}
