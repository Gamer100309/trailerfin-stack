use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::tempdir;
use trailerfin_rust::scrapers::imdb_trailers::ImdbTrailerScraper;

#[tokio::test]
async fn test_expired_strm_file_with_expired_timestamp() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("expired.strm");

    let expired_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64 - 100;
    let url = format!("https://video.test/video.mp4?Expires={}", expired_ts);
    let scraper = ImdbTrailerScraper {};
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "{}", url).unwrap();

    assert!(scraper.is_strm_expired(&file_path).unwrap());
}

#[tokio::test]
async fn test_valid_strm_file_with_future_expiry() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("valid.strm");

    let future_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64 + 99999;
    let url = format!("https://video.test/video.mp4?Expires={}", future_ts);
    let scraper = ImdbTrailerScraper {};
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "{}", url).unwrap();

    assert!(!scraper.is_strm_expired(&file_path).unwrap());
}

#[tokio::test]
async fn test_strm_file_missing_expiry() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("no_expiry.strm");
    let scraper = ImdbTrailerScraper {};
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "https://video.test/video.mp4").unwrap();

    assert!(scraper.is_strm_expired(&file_path).unwrap());
}

#[tokio::test]
async fn test_strm_file_missing_entirely() {
    let bogus_path = PathBuf::from("totally_missing.strm");
    let scraper = ImdbTrailerScraper {};
    assert!(scraper.is_strm_expired(&bogus_path).unwrap());
}