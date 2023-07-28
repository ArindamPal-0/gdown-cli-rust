# GDown

Google Drive Folder Downloader, Rust CLI

## Description

Downloading a complete folder from google drive triggers creation of an archive of the folder first, which takes ages. So, this cli tool can be used to download a complete folder one file at a time without creating an archive.

This is rust implementation of the [gdown](https://github.com/ArindamPal-0/gdown) project which is written in Typescript.

## Setup

```shell
cargo run
```

Create a new project in [google cloud console](https://console.cloud.google.com/), enable the [google drive api](https://console.cloud.google.com/flows/enableapi?apiid=drive.googleapis.com) for your google cloud project.
Now create a service account under [credentials](https://console.cloud.google.com/apis/credentials) section. For this service account, create and download a new json key, name it `credentials.json` and put it in the root directory.

Make sure that the drive scope are as follows:
- https://www.googleapis.com/auth/drive.metadata.readonly (View metadata for files in your Drive.)
- https://www.googleapis.com/auth/drive.readonly (View and download all your Drive files.)

Just put the `credentials.json` file in the project root and the bearer token will be automatically fetched from the google auth server, and it will be used for further requests.

After that add the google drive file id you want to download into the `file_id` variable in `main.rs` file. Then you can run the application to download the file.

## Todo

- [x] connect to google drive api
- [x] auth using service account
- [ ] auth using oauth
- [x] get file details from file id
- [x] download a file
- [ ] get file list from folder drive id
- [ ] download all files in a folder
- [ ] download a folder recursively
- [ ] create a cli application
- [ ] run on separate threads
