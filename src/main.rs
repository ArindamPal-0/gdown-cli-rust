// #![allow(dead_code, unreachable_code)]
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{
    env, fs,
    io::{Read, Write},
};
use tokio_stream::StreamExt;

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
struct DFile {
    id: String,
    name: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    #[serde_as(as = "DisplayFromStr")]
    size: u32,
}

// to get json data from credentials.json
#[derive(Debug, Clone, Deserialize)]
struct CredJSON {
    private_key: String,  // for JWT secret used for encoding
    client_email: String, // for `JWTClaim.iss`
    token_uri: String,    // for `JWTClaim.aud`
}

// for jwt clam set
#[derive(Deserialize, Serialize, Debug)]
struct JWTClaim {
    iss: String,   // The email address of the service account.
    scope: String, // A space-delimited list of the permissions that the application requests.
    aud: String,   // value is always https://oauth2.googleapis.com/token.
    exp: usize, // The expiration time of the assertion, specified as seconds since 00:00:00 UTC, January 1, 1970. This value has a maximum of 1 hour after the issued time.
    iat: usize, // The time the assertion was issued, specified as seconds since 00:00:00 UTC, January 1, 1970.
}

// example:
// JWTClaim {
//   "iss": "761326798069-r5mljlln1rd4lrbhg75efgigp36m78j5@developer.gserviceaccount.com",
//   "sub": "some.user@example.com",
//   "scope": "https://www.googleapis.com/auth/prediction",
//   "aud": "https://oauth2.googleapis.com/token",
//   "exp": 1328554385,
//   "iat": 1328550785
// }

// for google auth token request body
#[derive(Debug, Clone, Serialize)]
struct TokenReqBody {
    grant_type: String,
    assertion: String,
}

// for google auth token response body
#[derive(Debug, Clone, Deserialize)]
struct TokenJSONRes {
    access_token: String,
    expires_in: usize,
    token_type: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("GDown");

    /* Read credentials.json */
    let credentials_path = env::current_dir()?.as_path().join("credentials.json");
    let mut credentials_file_handle = fs::File::open(credentials_path)?;

    let mut cred_json_string = String::new();
    credentials_file_handle.read_to_string(&mut cred_json_string)?;

    // let json_credentials: serde_json::Value = serde_json::from_str(&json_string)?;
    // println!("{}", json_credentials);

    // return Ok(());

    /* parse credentials.json into CredJSON */
    let cred_json: CredJSON = serde_json::from_str(&cred_json_string)?;
    // println!("{:?}", cred_json);

    /* Creating Jsonwebtoken */

    // current time
    let iat = Utc::now().timestamp();
    // expiry time of 1 hr from current time
    let exp = Utc::now()
        .checked_add_signed(Duration::hours(1))
        .expect("invalid duration in exp time")
        .timestamp();
    let scope = [
        "https://www.googleapis.com/auth/drive.metadata.readonly", // View metadata for files in your Drive.
        "https://www.googleapis.com/auth/drive.readonly", // View and download all your Drive files.
    ]
    .join(" ");
    let token_uri = cred_json.token_uri.to_string();

    let jwt_claim = JWTClaim {
        iss: cred_json.client_email.to_owned(),
        scope: scope.to_owned(),
        aud: token_uri.clone(),
        exp: exp as usize,
        iat: iat as usize,
    };

    let jwt_header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);

    let jwt_key = jsonwebtoken::EncodingKey::from_rsa_pem(cred_json.private_key.as_bytes())?;

    let jwt = jsonwebtoken::encode(&jwt_header, &jwt_claim, &jwt_key)
        .context("could not create jsonwebtoken.")?;
    // println!("jwt: {}", jwt);

    let token_req_body = TokenReqBody {
        grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer".to_string(),
        assertion: jwt,
    };

    // requires `cargo add serde_urlencoded`
    // let body = serde_urlencoded::to_string(token_req_body.clone())?;
    // println!("{}", body);

    /* Requesting Bearer token from google auth server by passing the created jwt */

    let client = reqwest::Client::new();
    let token_request = client.post(token_uri).form(&token_req_body).send();

    let token_response = token_request.await?;

    // let token_res_data: serde_json::Value = token_response.json().await?;
    // println!("token_req_data: {:#?}", token_res_data);

    let token_json_res: TokenJSONRes = token_response.json().await?;
    // println!("token_json_res: {:#?}", token_json_res);

    /* Get bearer token from token_response */
    let token = token_json_res.access_token.to_owned();

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

    // FIXME: check for the following errors:
    // 1. token expiry or invalid auth error
    // 2. not authorized to access the file
    let response = request.await?;

    /* Quickly print the json data */
    // let data: serde_json::Value = response.json().await?;
    // println!("{data}");

    // return Ok(());

    /* Get and parse File from the response */
    let file: DFile = response
        .json()
        .await
        .context("auth token has expired. Hence getting error response.")?;
    println!("{:?}", file);

    /* Generate the download folder if not exists */
    let download_folder = env::current_dir()?.as_path().join("downloads");

    // println!("folder_path: {}", download_folder.display());

    if !download_folder.exists() {
        println!(
            "{} does not exists, creating it.",
            download_folder.display()
        );

    fs::create_dir_all(&download_folder).context(format!(
        "{} could not be created",
        download_folder.display()
    ))?;
    }


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
