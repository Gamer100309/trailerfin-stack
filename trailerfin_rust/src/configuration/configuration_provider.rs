use anyhow::{anyhow, Context};
use std::{path::PathBuf, sync::Arc};
use std::path::Path;
use config::Config;
use tracing::{info};
use serde::de::{self, Deserializer};
use serde::Deserialize;

const DATASOURCES: [&str; 2] = ["IMDB", "TMDB"];

fn case_insensitive_datasource<'de, D>(deserializer: D) -> Result<DataSource, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.to_lowercase().as_str() {
        "imdb" => Ok(DataSource::Imdb),
        "tmdb" => Ok(DataSource::Tmdb),
        other => Err(de::Error::custom(format!("invalid TRAILERFIN_DATA_SOURCE: {}. Must be one of: {:?}", other, DATASOURCES)))
    }
}

fn case_insensitive_mode<'de, D>(deserializer: D) -> Result<TrailerMode, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.to_lowercase().as_str() {
        "link" => Ok(TrailerMode::Link),
        "download" => Ok(TrailerMode::Download),
        other => Err(de::Error::custom(format!("invalid TRAILERFIN_MODE: {}. Must be one of: [\"link\", \"download\"]", other)))
    }
}

fn validate_path(path: &str, name: &str) -> anyhow::Result<PathBuf> {
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() || !path_buf.is_dir() {
        return Err(anyhow::anyhow!(
                "Provided path for {} does not exist or is not a directory: {:?}",
                name,
                path_buf
            ));
    }
    Ok(path_buf.canonicalize().with_context(|| format!("Failed to canonicalize path for {}: {:?}", name, path_buf))?)
}

fn deserialize_trimmed_csv<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.split(',')
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect())
}

#[derive(Debug, Default, serde::Deserialize, PartialEq)]
pub enum DataSource {
    #[default]
    Imdb,
    Tmdb,
}

#[derive(Debug, Default, serde::Deserialize, PartialEq, Clone, Copy)]
pub enum TrailerMode {
    #[default]
    Link,
    Download,
}

#[derive(Debug, Default, serde::Deserialize, PartialEq)]
pub struct AppConfig {
    pub scan_path: String,
    pub video_filename: String,
    pub should_schedule: bool,
    pub schedule: Option<String>,
    pub user_agent: String,
    pub threads: usize,
    pub cache_path: String,

    #[serde(default)]
    #[serde(deserialize_with = "case_insensitive_datasource")]
    pub data_source: DataSource,

    pub imdb_rate_limit: String,
    pub tmdb_rate_limit: String,
    pub tmdb_api_key: Option<String>,

    /// Base URL of a ytdlp2STRM (or compatible) resolver proxy, e.g. "http://ytdlp2strm:5000".
    /// When set, TMDB-based trailer resolution bypasses IMDb scraping entirely: it fetches
    /// the official YouTube trailer key via TMDB's own /videos endpoint and writes a .strm
    /// file pointing at "{ytdlp_proxy_base_url}/youtube/direct/{key}" instead of scraping
    /// IMDb's (now bot-protected) website for a direct CDN video URL.
    pub ytdlp_proxy_url: Option<String>,

    /// Preferred language for trailers, e.g. "de-DE". If a trailer exists in
    /// this language on TMDB, it's used; otherwise falls back to the default
    /// (usually English/original) trailer. Leave unset to always use the
    /// default behavior.
    pub preferred_language: Option<String>,

    /// "link" (default): writes a .strm file pointing to the ytdlp2STRM
    /// proxy (no local storage used). "download": actually downloads the
    /// resolved video and saves it as an .mp4 file instead.
    #[serde(default)]
    #[serde(deserialize_with = "case_insensitive_mode")]
    pub mode: TrailerMode,

    /// Optional path to a Netscape-format cookies.txt file (mounted into
    /// this container), used to forward youtube.com/google.com cookies on
    /// the actual video download request. Some CDN URLs generated via an
    /// authenticated request require this to avoid a 403 Forbidden.
    pub download_cookies_file: Option<String>,

