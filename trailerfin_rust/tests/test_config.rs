use std::env;
use std::path::{Path};
use tempfile::tempdir;
use trailerfin_rust::configuration::configuration_provider::ConfigurationProvider;

#[test]
#[serial_test::serial]
fn test_valid_env_config_loads_successfully() {
    clear_env();
    let temp = setup_empty_dir();
    unsafe {
        env::set_var("TRAILERFIN_VIDEO_FILENAME", "test.strm");
        env::set_var("TRAILERFIN_USER_AGENT", "TestAgent/1.0");
        env::set_var("TRAILERFIN_SHOULD_SCHEDULE", "false");
        env::set_var("TRAILERFIN_SCHEDULE", "0 0 * * *");
        env::set_var("TRAILERFIN_THREADS", "4");
    }

    let config = ConfigurationProvider::load_config().expect("Expected config to load");

    assert_eq!(
        Path::new(&config.scan_path).canonicalize().unwrap(),
        temp.path().join("scan-me").canonicalize().unwrap()
    );
    assert_eq!(config.video_filename, "test.strm");
    assert_eq!(config.user_agent, "TestAgent/1.0");
    assert_eq!(config.threads, 4);
    assert_eq!(config.tv_folders.len(), 2);
    assert_eq!(config.tv_folders[0], "Tv Shows");
    assert_eq!(config.tv_folders[1], "Kids TV");
    assert_eq!(config.movie_folders.len(), 2);
    assert_eq!(config.movie_folders[0], "Movies");
    assert_eq!(config.movie_folders[1], "Kids");
    assert!(!config.should_schedule);
    clear_env();
}

#[test]
#[serial_test::serial]
fn test_missing_scan_path_fails() {
    clear_env();

    unsafe {
        env::set_var("TRAILERFIN_VIDEO_FILENAME", "x.strm");
        env::set_var("TRAILERFIN_USER_AGENT", "x");
        env::set_var("TRAILERFIN_SHOULD_SCHEDULE", "false");
    }

    let result = ConfigurationProvider::load_config();
    assert!(result.is_err());
    clear_env();
}

#[test]
#[serial_test::serial]
fn test_threads_less_than_one_fails() {
    clear_env();
    let _temp = setup_empty_dir();
    unsafe {
        env::set_var("TRAILERFIN_VIDEO_FILENAME", "test.strm");
        env::set_var("TRAILERFIN_USER_AGENT", "TestAgent/1.0");
        env::set_var("TRAILERFIN_SHOULD_SCHEDULE", "false");
        env::set_var("TRAILERFIN_SCHEDULE", "0 0 * * *");
        env::set_var("TRAILERFIN_THREADS", "-1");
    }

    let result = ConfigurationProvider::load_config();
    assert!(result.is_err());
    clear_env();
}

#[test]
#[serial_test::serial]
fn test_empty_video_filename_fails() {
    clear_env();
    let _temp = setup_empty_dir();
    unsafe {
        env::set_var("TRAILERFIN_VIDEO_FILENAME", "");
        env::set_var("TRAILERFIN_USER_AGENT", "TestAgent");
        env::set_var("TRAILERFIN_SHOULD_SCHEDULE", "false");
    }

    let result = ConfigurationProvider::load_config();
    assert!(result.is_err());
    clear_env();
}

#[test]
fn test_schedule_required_when_enabled() {
    clear_env();
    let _temp = setup_empty_dir();
    unsafe {
        env::set_var("TRAILERFIN_VIDEO_FILENAME", "video.strm");
        env::set_var("TRAILERFIN_USER_AGENT", "TestAgent");
        env::set_var("TRAILERFIN_SHOULD_SCHEDULE", "true");
        env::set_var("TRAILERFIN_SCHEDULE", "");
    }

    let result = ConfigurationProvider::load_config();
    assert!(result.is_err());
    clear_env();
}

fn clear_env() {
    unsafe {
        for key in [
            "TRAILERFIN_SCAN_PATH",
            "TRAILERFIN_VIDEO_FILENAME",
            "TRAILERFIN_USER_AGENT",
            "TRAILERFIN_SHOULD_SCHEDULE",
            "TRAILERFIN_SCHEDULE",
        ] {
            env::remove_var(key);
        }
    }
}

fn setup_empty_dir() -> tempfile::TempDir {
    let temp = tempdir().unwrap();

    let scan_path = temp.path().join("scan-me");
    let cache_path = temp.path().join("cache-me");

    for subdir in ["Tv Shows", "Kids TV", "Movies", "Kids"] {
        std::fs::create_dir_all(scan_path.join(subdir)).unwrap();
    }

    std::fs::create_dir_all(&cache_path).unwrap();
    std::fs::create_dir_all(&scan_path).unwrap();

    unsafe {
        env::set_var("TRAILERFIN_SCAN_PATH", scan_path);
        env::set_var("TRAILERFIN_CACHE_PATH", cache_path);
        env::set_var("TRAILERFIN_TV_FOLDERS", "Tv Shows, Kids TV");
        env::set_var("TRAILERFIN_MOVIE_FOLDERS", "Movies, Kids");
    }

    temp
}