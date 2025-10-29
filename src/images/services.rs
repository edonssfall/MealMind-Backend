use anyhow::Context;
use bytes::Bytes;
use uuid::Uuid;

use super::repo;
use crate::state::AppState;

pub struct UploadItem<'a> {
    pub body: Bytes,
    pub content_type: &'a str,
}

pub async fn upload_and_link_images(
    st: &AppState,
    user_id: Uuid,
    meal_id: Uuid,
    images: Vec<UploadItem<'_>>,
) -> anyhow::Result<Vec<Uuid>> {
    anyhow::ensure!(!images.is_empty(), "no images provided");

    struct Obj {
        id: Uuid,
        key: String,
    }
    let mut objs = Vec::with_capacity(images.len());
    for img in images {
        let id = Uuid::new_v4();
        let ext = ext_from_mime(img.content_type).unwrap_or("bin");
        let key = format!("meals/{}/{}-{}.{}", user_id, meal_id, id, ext);
        st.storage
            .put_object(&key, img.body, img.content_type)
            .await
            .with_context(|| format!("put_object {}", key))?;
        objs.push(Obj { id, key });
    }

    let mut tx = st.db.begin().await.context("begin tx")?;
    for o in &objs {
        repo::insert_photo_tx(&mut tx, o.id, user_id, Some(meal_id), &o.key).await?;
    }
    tx.commit().await.context("commit tx")?;

    Ok(objs.into_iter().map(|o| o.id).collect())
}

pub async fn presign_many(
    st: &AppState,
    keys: Vec<String>,
    expires_seconds: u64,
) -> anyhow::Result<Vec<String>> {
    let mut out = Vec::with_capacity(keys.len());
    for k in keys {
        out.push(st.storage.presign_get(&k, expires_seconds).await?);
    }
    Ok(out)
}

fn ext_from_mime(ct: &str) -> Option<&'static str> {
    match ct {
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/heic" => Some("heic"),
        _ => None,
    }
}

pub async fn presign_by_photo_id(st: &AppState, s3_key: String) -> anyhow::Result<String> {
    const TTL_SECS: u64 = 30 * 60;
    st.storage
        .presign_get(&s3_key, TTL_SECS)
        .await
        .with_context(|| format!("presign url for s3_key {}", s3_key))
}

#[cfg(test)]
mod image_tests {
    use crate::state::AppState;

    #[test]
    fn test_ext_from_mime() {
        assert_eq!(super::ext_from_mime("image/jpeg"), Some("jpg"));
        assert_eq!(super::ext_from_mime("image/jpg"), Some("jpg"));
        assert_eq!(super::ext_from_mime("image/png"), Some("png"));
        assert_eq!(super::ext_from_mime("image/webp"), Some("webp"));
        assert_eq!(super::ext_from_mime("image/heic"), Some("heic"));
        assert_eq!(super::ext_from_mime("application/octet-stream"), None);
        assert_eq!(super::ext_from_mime("whatever/else"), None);
    }

    #[tokio::test]
    async fn test_presign_many_and_one() {
        let state = AppState::fake();

        let urls = super::presign_many(&state, vec!["a/b/c.jpg".into(), "x/y/z.png".into()], 1800)
            .await
            .unwrap();
        assert_eq!(urls.len(), 2);
        assert!(urls[0].contains("a/b/c.jpg"));
        assert!(urls[1].contains("x/y/z.png"));

        let one = super::presign_by_photo_id(&state, "q/w/e.webp".into())
            .await
            .unwrap();
        assert!(one.contains("q/w/e.webp"));
    }
}
