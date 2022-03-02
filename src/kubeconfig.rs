use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use dirs::home_dir;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use stable_eyre::eyre::Result;

type KeepStuff = HashMap<String, Value>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KubeConfig {
	#[serde(rename = "current-context")]
	pub current_context: String,
	#[serde(flatten)]
	pub just_keep: KeepStuff,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contexts {
	pub context: KeepStuff,
	pub name: String,
}

impl KubeConfig {
	fn config_location() -> PathBuf {
		// TODO: check if valid
		match env::var("KUBECONFIG").ok() {
			Some(provided_path) => PathBuf::from(&provided_path.split(':').next().unwrap()),
			None => {
				let mut config_file = home_dir().unwrap();
				config_file.push(".kube/config");
				config_file
			},
		}
	}

	pub fn from<S: AsRef<Path> + ?Sized>(location: &S) -> Result<Self> {
		let file = fs::File::open(&location)?;
		let reader = io::BufReader::new(file);
		let config: KubeConfig = serde_yaml::from_reader(reader)?;

		Ok(config)
	}

	pub fn new() -> Result<Self> {
		// TODO: only checks default
		// Panics if file not there

		let config_file = Self::config_location();

		let kc = Self::from(&config_file)?;

		Ok(kc)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn reads() {
		let config = KubeConfig::from("tests/static/kubeconfig");

		assert!(config.is_ok());
	}

	#[test]
	pub fn get_context() {
		let config = KubeConfig::from("tests/static/kubeconfig").unwrap();

		assert_eq!(&config.current_context, "external-us-prod");
	}
}
