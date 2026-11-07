#![cfg(feature = "extism")]

use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use vortideck_core::core::{
    NoopHttpClient, NoopStorageProvider, NoopSystemInterface, PlatformContext,
};
use vortideck_core::plugins::PluginManager;

fn built_plugin() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("core crate must remain inside the VortiDeck repository")
        .join("examples/plugin-sdk-contract/dist/vortideck-plugin-sdk-contract.wasm")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn documented_sdk_plugin_loads_and_invokes_a_real_contribution() {
    let path = built_plugin();
    let wasm = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{} ({error}); build it with `pnpm --filter vortideck-plugin-sdk-contract build`",
            path.display()
        )
    });
    let platform = Arc::new(PlatformContext::new(
        Arc::new(NoopStorageProvider),
        Arc::new(NoopHttpClient),
        Arc::new(NoopSystemInterface),
        None,
    ));
    let manager = PluginManager::new_with_context(platform);
    let plugin_id = manager
        .load_plugin_from_bytes(&wasm, None, Some(wasm.len() as u64), None)
        .await
        .expect("SDK plugin should load through the production Extism path");

    let result = manager
        .execute_plugin(
            &plugin_id,
            "execute",
            json!({ "__node_id": "uppercase", "input": "VortiDeck" }),
        )
        .await
        .expect("documented uppercase contribution should execute");

    assert_eq!(result["success"], true);
    assert_eq!(result["output"], "VORTIDECK");
}
