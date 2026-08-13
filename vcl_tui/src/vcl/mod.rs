pub mod convert;
pub mod model;

pub use model::Image;

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
