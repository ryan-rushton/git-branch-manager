use std::path::PathBuf;

use color_eyre::eyre::Result;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct AppConfig {
  #[serde(default)]
  pub _data_dir: PathBuf,
  #[serde(default)]
  pub _config_dir: PathBuf,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
  #[serde(default, flatten)]
  pub config: AppConfig,
}

impl Config {
  pub fn new() -> Result<Self, config::ConfigError> {
    let data_dir = crate::utils::get_data_dir();
    let config_dir = crate::utils::get_config_dir();
    let mut builder = config::Config::builder()
      .set_default("_data_dir", data_dir.to_str().unwrap())?
      .set_default("_config_dir", config_dir.to_str().unwrap())?;

    let config_files = [
      ("config.json5", config::FileFormat::Json5),
      ("config.json", config::FileFormat::Json),
      ("config.yaml", config::FileFormat::Yaml),
      ("config.toml", config::FileFormat::Toml),
      ("config.ini", config::FileFormat::Ini),
    ];
    let mut found_config = false;
    for (file, format) in &config_files {
      builder = builder.add_source(config::File::from(config_dir.join(file)).format(*format).required(false));
      if config_dir.join(file).exists() {
        found_config = true
      }
    }
    if !found_config {
      log::error!("No configuration file found. Application may not behave as expected");
    }

    let cfg: Self = builder.build()?.try_deserialize()?;

    Ok(cfg)
  }
}
