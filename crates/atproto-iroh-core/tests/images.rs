//! Live proof of the Images primitive: one node uploads an image, a
//! second node (joined after the upload) syncs both the metadata and
//! the raw bytes with no separate blob-fetch step, confirming
//! `namespace::put_bytes`'s "content syncs with the entry" claim for
//! real binary content, not just text.

use atproto_iroh_core::{
    images::{list_images, load_image_bytes, upload_image},
    namespace::Node,
};
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

#[tokio::test]
async fn an_uploaded_image_syncs_metadata_and_bytes_to_a_new_peer() -> anyhow::Result<()> {
    let founder_node = Node::spawn().await?;
    let viewer_node = Node::spawn().await?;

    let founder = founder_node.author_create().await?;
    let doc = founder_node.create_namespace().await?;

    // Not a real PNG — arbitrary bytes are enough to prove the sync
    // mechanism; this module doesn't parse or validate image content.
    let image_bytes: Vec<u8> = vec![0x89, b'P', b'N', b'G', 1, 2, 3, 4, 5];
    let rkey = upload_image(
        &doc,
        founder,
        image_bytes.clone(),
        "image/png".into(),
        Some("a test image".into()),
    )
    .await?;

    let ticket = founder_node.share(&doc, ShareMode::Write).await?;
    let viewer_doc = viewer_node.join(ticket).await?;

    let deadline = Instant::now() + Duration::from_secs(20);
    let images = loop {
        match list_images(&viewer_node, &viewer_doc).await {
            Ok(images) if !images.is_empty() => break images,
            _ => {}
        }
        if Instant::now() >= deadline {
            panic!("image metadata never synced within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].0, founder);
    assert_eq!(images[0].1, rkey);
    assert_eq!(images[0].2.content_type, "image/png");
    assert_eq!(images[0].2.len, image_bytes.len() as u64);
    assert_eq!(images[0].2.caption.as_deref(), Some("a test image"));

    // Bytes are a separate entry with their own content-blob lag
    // (namespace.rs's documented finding) — retry the same way
    // get_record's own tests do.
    let deadline = Instant::now() + Duration::from_secs(20);
    let bytes = loop {
        match load_image_bytes(&viewer_node, &viewer_doc, founder, &rkey).await {
            Ok(Some(bytes)) => break bytes,
            _ => {}
        }
        if Instant::now() >= deadline {
            panic!("image bytes never synced within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };
    assert_eq!(bytes.to_vec(), image_bytes);

    founder_node.shutdown().await;
    viewer_node.shutdown().await;
    Ok(())
}
