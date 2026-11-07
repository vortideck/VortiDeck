use std::sync::Arc;
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use vortideck_core::services::overlay_authoring::{
    CompositionContent, CompositionDocument, CompositionEdit, OverlayAuthoringService,
};

fn trusted(window: &WebviewWindow) -> Result<(), String> {
    super::overlay_commands::ensure_trusted_overlay_window(window.label())
}

/// A selected small image is embedded in the editable composition, so another
/// profile/OBS browser never needs the creator's local filesystem path.
#[tauri::command]
pub async fn pick_overlay_image(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<Option<String>, String> {
    trusted(&window)?;
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("Image", &["png", "jpg", "jpeg", "gif", "webp"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|_| "Choose a local image")?;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| "Could not open image")?
            .take(1_048_577)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read image")?;
        encode_overlay_image(&bytes).map(Some)
    })
    .await
    .map_err(|_| "Image selection interrupted".to_string())?
}

fn encode_overlay_image(bytes: &[u8]) -> Result<String, String> {
    use base64::Engine;
    if bytes.is_empty() || bytes.len() > 1_048_576 {
        return Err(
            "Embedded images must be at most 1 MiB; resize the image or use an HTTPS source".into(),
        );
    }
    let format = image::guess_format(bytes).map_err(|_| "Unsupported or corrupt image")?;
    let mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Gif => "image/gif",
        image::ImageFormat::WebP => "image/webp",
        _ => return Err("Choose a PNG, JPEG, GIF or WebP image".into()),
    };
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(67_108_864);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|_| "Image cannot be decoded within the size limits")?;
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[cfg(test)]
mod image_tests {
    use super::encode_overlay_image;
    #[test]
    fn embeds_verified_image_bytes_without_exposing_a_local_path() {
        let image = image::DynamicImage::new_rgba8(2, 2);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        assert!(encode_overlay_image(bytes.get_ref())
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert!(encode_overlay_image(b"<svg onload='bad()'></svg>").is_err());
        assert!(encode_overlay_image(&vec![0; 1_048_577]).is_err());
    }
}

pub async fn observed_values_message(app: &tauri::AppHandle) -> Result<serde_json::Value, String> {
    use tauri::Manager;
    let state = app
        .try_state::<vortideck_core::services::flow::commands::FlowExecutorState>()
        .ok_or("Stream value owner unavailable")?;
    let executor = state
        .executor
        .lock()
        .await
        .clone()
        .ok_or("Stream value owner is not ready")?;
    let values = executor.observed_stream_values().await?;
    Ok(serde_json::json!({"category":"streamValues","payload":values}))
}

#[tauri::command]
pub async fn delete_overlay_composition(
    id: String,
    expected_revision: u64,
    window: WebviewWindow,
    service: State<'_, Arc<OverlayAuthoringService>>,
) -> Result<(), String> {
    trusted(&window)?;
    service.delete(&id, expected_revision).await
}

#[tauri::command]
pub async fn approve_imported_overlay_content(
    id: String,
    expected_revision: u64,
    window: WebviewWindow,
    service: State<'_, Arc<OverlayAuthoringService>>,
) -> Result<CompositionDocument, String> {
    trusted(&window)?;
    service
        .approve_imported_content(&id, expected_revision)
        .await
}

#[tauri::command]
pub async fn list_overlay_compositions(
    window: WebviewWindow,
    service: State<'_, Arc<OverlayAuthoringService>>,
) -> Result<Vec<CompositionDocument>, String> {
    trusted(&window)?;
    Ok(service.list().await)
}

#[tauri::command]
pub async fn read_overlay_composition(
    id: String,
    window: WebviewWindow,
    service: State<'_, Arc<OverlayAuthoringService>>,
) -> Result<CompositionDocument, String> {
    trusted(&window)?;
    service.read(&id).await
}

#[tauri::command]
pub async fn create_overlay_composition(
    id: String,
    content: CompositionContent,
    window: WebviewWindow,
    service: State<'_, Arc<OverlayAuthoringService>>,
) -> Result<CompositionDocument, String> {
    trusted(&window)?;
    let document = service.create(id, content).await?;
    Ok(document)
}

#[tauri::command]
pub async fn edit_overlay_composition(
    id: String,
    expected_revision: u64,
    edit: CompositionEdit,
    window: WebviewWindow,
    service: State<'_, Arc<OverlayAuthoringService>>,
) -> Result<CompositionDocument, String> {
    trusted(&window)?;
    let document = service.edit(&id, expected_revision, edit).await?;
    Ok(document)
}

#[tauri::command]
pub async fn validate_overlay_composition(
    content: CompositionContent,
    window: WebviewWindow,
) -> Result<Vec<serde_json::Value>, String> {
    trusted(&window)?;
    content.validate()?;
    Ok(content.diagnostics())
}

#[tauri::command]
pub async fn validate_draft_widget_settings(
    package_id: Option<String>,
    contribution_id: Option<String>,
    config: serde_json::Value,
    window: WebviewWindow,
    core: State<'_, vortideck_core::VortiDeckCore>,
) -> Result<serde_json::Value, String> {
    trusted(&window)?;
    let schema = crate::overlay::settings::schema_for(
        core.inner(),
        package_id.as_deref(),
        contribution_id.as_deref(),
    )
    .await?;
    vortideck_core::plugins::widget_settings::validate_widget_config(schema.as_ref(), &config)
}

#[tauri::command]
pub async fn export_overlay_setup(
    app: tauri::AppHandle,
    window: WebviewWindow,
    service: State<'_, Arc<OverlayAuthoringService>>,
    core: State<'_, vortideck_core::VortiDeckCore>,
    db: State<'_, vortideck_core::db::db::DbPool>,
) -> Result<Option<String>, String> {
    trusted(&window)?;
    use vortideck_core::services::overlay_portability::{PackageDependency, PortableSetup};
    let documents = service.list().await;
    let packages = super::widget_commands::list_packages_impl(core.inner(), db.inner()).await?;
    let ids: std::collections::BTreeSet<_> = documents
        .iter()
        .flat_map(|document| &document.content.elements)
        .filter_map(|element| {
            element
                .get("widgetId")
                .or_else(|| element.get("packageId"))
                .and_then(serde_json::Value::as_str)
        })
        .map(|id| vortideck_core::plugins::canonical_package_id(id).to_string())
        .collect();
    let dependencies = ids
        .into_iter()
        .map(|id| {
            let installed = packages.iter().find(|package| package.id == id);
            let dependency = PackageDependency {
                version: installed
                    .map(|package| package.version.clone())
                    .unwrap_or_else(|| "unknown".into()),
                consent_required: installed.is_none_or(|package| package.trust != "builtin"),
            };
            (id, dependency)
        })
        .collect();
    let setup = PortableSetup::export(&documents, dependencies)?;
    let bytes = serde_json::to_vec_pretty(&setup).map_err(|_| "Could not encode portable setup")?;
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        let Some(file) = app
            .dialog()
            .file()
            .set_file_name("VortiDeck-overlay-setup.json")
            .add_filter("VortiDeck setup", &["json"])
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|_| "Choose a local setup file")?;
        let parent = path.parent().ok_or("Setup file directory missing")?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| "Could not create setup file")?;
        temporary
            .write_all(&bytes)
            .and_then(|_| temporary.flush())
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|_| "Could not write setup file")?;
        temporary
            .persist(&path)
            .map_err(|_| "Could not save setup file")?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|_| "Setup export interrupted".to_string())?
}

