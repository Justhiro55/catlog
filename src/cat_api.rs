use anyhow::Result;

pub async fn fetch_cat_image_for_status(status_code: u16) -> Result<Vec<u8>> {
    let url = format!("https://http.cat/{}", status_code);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}
