//! Durable composition drafts. Renderers and transports do not own authoring history.
//! The open element fields preserve existing creator extensions; publication still
//! validates package consent and widget settings in the existing overlay host.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    io::Write,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::{Mutex, OwnedMutexGuard};

const MAX_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;
const MAX_HISTORY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompositionContent {
    pub name: String,
    pub elements: Vec<Value>,
    pub settings: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompositionDocument {
    pub schema_version: u16,
    pub id: String,
    pub revision: u64,
    pub created: String,
    pub last_modified: String,
    pub content: CompositionContent,
    pub published_revision: Option<u64>,
    pub can_undo: bool,
    pub can_redo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum CompositionEdit {
    Replace { content: CompositionContent },
    UpsertElements { elements: Vec<Value> },
    RemoveElements { ids: Vec<String> },
    Undo,
    Redo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredDraft {
    document: CompositionDocument,
    history: Vec<CompositionContent>,
    cursor: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Library {
    schema_version: u16,
    drafts: BTreeMap<String, StoredDraft>,
}

#[derive(Clone)]
pub struct OverlayAuthoringService {
    path: PathBuf,
    library: Arc<Mutex<Library>>,
    changes: tokio::sync::broadcast::Sender<CompositionDocument>,
}

pub struct PublicationGuard {
    library: OwnedMutexGuard<Library>,
    service: OverlayAuthoringService,
    id: String,
}

/// Stored with the active output before acknowledging publication in the draft
/// library. Recovery repairs metadata only; it never renders or publishes again.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationReceipt {
    pub id: String,
    pub expected_revision: u64,
    pub content: CompositionContent,
}

impl CompositionContent {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() || self.name.len() > 256 {
            return Err("Composition name must contain 1–256 bytes".into());
        }
        if self.elements.len() > 512 {
            return Err("Composition exceeds 512 elements".into());
        }
        if serde_json::to_vec(self)
            .map_err(|_| "Invalid composition")?
            .len()
            > MAX_DOCUMENT_BYTES
        {
            return Err("Composition exceeds 2 MiB".into());
        }
        for key in ["canvasWidth", "canvasHeight"] {
            let size = self
                .settings
                .get(key)
                .and_then(Value::as_f64)
                .ok_or_else(|| format!("Missing {key}"))?;
            if !(1.0..=16384.0).contains(&size) || size.fract() != 0.0 {
                return Err(format!("Invalid {key}"));
            }
        }
        let mut ids = HashSet::new();
        for element in &self.elements {
            let id = element
                .get("id")
                .and_then(Value::as_str)
                .ok_or("Element needs a stable ID")?;
            if id.is_empty()
                || id.len() > 128
                || !id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
                || !ids.insert(id)
            {
                return Err("Invalid or duplicate element ID".into());
            }
            for key in ["x", "y", "width", "height"] {
                let number = element
                    .get(key)
                    .and_then(Value::as_f64)
                    .ok_or_else(|| format!("Element '{id}' needs {key}"))?;
                let valid = if key == "width" || key == "height" {
                    number > 0.0 && number <= 65536.0
                } else {
                    number.abs() <= 1_000_000.0
                };
                if !number.is_finite() || !valid {
                    return Err(format!("Element '{id}' has invalid {key}"));
                }
            }
        }
        Ok(())
    }

    pub fn diagnostics(&self) -> Vec<Value> {
        let width = self.settings["canvasWidth"].as_f64().unwrap_or(0.0);
        let height = self.settings["canvasHeight"].as_f64().unwrap_or(0.0);
        self.elements.iter().filter_map(|element| {
            let x = element["x"].as_f64()?; let y = element["y"].as_f64()?;
            let w = element["width"].as_f64()?; let h = element["height"].as_f64()?;
            if element.get("visible").and_then(Value::as_bool) == Some(false) { return None; }
            (x < 0.0 || y < 0.0 || x + w > width || y + h > height).then(|| serde_json::json!({
                "code": if x + w <= 0.0 || y + h <= 0.0 || x >= width || y >= height { "outside_canvas" } else { "clipped_element" },
                "elementId": element["id"], "message": "Visible element extends outside the saved canvas"
            }))
        }).collect()
    }
}

impl OverlayAuthoringService {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let library = if path.exists() {
            let bytes = std::fs::read(&path).map_err(|_| "Could not read composition library")?;
            let library: Library = serde_json::from_slice(&bytes)
                .map_err(|_| "Invalid composition library; original file retained")?;
            if library.schema_version != 1 {
                return Err("Unsupported composition library version".into());
            }
            for (id, draft) in &library.drafts {
                if id != &draft.document.id
                    || draft.document.schema_version != 1
                    || draft.document.revision == 0
                    || draft.cursor >= draft.history.len()
                    || draft.history.len() > 100
                    || draft.history[draft.cursor] != draft.document.content
                    || draft.document.can_undo != (draft.cursor > 0)
                    || draft.document.can_redo != (draft.cursor + 1 < draft.history.len())
                    || draft
                        .document
                        .published_revision
                        .is_some_and(|revision| revision > draft.document.revision)
                    || serde_json::to_vec(&draft.history)
                        .map_err(|_| "Invalid saved history")?
                        .len()
                        > MAX_HISTORY_BYTES
                {
                    return Err("Invalid saved composition history".into());
                }
                for content in &draft.history {
                    content.validate()?;
                }
            }
            library
        } else {
            Library {
                schema_version: 1,
                drafts: BTreeMap::new(),
            }
        };
        let (changes, _) = tokio::sync::broadcast::channel(64);
        Ok(Self {
            path,
            library: Arc::new(Mutex::new(library)),
            changes,
        })
    }

    fn persist_at(path: &std::path::Path, library: &Library) -> Result<(), String> {
        let parent = path
            .parent()
            .ok_or("Composition library directory missing")?;
        std::fs::create_dir_all(parent)
            .map_err(|_| "Could not create composition library directory")?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| "Could not create composition transaction")?;
        let bytes =
            serde_json::to_vec(library).map_err(|_| "Could not encode composition library")?;
        temporary
            .write_all(&bytes)
            .and_then(|_| temporary.flush())
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|_| "Could not write composition transaction")?;
        temporary
            .persist(path)
            .map_err(|_| "Could not commit composition transaction")?;
        Ok(())
    }

    async fn persist(&self, library: &Library) -> Result<(), String> {
        let path = self.path.clone();
        let library = library.clone();
        tokio::task::spawn_blocking(move || Self::persist_at(&path, &library))
            .await
            .map_err(|_| "Composition transaction interrupted".to_string())?
    }

    pub async fn list(&self) -> Vec<CompositionDocument> {
        self.library
            .lock()
            .await
            .drafts
            .values()
            .map(|draft| draft.document.clone())
            .collect()
    }

    pub fn subscribe_changes(&self) -> tokio::sync::broadcast::Receiver<CompositionDocument> {
        self.changes.subscribe()
    }

    pub async fn read(&self, id: &str) -> Result<CompositionDocument, String> {
        self.library
            .lock()
            .await
            .drafts
            .get(id)
            .map(|draft| draft.document.clone())
            .ok_or("Composition not found".into())
    }

    pub async fn delete(&self, id: &str, expected_revision: u64) -> Result<(), String> {
        let service = self.clone();
        let id = id.to_string();
        tokio::spawn(async move {
            let mut library = service.library.lock().await;
            let draft = library.drafts.get(&id).ok_or("Composition not found")?;
            if draft.document.revision != expected_revision {
                return Err("Stale composition revision; reload before deleting".into());
            }
            let mut candidate = library.clone();
            candidate.drafts.remove(&id);
            service.persist(&candidate).await?;
            *library = candidate;
            Ok(())
        })
        .await
        .map_err(|_| "Composition deletion interrupted".to_string())?
    }

    pub async fn create(
        &self,
        id: String,
        content: CompositionContent,
    ) -> Result<CompositionDocument, String> {
        let service = self.clone();
        tokio::spawn(async move { service.create_owned(id, content).await })
            .await
            .map_err(|_| "Composition creation interrupted".to_string())?
    }

    /// Import as new identities in one durable transaction, never overwrite a
    /// destination project or publish imported content.
    pub async fn import_setup(
        &self,
        setup: super::overlay_portability::PortableSetup,
    ) -> Result<Vec<CompositionDocument>, String> {
        setup.validate()?;
        let service = self.clone();
        tokio::spawn(async move {
            let mut library = service.library.lock().await;
            let mut candidate = library.clone();
            let mut documents = Vec::new();
            for content in setup.compositions {
                let id = uuid::Uuid::new_v4().to_string();
                let now = chrono::Utc::now().to_rfc3339();
                let document = CompositionDocument {
                    schema_version: 1,
                    id: id.clone(),
                    revision: 1,
                    created: now.clone(),
                    last_modified: now,
                    content: content.clone(),
                    published_revision: None,
                    can_undo: false,
                    can_redo: false,
                };
                candidate.drafts.insert(
                    id,
                    StoredDraft {
                        document: document.clone(),
                        history: vec![content],
                        cursor: 0,
                    },
                );
                documents.push(document);
            }
            service.persist(&candidate).await?;
            *library = candidate;
            for document in &documents {
                let _ = service.changes.send(document.clone());
            }
            Ok(documents)
        })
        .await
        .map_err(|_| "Composition import interrupted".to_string())?
    }

    async fn create_owned(
        &self,
        id: String,
        content: CompositionContent,
    ) -> Result<CompositionDocument, String> {
        if id.is_empty()
            || id.len() > 128
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err("Invalid composition ID".into());
        }
        content.validate()?;
        let mut library = self.library.lock().await;
        if library.drafts.contains_key(&id) {
            return Err("Composition already exists; read its revision before editing".into());
        }
        let now = chrono::Utc::now().to_rfc3339();
        let document = CompositionDocument {
            schema_version: 1,
            id: id.clone(),
            revision: 1,
            created: now.clone(),
            last_modified: now,
            content: content.clone(),
            published_revision: None,
            can_undo: false,
            can_redo: false,
        };
        let mut candidate = library.clone();
        candidate.drafts.insert(
            id,
            StoredDraft {
                document: document.clone(),
                history: vec![content],
                cursor: 0,
            },
        );
        self.persist(&candidate).await?;
        *library = candidate;
        let _ = self.changes.send(document.clone());
        Ok(document)
    }

    pub async fn edit(
        &self,
        id: &str,
        expected_revision: u64,
        edit: CompositionEdit,
    ) -> Result<CompositionDocument, String> {
        let service = self.clone();
        let id = id.to_string();
        tokio::spawn(async move {
            service
                .edit_owned(&id, expected_revision, edit, false)
                .await
        })
        .await
        .map_err(|_| "Composition edit interrupted".to_string())?
    }

    pub async fn approve_imported_content(
        &self,
        id: &str,
        expected_revision: u64,
    ) -> Result<CompositionDocument, String> {
        let service = self.clone();
        let id = id.to_string();
        tokio::spawn(async move {
            let document = service.read(&id).await?;
            let mut content = document.content;
            for element in &mut content.elements {
                if element.get("pendingImportReview").and_then(Value::as_bool) == Some(true) {
                    element["pendingImportReview"] = Value::Bool(false);
                }
            }
            service
                .edit_owned(
                    &id,
                    expected_revision,
                    CompositionEdit::Replace { content },
                    true,
                )
                .await
        })
        .await
        .map_err(|_| "Imported content review interrupted".to_string())?
    }

    async fn edit_owned(
        &self,
        id: &str,
        expected_revision: u64,
        edit: CompositionEdit,
        allow_review: bool,
    ) -> Result<CompositionDocument, String> {
        let mut library = self.library.lock().await;
        let mut candidate = library.clone();
        let draft = candidate
            .drafts
            .get_mut(id)
            .ok_or("Composition not found")?;
        if draft.document.revision != expected_revision {
            return Err("Stale composition revision; reload before editing".into());
        }
        let mut content = draft.document.content.clone();
        let history_move = matches!(&edit, CompositionEdit::Undo | CompositionEdit::Redo);
        match edit {
            CompositionEdit::Replace {
                content: replacement,
            } => content = replacement,
            CompositionEdit::UpsertElements { elements } => {
                for element in elements {
                    let id = element
                        .get("id")
                        .and_then(Value::as_str)
                        .ok_or("Element needs a stable ID")?;
                    if let Some(index) = content
                        .elements
                        .iter()
                        .position(|existing| existing.get("id").and_then(Value::as_str) == Some(id))
                    {
                        content.elements[index] = element;
                    } else {
                        content.elements.push(element);
                    }
                }
            }
            CompositionEdit::RemoveElements { ids } => content.elements.retain(|element| {
                !ids.iter()
                    .any(|id| element.get("id").and_then(Value::as_str) == Some(id))
            }),
            CompositionEdit::Undo => {
                if draft.cursor == 0 {
                    return Err("Nothing to undo".into());
                }
                draft.cursor -= 1;
                content = draft.history[draft.cursor].clone();
            }
            CompositionEdit::Redo => {
                if draft.cursor + 1 >= draft.history.len() {
                    return Err("Nothing to redo".into());
                }
                draft.cursor += 1;
                content = draft.history[draft.cursor].clone();
            }
        }
        content.validate()?;
        if !allow_review && !history_move {
            for previous in &draft.document.content.elements {
                if previous.get("pendingImportReview").and_then(Value::as_bool) == Some(true)
                    && let Some(current) = content
                        .elements
                        .iter()
                        .find(|element| element.get("id") == previous.get("id"))
                    && current.get("pendingImportReview").and_then(Value::as_bool) != Some(true)
                {
                    return Err(
                        "Imported content needs an explicit user review before activation".into(),
                    );
                }
            }
        }
        if content == draft.document.content && !history_move {
            return Ok(draft.document.clone());
        }
        if !history_move {
            draft.history.truncate(draft.cursor + 1);
            draft.history.push(content.clone());
            while draft.history.len() > 100
                || serde_json::to_vec(&draft.history)
                    .map_err(|_| "Invalid history")?
                    .len()
                    > MAX_HISTORY_BYTES
            {
                draft.history.remove(0);
            }
            draft.cursor = draft.history.len() - 1;
        }
        draft.document.content = content;
        draft.document.revision = draft
            .document
            .revision
            .checked_add(1)
            .ok_or("Composition revision exhausted")?;
        draft.document.last_modified = chrono::Utc::now().to_rfc3339();
        draft.document.can_undo = draft.cursor > 0;
        draft.document.can_redo = draft.cursor + 1 < draft.history.len();
        let document = draft.document.clone();
        self.persist(&candidate).await?;
        *library = candidate;
        let _ = self.changes.send(document.clone());
        Ok(document)
    }

    /// Hold authoring admission through the existing host publication transaction.
    /// A stale editor cannot publish HTML for a draft changed by another caller.
    pub async fn publication(
        &self,
        id: &str,
        expected_revision: u64,
    ) -> Result<PublicationGuard, String> {
        let library = self.library.clone().lock_owned().await;
        let draft = library.drafts.get(id).ok_or("Composition not found")?;
        if draft.document.revision != expected_revision {
            return Err("Stale composition revision; reload before publishing".into());
        }
        if draft.document.content.elements.iter().any(|element| {
            element.get("pendingImportReview").and_then(Value::as_bool) == Some(true)
        }) {
            return Err("Review imported custom content before publishing this composition".into());
        }
        Ok(PublicationGuard {
            library,
            service: self.clone(),
            id: id.into(),
        })
    }

    pub async fn reconcile_publication(&self, receipt: PublicationReceipt) -> Result<bool, String> {
        receipt.content.validate()?;
        let library = self.library.clone().lock_owned().await;
        let Some(draft) = library.drafts.get(&receipt.id) else {
            return Ok(false);
        };
        // An acknowledged or subsequently edited draft must never be rolled back
        // by an older active-output receipt.
        if draft.document.revision != receipt.expected_revision {
            return Ok(false);
        }
        if draft.document.published_revision == Some(draft.document.revision)
            && draft.document.content == receipt.content
        {
            return Ok(false);
        }
        PublicationGuard {
            library,
            service: self.clone(),
            id: receipt.id,
        }
        .acknowledge(Some(receipt.content))
        .await?;
        Ok(true)
    }
}

