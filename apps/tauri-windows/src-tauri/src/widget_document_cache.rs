use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use vortideck_core::db::models::DeckWidgetDocument;

const CACHE_TTL: Duration = Duration::from_secs(30);

struct CachedDocument {
    document: DeckWidgetDocument,
    loaded_at: Instant,
}

fn cache() -> &'static Mutex<Option<CachedDocument>> {
    static CACHE: OnceLock<Mutex<Option<CachedDocument>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub async fn get(deck_id: i32) -> Option<DeckWidgetDocument> {
    let cache = cache().lock().await;
    cache
        .as_ref()
        .filter(|cached| {
            cached.document.deck_id == deck_id && cached.loaded_at.elapsed() < CACHE_TTL
        })
        .map(|cached| cached.document.clone())
}

pub async fn store(document: DeckWidgetDocument) {
    *cache().lock().await = Some(CachedDocument {
        document,
        loaded_at: Instant::now(),
    });
}

pub async fn invalidate(deck_id: i32) {
    let mut cache = cache().lock().await;
    if cache
        .as_ref()
        .is_some_and(|cached| cached.document.deck_id == deck_id)
    {
        *cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(deck_id: i32, revision: i32) -> DeckWidgetDocument {
        DeckWidgetDocument {
            deck_id,
            revision,
            updated_at: String::new(),
            items: Vec::new(),
        }
    }

    #[tokio::test]
    async fn invalidation_only_removes_the_matching_deck() {
        store(document(4, 2)).await;
        invalidate(3).await;
        assert_eq!(get(4).await.unwrap().revision, 2);
        invalidate(4).await;
        assert!(get(4).await.is_none());
    }
}
