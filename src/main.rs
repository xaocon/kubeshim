use std::env;
use std::path::PathBuf;
use std::fmt;

use kubeshim::spawn;
use log::{debug, trace};
use stable_eyre::eyre::{bail, Result};

#[tokio::main]
async fn main() {
	std::process::exit(match run().await {
		Ok(res) => res,
		Err(err) => {
			eprintln!("{:#?}", err);
			222
		},
	});
}

fn get_called_as() -> String {
	let mut args: Vec<String> = env::args().collect();
	trace!("args: {args:?}");
	let called_as = PathBuf::from(args.remove(0));
	let c = called_as.clone();
	let name = c.file_name().unwrap().to_string_lossy(); // If this doesn't work we should panic
	debug!("called as {name}");
	name
}

#[derive(Debug)]
struct RunErrorr {
	msg: String,
}

impl fmt::Display for RunErrorr {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{HELP}\n{self.msg}")
    }
}

// Should probably be in main but less to import this way and for the time being no one is using as a lib
pub async fn run() -> Result<i32> {
	stable_eyre::install()?;
	env_logger::Builder::from_env("KS_DEBUG").init();

	let base_dir = env::var("KS_DIR");

	// first test how it's called
	let called_as = get_called_as();

	if called_as == "kubeshim" || called_as == "ks" {
		return Err(RunErrorr{msg: "called as bin"});
	}

	let base_dir = PathBuf::from(base_dir.unwrap());

	// Set and check for config file
	// Maybe make Enum so I know if it was specified or build config here instead of passing path
	let mut config_file = base_dir.clone();
	config_file.push("kubeshim.yaml");
	debug!("Using config file: {:?}", &config_file);

	// TODO: should probably check it's readable here
	if ! &config_file.is_file() {
		return Err(RunErrorr{msg: "no config file"});
	}

	args.insert(0, name.to_string());

	spawn::run_and_proxy(args, config_file).await
}

fn print_help(extra: &str) -> Result<()> {
	println!("{HELP}");
	bail!("{}", extra);
}

// TODO: add more here
const HELP: &str = r#"
Set up a directory to hold this bin and symlinks to it with the names of commands you want to call it as.
Set env variable KS_DIS to the directory and make sure it's early in your PATH.
Example:
echo 'export KS_DIR="$HOME/.ks"' | tee -a $HOME/.zshenv | source /dev/stdin
echo 'export PATH="$KS_DIR:$PATH"' | tee -a $HOME/.zshrc | source /dev/stdin
mkdir $KS_DIR
cp kubeshim $KS_DIR/
ln -s kubeshim $KS_DIR/kubectl
hash -r
"#;

#[cfg(test)]
mod tests {
	#![allow(unused)]

	use super::*;

	#[should_panic]
	#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
	pub async fn basic_test() {
		main();
	}
}
