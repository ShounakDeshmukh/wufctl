pub mod convert;
pub mod model;

pub use model::{ActionResult, Image, RequestListEntry, RequestStatus};

use color_eyre::eyre::{ContextCompat, Result};

pub async fn get_images(client: &vcl_lib::VclClient) -> Result<Vec<Image>> {
    let value = client.get_images().await?;
    value
        .as_array()
        .context("expected get_images to return an array")?
        .iter()
        .map(Image::try_from)
        .collect()
}

pub async fn get_request_ids(client: &vcl_lib::VclClient) -> Result<Vec<RequestListEntry>> {
    let value = client.get_request_ids().await?;
    let requests = value
        .get("requests")
        .context("missing `requests`")?
        .as_array()
        .context("expected `requests` to be an array")?;
    requests.iter().map(RequestListEntry::try_from).collect()
}

pub async fn get_request_status(client: &vcl_lib::VclClient, id: i64) -> Result<RequestStatus> {
    RequestStatus::try_from(&client.get_request_status(id).await?)
}

/// `start` is `"now"` or a Unix timestamp.
pub async fn add_request(
    client: &vcl_lib::VclClient,
    image_id: i64,
    start: &str,
    length: i64,
) -> Result<ActionResult> {
    let value = client
        .add_request(image_id, start, length, None, false)
        .await?;
    ActionResult::try_from(&value)
}

pub async fn end_request(client: &vcl_lib::VclClient, id: i64) -> Result<ActionResult> {
    ActionResult::try_from(&client.end_request(id).await?)
}

pub async fn extend_request(
    client: &vcl_lib::VclClient,
    id: i64,
    minutes: i64,
) -> Result<ActionResult> {
    ActionResult::try_from(&client.extend_request(id, minutes).await?)
}
