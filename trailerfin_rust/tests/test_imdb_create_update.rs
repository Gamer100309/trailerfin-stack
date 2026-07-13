use std::fs::{self};
use tempfile::tempdir;
use trailerfin_rust::configuration::configuration_provider::{AppConfig, DataSource};
use trailerfin_rust::scrapers::imdb_trailers::ImdbTrailerScraper;

#[tokio::test]
async fn test_create_strm_file_writes_correct_url() {
    let dir = tempdir().unwrap();
    let folder = dir.path();

    let config = AppConfig {
        scan_path: folder.to_string_lossy().to_string(),
        video_filename: "video1.strm".to_string(),
        should_schedule: false,
        schedule: Some("* * * * *".to_string()),
        user_agent: "TestAgent".to_string(),
        threads: 1,
        cache_path: "./data/cache".to_string(),
        data_source: DataSource::Imdb,
        imdb_rate_limit: "30/minute".to_string(),
        tmdb_rate_limit: "50/second".to_string(),
        tmdb_api_key: Some("ham-and-cheese-sandwich".to_string()),
        tv_folders: vec!["shows", "kids tv"].iter().map(|s| s.to_string()).collect(),
        movie_folders: vec!["movies", "kids"].iter().map(|s| s.to_string()).collect(),
    };

    let url = "https://example.com/video.mp4";
    let scraper = ImdbTrailerScraper {};
    scraper.create_or_update_strm_file(folder, &config, url).unwrap();

    let written = fs::read_to_string(folder.join("backdrops").join("video1.strm")).unwrap();
    assert_eq!(written, url);
}

#[tokio::test]
async fn test_overwrite_existing_strm_file() {
    let dir = tempdir().unwrap();
    let folder = dir.path();
    let backdrops = folder.join("backdrops");
    fs::create_dir_all(&backdrops).unwrap();

    let file_path = backdrops.join("video1.strm");
    fs::write(&file_path, "old_url").unwrap();

    let config = AppConfig {
        scan_path: folder.to_string_lossy().to_string(),
        video_filename: "video1.strm".to_string(),
        should_schedule: false,
        schedule: Some("* * * * *".to_string()),
        user_agent: "TestAgent".to_string(),
        threads: 1,
        cache_path: "./data/cache".to_string(),
        data_source: DataSource::Imdb,
        imdb_rate_limit: "30/minute".to_string(),
        tmdb_rate_limit: "50/second".to_string(),
        tmdb_api_key: Some("ham-and-cheese-sandwich".to_string()),
        tv_folders: vec!["shows", "kids tv"].iter().map(|s| s.to_string()).collect(),
        movie_folders: vec!["movies", "kids"].iter().map(|s| s.to_string()).collect(),
    };

    let new_url = "https://example.com/new_video.mp4";
    let scraper = ImdbTrailerScraper {};
    scraper.create_or_update_strm_file(folder, &config, new_url).unwrap();

    let written = fs::read_to_string(file_path).unwrap();
    assert_eq!(written, new_url);
}