impl PublicationGuard {
    pub fn document(&self) -> &CompositionDocument {
        &self.library.drafts[&self.id].document
    }

    pub async fn acknowledge(
        mut self,
        normalized: Option<CompositionContent>,
    ) -> Result<CompositionDocument, String> {
        tokio::task::spawn_blocking(move || {
            let mut candidate = self.library.clone();
            let draft = candidate
                .drafts
                .get_mut(&self.id)
                .ok_or("Composition not found")?;
            if let Some(content) = normalized {
                content.validate()?;
                if content != draft.document.content {
                    draft.document.content = content.clone();
                    draft.history[draft.cursor] = content;
                    draft.document.revision = draft
                        .document
                        .revision
                        .checked_add(1)
                        .ok_or("Composition revision exhausted")?;
                    draft.document.last_modified = chrono::Utc::now().to_rfc3339();
                }
            }
            draft.document.published_revision = Some(draft.document.revision);
            let document = draft.document.clone();
            OverlayAuthoringService::persist_at(&self.service.path, &candidate)?;
            *self.library = candidate;
            let _ = self.service.changes.send(document.clone());
            Ok(document)
        })
        .await
        .map_err(|_| "Composition publication acknowledgement interrupted".to_string())?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn content() -> CompositionContent {
        CompositionContent {
            name: "Starting".into(),
            elements: vec![
                json!({"id":"chat","x":17,"y":21,"width":300,"height":400,"config":{"theme":"original"}}),
            ],
            settings: json!({"canvasWidth":1920,"canvasHeight":1080,"zoom":100,"theme":{"cssVars":{}}}),
        }
    }

    #[tokio::test]
    async fn atomic_edits_history_restart_and_publication_preserve_geometry() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("drafts.json");
        let service = OverlayAuthoringService::open(path.clone()).unwrap();
        let original = service.create("starting".into(), content()).await.unwrap();
        let mut changed = content();
        changed.elements[0]["x"] = json!(180);
        changed.elements[0]["config"]["theme"] = json!("new");
        let edited = service
            .edit(
                "starting",
                original.revision,
                CompositionEdit::Replace {
                    content: changed.clone(),
                },
            )
            .await
            .unwrap();
        assert_eq!(edited.published_revision, None);
        assert!(service
            .edit(
                "starting",
                original.revision,
                CompositionEdit::RemoveElements {
                    ids: vec!["chat".into()]
                }
            )
            .await
            .is_err());
        let undo = service
            .edit("starting", edited.revision, CompositionEdit::Undo)
            .await
            .unwrap();
        assert_eq!(undo.content, original.content);
        drop(service);
        let service = OverlayAuthoringService::open(path).unwrap();
        let redo = service
            .edit("starting", undo.revision, CompositionEdit::Redo)
            .await
            .unwrap();
        assert_eq!(redo.content, changed);
        assert!(service
            .publication("starting", edited.revision)
            .await
            .is_err());
        let published = service
            .publication("starting", redo.revision)
            .await
            .unwrap()
            .acknowledge(None)
            .await
            .unwrap();
        assert_eq!(published.published_revision, Some(redo.revision));
        let undo = service
            .edit("starting", published.revision, CompositionEdit::Undo)
            .await
            .unwrap();
        assert_eq!(undo.published_revision, Some(redo.revision));
        assert_eq!(undo.content.elements[0]["x"], 17);
    }

