use std::path::PathBuf;

#[allow(unused_imports)]
use clap::{CommandFactory, Parser, Subcommand};
use dirs::home_dir;
use kubeshim::spawn;
use log::{debug, LevelFilter};
use simple_logger::SimpleLogger;
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

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
	#[clap(short, long, parse(from_occurrences))]
	verbose: usize,

	#[clap(short, long)]
	config: Option<String>,

	#[clap(subcommand)]
	subcommand: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
	/// Run the command
	#[clap(visible_alias = "r")]
	Run {
		#[clap(required = true, multiple_values = true, last = true)]
		commands: Vec<String>,
	},
}

// Should probably be in main but less to import this way and for the time being no one is using as a lib
pub async fn run() -> Result<i32> {
	stable_eyre::install()?;
	let cli = Cli::parse();

	let level = match cli.verbose {
		0 => LevelFilter::Error,
		1 => LevelFilter::Warn,
		2 => LevelFilter::Info,
		3 => LevelFilter::Debug,
		_ => LevelFilter::Trace,
	};

	SimpleLogger::new()
		.with_colors(true)
		.with_level(level)
		.init()
		.unwrap();

	// Set and check for config file
	// Maybe make Enum so I know if it was specified or build config here instead of passing path
	let config_file = match cli.config {
		Some(c) => PathBuf::from(c),
		None => {
			let mut config_file = home_dir().unwrap();
			config_file.push(".config/kubeshim.yaml");

			config_file
		},
	};
	debug!("Using config file: {:?}", &config_file);

	// TODO: should probably check it's readable here
	if !&config_file.is_file() {
		bail!("Config file {} is missing", &config_file.to_str().unwrap())
	}

	let res = match cli.subcommand {
		Commands::Run { commands } => spawn::run_and_proxy(commands, config_file).await?,
	};

	// TODO: this is the start of getting rid of exec_script
	// let cli_command = Cli::command().get_name().to_string();
	// if (cli.subcommand.is_none() && ( cli_command == "kubeshim" || cli_command == "ks")) ||
	// 	(cli.subcommand.is_some() && ( cli_command != "kubeshim" && cli_command != "ks")) {
	// 		Cli::command().error(clap::ErrorKind::DisplayHelp, "Call directly with run or set up as symlink or alias").exit();
	// }

	// if let Some(Commands::Run { commands }) = cli.subcommand {
	// 	res = spawn::run_and_proxy(commands, config_file, false).await?
	// } else {
	// 	res = spawn::run_and_proxy(commands, config_file, true).await?
	// }

	Ok(res)
}

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
