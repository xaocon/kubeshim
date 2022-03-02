use std::fs;
use std::path::Path;

use log::trace;
use serde::{Deserialize, Serialize};
use stable_eyre::eyre::Result;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Config {
	pub proxy: Vec<String>,
	pub no_proxy: String,
	pub contexts: Vec<Context>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Context {
	pub name: String,
	pub address: Option<String>,
}

impl Config {
	pub fn new<S: AsRef<Path> + ?Sized>(config_file: &S) -> Result<Self> {
		let file = fs::File::open(&config_file)?;
		let reader = std::io::BufReader::new(file);
		let config: Self = serde_yaml::from_reader(reader)?;

		Ok(config)
	}

	pub fn from(config_str: &str) -> Result<Self> {
		// TODO: give better help with errors here
		trace!("file contents:\n{}\n\n", &config_str);
		let config: Config = serde_yaml::from_str(config_str)?;
		Ok(config)
	}
}

#[cfg(test)]
mod tests {
	#![allow(unused)]

	use super::*;
	use crate::macros::*;

	impl Context {
		fn new_test(name: &str, sock: &str) -> Self {
			let address = match sock {
				"" => None,
				s => Some(s.to_owned()),
			};

			Self {
				name: name.to_owned(),
				address,
			}
		}
	}

	#[test]
	pub fn basic_test() {
		let file = Config::new("kubeshim.yaml.example");
		assert!(file.is_ok());
	}

	#[test]
	pub fn real_test() {
		let contexts = [
			("hq1", ""),
			("hq2", "127.0.0.1:12341"),
			("au-prod", "127.0.0.1:12342"),
			("us-staging", "127.0.0.1:12343"),
			(r"external-\D{2}-(staging|prod)", "8.8.8.8:11111"),
			(".*", "127.0.0.1:55555"),
		]
		.iter()
		.map(|(name, address)| Context::new_test(name, address))
		.collect();

		let mut config = Config {
			proxy: vec_of_strings!["10.0.0.0/8"],
			no_proxy: ".internal,googleapis.com".to_string(),
			contexts,
		};

		let file = Config::new("kubeshim.yaml.example").unwrap();
		assert_eq!(file, config);

		let first = config.contexts.remove(0);
		config.contexts.push(first);
		assert_ne!(file, config);
	}
}