#[tauri::command]
pub async fn import_overlay_setup(
    app: tauri::AppHandle,
    window: WebviewWindow,
    service: State<'_, Arc<OverlayAuthoringService>>,
    core: State<'_, vortideck_core::VortiDeckCore>,
    db: State<'_, vortideck_core::db::db::DbPool>,
) -> Result<Option<serde_json::Value>, String> {
    trusted(&window)?;
    use vortideck_core::services::overlay_portability::{PortableSetup, MAX_SETUP_BYTES};
    let bytes = tokio::task::spawn_blocking(move || -> Result<Option<Vec<u8>>, String> {
        use std::io::Read;
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("VortiDeck setup", &["json"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|_| "Choose a local setup file")?;
        let file = std::fs::File::open(path).map_err(|_| "Could not open setup file")?;
        let mut bytes = Vec::new();
        file.take(MAX_SETUP_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read setup file")?;
        Ok(Some(bytes))
    })
    .await
    .map_err(|_| "Setup import interrupted".to_string())??;
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let setup = PortableSetup::decode(&bytes)?;
    let mut diagnostics = setup.validate()?;
    let packages = super::widget_commands::list_packages_impl(core.inner(), db.inner()).await?;
    for (id, dependency) in &setup.packages {
        match packages.iter().find(|package| &package.id == id) {
            None => diagnostics.push(serde_json::json!({"code":"missing_package","message":format!("Install package '{id}' {} before publishing", dependency.version)})),
            Some(package) if !package.is_granted || package.needs_reconsent => diagnostics.push(serde_json::json!({"code":"package_consent","message":format!("Review package '{id}' access before publishing")})),
            Some(package) if package.version != dependency.version => diagnostics.push(serde_json::json!({"code":"package_version","message":format!("Package '{id}' differs from exported version {}; validate its settings and appearance", dependency.version)})),
            _ => {},
        }
    }
    let documents = service.import_setup(setup).await?;
    Ok(Some(
        serde_json::json!({"documents":documents,"diagnostics":diagnostics}),
    ))
}
