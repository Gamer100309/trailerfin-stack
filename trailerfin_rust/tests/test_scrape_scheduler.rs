use std::env;
use tempfile::tempdir;

#[tokio::test]
async fn test_scheduler_triggers_scan() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;
    use std::{env, sync::Arc};
    use trailerfin_rust::configuration::configuration_provider::ConfigurationProvider;
    use trailerfin_rust::schedulers::{get_scraping_scheduler, initialize_schedulers};
    use trailerfin_rust::schedulers::types::ScanFn;

    initialize_schedulers();

    let temp = setup_empty_dir();

    static CALLED: AtomicUsize = AtomicUsize::new(0);

    unsafe {
        env::set_var("TRAILERFIN_VIDEO_FILENAME", "test.strm");
        env::set_var("TRAILERFIN_USER_AGENT", "TestAgent");
        env::set_var("TRAILERFIN_SHOULD_SCHEDULE", "true");
        env::set_var("TRAILERFIN_SCHEDULE", "*/1 * * * * *");
    }

    let config = ConfigurationProvider::load_config().expect("Expected config to load");

    let mock_scan: ScanFn = Arc::new(|_cfg| {
        Box::pin(async {
            CALLED.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    });

    let is_running = Arc::new(AtomicBool::new(false));

    let sched = get_scraping_scheduler()
        .setup_scheduler_with_lock(config, Some(mock_scan), is_running)
        .await
        .unwrap();

    sched.start().await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    let config = ConfigurationProvider::load_config().expect("Expected config to load");

    assert_eq!(&config.scan_path, temp.path().join("scan-me").to_str().unwrap());
    assert!(CALLED.load(Ordering::SeqCst) >= 1);
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