    #[tokio::test]
    async fn failed_persistence_and_invalid_batch_never_change_saved_draft() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("drafts.json");
        let service = OverlayAuthoringService::open(path.clone()).unwrap();
        let original = service.create("starting".into(), content()).await.unwrap();
        let mut invalid = content();
        invalid.elements.push(invalid.elements[0].clone());
        assert!(service
            .edit("starting", 1, CompositionEdit::Replace { content: invalid })
            .await
            .is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(service
            .edit(
                "starting",
                1,
                CompositionEdit::RemoveElements {
                    ids: vec!["chat".into()]
                }
            )
            .await
            .is_err());
        assert_eq!(
            service.read("starting").await.unwrap().content,
            original.content
        );
    }

    #[test]
    fn clipping_diagnostics_do_not_reposition_creator_elements() {
        let mut content = content();
        content.elements[0]["x"] = json!(2000);
        content.validate().unwrap();
        assert_eq!(content.diagnostics()[0]["code"], "outside_canvas");
        assert_eq!(content.elements[0]["x"], 2000);
    }

    #[tokio::test]
    async fn recovery_acknowledges_saved_output_once_without_overwriting_newer_edits() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("drafts.json");
        let service = OverlayAuthoringService::open(path.clone()).unwrap();
        service.create("starting".into(), content()).await.unwrap();
        let mut normalized = content();
        normalized.elements[0]["config"]["default"] = json!(true);
        let receipt = PublicationReceipt {
            id: "starting".into(),
            expected_revision: 1,
            content: normalized.clone(),
        };
        drop(service);
        let service = OverlayAuthoringService::open(path.clone()).unwrap();
        assert!(service
            .reconcile_publication(receipt.clone())
            .await
            .unwrap());
        let published = service.read("starting").await.unwrap();
        assert_eq!(published.content, normalized);
        assert_eq!(published.published_revision, Some(published.revision));
        assert!(!service
            .reconcile_publication(receipt.clone())
            .await
            .unwrap());
        let mut changed = normalized;
        changed.elements[0]["x"] = json!(720);
        service
            .edit(
                "starting",
                published.revision,
                CompositionEdit::Replace {
                    content: changed.clone(),
                },
            )
            .await
            .unwrap();
        assert!(!service.reconcile_publication(receipt).await.unwrap());
        assert_eq!(service.read("starting").await.unwrap().content, changed);
        let mut stored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        stored["drafts"]["starting"]["history"][0]["elements"][0]["width"] = json!(-10);
        std::fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
        assert!(OverlayAuthoringService::open(path).is_err());
    }

    #[tokio::test]
    async fn imported_content_is_a_new_inactive_draft_until_explicit_review() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("drafts.json");
        let service = OverlayAuthoringService::open(path).unwrap();
        service.create("starting".into(), content()).await.unwrap();
        let setup = super::super::overlay_portability::PortableSetup::export(
            &service.list().await,
            BTreeMap::new(),
        )
        .unwrap();
        let imported = super::super::overlay_portability::PortableSetup::decode(
            &serde_json::to_vec(&setup).unwrap(),
        )
        .unwrap();
        let documents = service.import_setup(imported).await.unwrap();
        let document = &documents[0];
        assert_ne!(document.id, "starting");
        assert_eq!(document.published_revision, None);
        assert!(service
            .publication(&document.id, document.revision)
            .await
            .is_err());
        let mut bypass = document.content.clone();
        bypass.elements[0]["pendingImportReview"] = Value::Bool(false);
        assert!(service
            .edit(
                &document.id,
                document.revision,
                CompositionEdit::Replace { content: bypass }
            )
            .await
            .is_err());
        let approved = service
            .approve_imported_content(&document.id, document.revision)
            .await
            .unwrap();
        assert!(service
            .publication(&approved.id, approved.revision)
            .await
            .is_ok());
        let undo = service
            .edit(&approved.id, approved.revision, CompositionEdit::Undo)
            .await
            .unwrap();
        assert!(service.publication(&undo.id, undo.revision).await.is_err());
        let redo = service
            .edit(&undo.id, undo.revision, CompositionEdit::Redo)
            .await
            .unwrap();
        assert!(service.publication(&redo.id, redo.revision).await.is_ok());
        assert_eq!(service.read("starting").await.unwrap().content, content());
    }
}