    /// Priority-ordered list of preferred YouTube channel names (e.g. official
    /// studio channels). If a trailer candidate's channel matches an entry
    /// here, it's preferred over others - earlier entries win ties. Matching
    /// is case-insensitive substring matching against the channel name.
    #[serde(default, deserialize_with = "deserialize_trimmed_csv")]
    pub channel_whitelist: Vec<String>,

    /// Channel names to exclude entirely, even if they'd otherwise be the
    /// best/only candidate. Case-insensitive substring matching.
    #[serde(default, deserialize_with = "deserialize_trimmed_csv")]
    pub channel_blacklist: Vec<String>,

    #[serde(default, deserialize_with = "deserialize_trimmed_csv")]
    pub tv_folders: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_trimmed_csv")]
    pub movie_folders: Vec<String>,
}

#[derive(Debug)]
pub struct ConfigurationProvider;

impl ConfigurationProvider {
    pub fn load_config() -> anyhow::Result<Arc<AppConfig>> {
        let config = Config::builder()
            .set_default("scan_path", "/mnt/plex")?
            .set_default("user_agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124.0.0.0")?
            .set_default("should_schedule", false)?
            .set_default("video_filename", "video1.strm")?
            .set_default("threads", 1)?
            .set_default("cache_path", "/config")?
            .set_default("data_source", "IMDB")?
            .set_default("mode", "link")?
            .set_default("imdb_rate_limit", "30/minute")?
            .set_default("tmdb_rate_limit", "50/second")?
            .add_source(
                config::Environment::with_prefix("TRAILERFIN")
            )
            .build()?;

        let config: AppConfig = config.try_deserialize()?;

        if config.threads < 1 {
            return Err(anyhow::anyhow!("TRAILERFIN_THREADS must be greater than or equal to 1"));
        }

        if config.scan_path.is_empty() {
            return Err(anyhow::anyhow!("TRAILERFIN_SCAN_PATH must be set and cannot be empty"));
        }

        if config.user_agent.trim().is_empty() {
            return Err(anyhow::anyhow!("TRAILERFIN_USER_AGENT must be set and cannot be empty"));
        }

        if config.video_filename.trim().is_empty() {
            return Err(anyhow!("TRAILERFIN_VIDEO_FILENAME must be set and cannot be empty"));
        }

        if config.should_schedule {
            match config.schedule.as_deref().map(str::trim) {
                Some("") | None => {
                    return Err(anyhow!("TRAILERFIN_SCHEDULE must be set and not empty when scheduling is enabled"));
                }
                _ => {}
            }
        }

        if config.data_source == DataSource::Tmdb {
            match config.tmdb_api_key.as_deref().map(str::trim) {
                Some("") | None => {
                    return Err(anyhow!("TRAILERFIN_TMDB_API_KEY must be set and not empty when datasource is set to TMDB"));
                }
                _ => {}
            }
        }

        if config.cache_path.trim().is_empty() {
            return Err(anyhow!("TRAILERFIN_CACHE_PATH must be set and cannot be empty"));
        }

        _ = validate_path(&config.scan_path, "TRAILERFIN_SCAN_PATH")?;
        _ = validate_path(&config.cache_path, "TRAILERFIN_CACHE_PATH")?;

        if config.tv_folders.is_empty() && config.movie_folders.is_empty() {
            return Err(anyhow!("At least one of TRAILERFIN_TV_FOLDERS or TRAILERFIN_MOVIE_FOLDERS must be set and non-empty"));
        }

        for folder in config.tv_folders.iter().chain(config.movie_folders.iter()) {
            let full_path = Path::new(&config.scan_path).join(folder);
            validate_path(full_path.to_str().unwrap(), &format!("subfolder: {}", folder))?;
        }

        info!("Loaded configuration: {:?}", config);

        Ok(Arc::new(config))
    }
}