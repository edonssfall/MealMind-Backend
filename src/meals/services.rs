use anyhow::Context;
use bytes::Bytes;
use uuid::Uuid;

use crate::photos::services::{upload_and_link_images, UploadItem};
use crate::state::AppState;

use super::dto::{CreatedMealRequest, CreatedMealResponse};

fn normalize_images(req: &CreatedMealRequest) -> anyhow::Result<Vec<(Bytes, String)>> {
    anyhow::ensure!(!req.images.is_empty(), "no images provided");

    let mut out = Vec::with_capacity(req.images.len());
    for (i, buf) in req.images.iter().enumerate() {
        let ct = req
            .content_types
            .get(i)
            .map(|s| s.as_str())
            .unwrap_or("image/jpeg")
            .to_string();

        out.push((Bytes::from(buf.clone().into_vec()), ct));
    }
    Ok(out)
}

pub async fn create_meal_with_images(
    st: &AppState,
    user_id: Uuid,
    req: CreatedMealRequest,
) -> anyhow::Result<CreatedMealResponse> {
    let normalized = normalize_images(&req)?;

    let mut tx = st.db.begin().await.context("begin tx")?;
    let (meal_id, created_at) = crate::meals::repo::create_meal_tx(tx.as_mut(), user_id).await?;
    tx.commit().await.context("commit meal")?;

    let imgs: Vec<UploadItem<'_>> = normalized
        .iter()
        .map(|(body, ct)| UploadItem {
            body: body.clone(),
            content_type: ct.as_str(),
        })
        .collect();

    let ids = upload_and_link_images(st, meal_id, imgs).await?;
    Ok(CreatedMealResponse {
        id: meal_id,
        created_at,
        images: ids,
    })
}

#[cfg(test)]
mod meals_tests {
    use super::*;
    use serde_bytes::ByteBuf;

    #[test]
    fn normalize_images_defaults_to_jpeg() {
        let req = CreatedMealRequest {
            images: vec![
                ByteBuf::from(vec![1, 2, 3]),
                ByteBuf::from(vec![4, 5, 6, 7]),
            ],
            content_types: vec![], // не передали — должно стать image/jpeg
        };

        let items = super::normalize_images(&req).expect("ok");
        assert_eq!(items.len(), 2);

        // порядок и содержимое
        assert_eq!(items[0].0.len(), 3);
        assert_eq!(items[1].0.len(), 4);

        // дефолтный content-type
        assert_eq!(items[0].1, "image/jpeg");
        assert_eq!(items[1].1, "image/jpeg");
    }

    #[test]
    fn normalize_images_respects_content_types() {
        let req = CreatedMealRequest {
            images: vec![ByteBuf::from(vec![9, 9]), ByteBuf::from(vec![8])],
            content_types: vec!["image/png".into(), "image/webp".into()],
        };

        let items = super::normalize_images(&req).expect("ok");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].1, "image/png");
        assert_eq!(items[1].1, "image/webp");
    }

    #[test]
    fn normalize_images_empty_is_error() {
        let req = CreatedMealRequest {
            images: vec![],
            content_types: vec![],
        };
        let err = super::normalize_images(&req).unwrap_err();
        assert!(err.to_string().contains("no images provided"));
    }
}
