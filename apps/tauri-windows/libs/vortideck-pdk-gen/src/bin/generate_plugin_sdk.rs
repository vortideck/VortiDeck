use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use vortideck_pdk_gen::{extract_host_functions, PdkGenerator, TypeScriptGenerator};

struct Artifact {
    path: PathBuf,
    content: String,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("generator crate must remain inside the VortiDeck repository")
        .to_path_buf()
}

fn write_if_changed(path: &Path, content: &str) -> Result<bool, String> {
    if fs::read(path).ok().as_deref() == Some(content.as_bytes()) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, content).map_err(|error| error.to_string())?;
    Ok(true)
}

fn main() -> Result<(), String> {
    let check = env::args().skip(1).any(|argument| argument == "--check");
    let repository = repository_root();
    let manager = repository.join("apps/tauri-windows/libs/vortideck-core/src/plugins/manager.rs");
    let source = fs::read_to_string(&manager).map_err(|error| error.to_string())?;
    let functions = extract_host_functions(&source);
    if functions.is_empty() {
        return Err("No #[vortideck_host] functions found".to_string());
    }

    let generator = TypeScriptGenerator;
    let pdk = generator.generate(&functions);
    let declarations = generator
        .generate_type_declarations(&functions)
        .ok_or("TypeScript declarations generator is unavailable")?;
    let api = generator
        .generate_sdk_wrapper(&functions)
        .ok_or("TypeScript SDK wrapper generator is unavailable")?;
    let sdk = repository.join("packages/plugin-sdk/src");
    let example = repository.join("examples/plugin-new-pdk/src");
    let artifacts = [
        Artifact {
            path: sdk.join("pdk.generated.ts"),
            content: pdk.clone(),
        },
        Artifact {
            path: sdk.join("index.d.ts"),
            content: declarations.clone(),
        },
        Artifact {
            path: sdk.join("api.ts"),
            content: api,
        },
        Artifact {
            path: example.join("pdk.generated.ts"),
            content: pdk,
        },
        Artifact {
            path: example.join("index.d.ts"),
            content: declarations,
        },
    ];

    let stale = artifacts
        .iter()
        .filter(|artifact| {
            fs::read(&artifact.path).ok().as_deref() != Some(artifact.content.as_bytes())
        })
        .collect::<Vec<_>>();
    if check && !stale.is_empty() {
        for artifact in stale {
            eprintln!("stale: {}", artifact.path.display());
        }
        return Err("Generated plugin SDK artifacts are stale".to_string());
    }
    if check {
        println!(
            "Plugin SDK generated artifacts are current ({} host functions).",
            functions.len()
        );
        return Ok(());
    }

    for artifact in artifacts {
        let changed = write_if_changed(&artifact.path, &artifact.content)?;
        println!(
            "{} {}",
            if changed { "updated" } else { "unchanged" },
            artifact.path.display()
        );
    }
    Ok(())
}
