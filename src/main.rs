use anyhow::{Context, Result};
use indicatif::ProgressBar;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::{env, fs, io::Write};
use tokio_stream::StreamExt;

// #[allow(dead_code)]
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
struct File {
    id: String,
    name: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    #[serde_as(as = "DisplayFromStr")]
    size: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("GDown");

    dotenv::dotenv().ok();

    /* Get bearer token from env variable */
    let token: String =
        env::var("BEARER_TOKEN").expect("BEARER_TOKEN not provided as env variable");

    /* Set google drive file id */
    // file.txt
    let file_id = "1NuuL9qNo5BJYnfNqN_lxBOUN0P-AociQ";
    /* request to get file metadata */
    let request_url = format!("https://www.googleapis.com/drive/v3/files/{}", file_id);
    let request_url = request_url.as_str();

    let client = reqwest::Client::new();
    let request = client
        .get(request_url)
        .query(&[("fields", "id, name, mimeType, size")])
        .bearer_auth(&token)
        .send();

    // FIXME: check for token expiry or invalid auth error
    let response = request.await?;

    /* Quickly print the json data */
    // let data: serde_json::Value = response.json().await?;
    // println!("{data}");

    // return Ok(());

    /* Get and parse File from the response */
    let file: File = response
        .json()
        .await
        .context("auth token has expired. Hence getting error response.")?;
    println!("{:?}", file);

    /* Generate the download folder if not exists */
    let download_folder = env::current_dir()?.as_path().join("downloads");

    // println!("folder_path: {}", download_folder.display());

    if !download_folder.exists() {
        println!("{} does not exists, creating it.", download_folder.display());
    }

    fs::create_dir_all(&download_folder).context(format!(
        "{} could not be created",
        download_folder.display()
    ))?;

    let file_name = file.name.clone();
    let file_path = download_folder.join(file_name);

    // println!("file_path: {}", file_path.display());

    /* request to download the file */
    let client = reqwest::Client::new();
    let download_request = client
        .get(request_url)
        .query(&[("alt", "media")])
        .bearer_auth(&token)
        .send();

    let download_response = download_request.await.context("download request failed")?;

    // FIXME: proper alternative if content_length is not defined
    let content_length = download_response
        .content_length()
        .context("content_length is not defined")?;

    /* Downloading the file */
    // get the file bytes
    // let download_data = download_response.bytes().await?;
    let mut download_stream = download_response.bytes_stream();

    // create/open the file
    let mut file_handle = fs::File::create(file_path.clone())
        .context(format!("could not open file: {}", file_path.display()))?;

    // TODO: add proper size conversion (B, KB, MB, GB)
    // TODO: create a utility function to return size w/ unit
    let file_size_in_megabytes = file.size / (1024 * 1024);

    println!("Downloading: {} ({} MB)", file.name, file_size_in_megabytes);
    
    // TODO: update progress_bar to include size downloaded.
    let bar = ProgressBar::new(100);
    let mut downloaded_length: u64 = 0;

    while let Some(chunk) = download_stream.next().await {
        // get the chunks from download stream
        let chunk_bytes = &chunk?;

        // update the progress bar
        let chunk_length = chunk_bytes.len();
        // println!("chunk length: {}", chunk_length);
        downloaded_length += chunk_length as u64;
        let progress_length = (downloaded_length * 100) / content_length;
        bar.update(|progress_state| {
            progress_state.set_pos(progress_length);
        });

        // write to file
        file_handle.write_all(chunk_bytes)?;
        file_handle.flush()?;
    }

    bar.finish();

    Ok(())
}
