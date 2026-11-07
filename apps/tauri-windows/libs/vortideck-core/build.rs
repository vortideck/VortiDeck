use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use vortideck_pdk_gen::{extract_host_functions, PdkGenerator, TypeScriptGenerator};

fn assert_generated(path: &Path, expected: &str, stale: &mut Vec<PathBuf>) {
    if fs::read(path).ok().as_deref() != Some(expected.as_bytes()) {
        stale.push(path.to_path_buf());
    }
}

fn main() {
    println!("cargo:rerun-if-changed=src/plugins/manager.rs");
    println!("cargo:rerun-if-changed=../vortideck-pdk-gen/src");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let repository = manifest_dir.ancestors().nth(4).unwrap();
    let manager_path = manifest_dir.join("src/plugins/manager.rs");
    let functions = extract_host_functions(
        &fs::read_to_string(&manager_path).expect("Failed to read plugin manager host functions"),
    );
    let generator = TypeScriptGenerator;
    let pdk = generator.generate(&functions);
    let declarations = generator
        .generate_type_declarations(&functions)
        .expect("TypeScript declarations generator is unavailable");
    let api = generator
        .generate_sdk_wrapper(&functions)
        .expect("TypeScript SDK wrapper generator is unavailable");

    let mut stale = Vec::new();
    let sdk = repository.join("packages/plugin-sdk/src");
    let example = repository.join("examples/plugin-new-pdk/src");
    assert_generated(&sdk.join("pdk.generated.ts"), &pdk, &mut stale);
    assert_generated(&sdk.join("index.d.ts"), &declarations, &mut stale);
    assert_generated(&sdk.join("api.ts"), &api, &mut stale);
    assert_generated(&example.join("pdk.generated.ts"), &pdk, &mut stale);
    assert_generated(&example.join("index.d.ts"), &declarations, &mut stale);

    if !stale.is_empty() {
        let paths = stale
            .iter()
            .map(|path| format!("\n  - {}", path.display()))
            .collect::<String>();
        panic!(
            "Generated plugin SDK artifacts are stale:{paths}\nRun `pnpm generate:plugin-sdk` from the repository root and commit the result."
        );
    }
}
