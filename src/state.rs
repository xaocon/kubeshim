use std::borrow::Borrow;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use iprange::IpRange;
use regex::Regex;
use stable_eyre::eyre::{eyre, Result};

use crate::config::Config;
use crate::kubeconfig::KubeConfig;

pub const NUM_ALPHABET: usize = 26;
pub const NUM_DIGIT: usize = 10;
pub const NUM_SPECIAL: usize = 2;
pub const NUM_CHILDREN: usize = NUM_ALPHABET + NUM_DIGIT + NUM_SPECIAL;

pub const MATCHED: usize = usize::MAX;
pub const NOT_MATCHED: usize = usize::MAX - 1;

pub struct State {
	pub setup: Setup,
}

pub struct Setup {
	pub socks_address: Option<SocketAddr>,
	pub proxy: Pattern,
	pub no_proxy: String,
	pub config_path: PathBuf,
}

impl State {
	pub fn new(config_file: &Path) -> Result<Self> {
		let file_config = Config::new(&config_file)?;

		let proxy_list = file_config.proxy.iter().map(|x| x.as_str()).collect();

		let config = KubeConfig::new()?;
		let current_context = config.current_context;

		let mut socks_address: Option<SocketAddr> = None;
		for context in file_config.contexts.iter() {
			let patt = Regex::new(&context.name)?;
			if patt.is_match(&current_context) {
				if let Some(addr) = &context.address {
					socks_address = Some(SocketAddr::from_str(addr)?)
				}
				break;
			}
		}

		let setup = Setup {
			socks_address,
			proxy: Pattern::from_strs(proxy_list).expect("proxy config error"),
			no_proxy: file_config.no_proxy,
			config_path: config_file.to_owned(),
		};

		Ok(Self { setup })
	}
}

pub struct Pattern {
	// usize::MAX -> matched
	// usize::MAX-1 -> Not matched
	// other -> index of first child, should be <= self.0.len() - NUM_CHILDREN
	// should not be empty
	host_trie: Vec<usize>,
	ipv4: IpRange<Ipv4Net>,
	ipv6: IpRange<Ipv6Net>,
}

impl Default for Pattern {
	fn default() -> Self {
		Pattern {
			host_trie: vec![NOT_MATCHED],
			ipv4: Default::default(),
			ipv6: Default::default(),
		}
	}
}

impl Pattern {
	fn codec(ch: u8) -> Result<usize> {
		if (b'A'..=b'Z').contains(&ch) {
			Ok((ch - b'A') as usize)
		} else if (b'a'..=b'z').contains(&ch) {
			Ok((ch - b'a') as usize)
		} else if (b'0'..=b'9').contains(&ch) {
			Ok((ch - b'0') as usize + NUM_ALPHABET)
		} else if ch == b'.' {
			Ok(NUM_ALPHABET + NUM_DIGIT)
		} else if ch == b'-' {
			Ok(NUM_ALPHABET + NUM_DIGIT + 1)
		} else {
			Err(eyre!("Unexpected char: {}", ch))
		}
	}

	fn add_host(&mut self, suffix: &[u8]) -> Result<()> {
		let mut current = 0;
		for &b in suffix.iter().rev() {
			let child = match self.host_trie[current] {
				MATCHED => return Ok(()),
				NOT_MATCHED => {
					let child = self.host_trie.len();
					self.host_trie
						.extend(std::iter::repeat(NOT_MATCHED).take(NUM_CHILDREN));
					self.host_trie[current] = child;
					child
				},
				child => child,
			};
			current = child + Self::codec(b)?;
		}
		self.host_trie[current] = MATCHED;
		Ok(())
	}

	pub fn add(&mut self, host_or_ip: &str) -> Result<()> {
		if let Ok(ipnet) = host_or_ip.parse::<IpNet>() {
			match ipnet {
				IpNet::V4(ipnet) => {
					self.ipv4.add(ipnet);
				},
				IpNet::V6(ipnet) => {
					self.ipv6.add(ipnet);
				},
			}
			Ok(())
		} else {
			self.add_host(host_or_ip.as_bytes())
		}
	}

	pub fn build(&mut self) {
		self.host_trie.shrink_to_fit();
		self.ipv4.simplify();
		self.ipv6.simplify();
	}

	pub fn contain_host(&self, uri: &[u8]) -> bool {
		let mut current = 0usize;
		for &b in uri.iter().rev() {
			match self.host_trie[current] {
				MATCHED => return true,
				NOT_MATCHED => return false,
				child => {
					match Self::codec(b) {
						Ok(n) => current = child + n,
						Err(_) => return false,
					}
				},
			}
		}
		self.host_trie[current] == MATCHED
	}

	pub fn contain_ip(&self, ip: &IpAddr) -> bool {
		match ip {
			IpAddr::V4(ip) => self.ipv4.contains(ip),
			IpAddr::V6(ip) => self.ipv6.contains(ip),
		}
	}

	pub fn from_strs(iter: Vec<&str>) -> Result<Pattern> {
		let mut pattern = Pattern::default();

		for s in iter.iter() {
			pattern.add(s.borrow())?
		}
		pattern.build();

		Ok(pattern)
	}
}

#[cfg(test)]
mod tests {
	use std::env;

	use super::*;

	#[test]
	fn check_external_pattern_match() {
		env::set_var("KUBECONFIG", "./tests/static/kubeconfig");
		let kubeshim_config_path = PathBuf::from("./kubeshim.yaml.example");
		let state = State::new(&kubeshim_config_path);
		let state = state.unwrap();

		assert_eq!(
			state.setup.socks_address,
			Some(SocketAddr::from_str("8.8.8.8:11111").unwrap())
		)
	}
}
