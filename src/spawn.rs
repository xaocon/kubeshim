use std::env;
use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use log::{debug, error, info, trace};
use path_absolutize::*;
use stable_eyre::eyre::{bail, eyre, Result};
use tokio::signal::ctrl_c;
use tokio::sync::oneshot::{channel, Receiver};

use crate::proxy;

// TODO: could make zombies, need to cleanup if command doesn't finish before this run
// https://docs.rs/tokio/1.5.0/tokio/process/struct.Command.html#method.kill_on_drop

// Should probably clean this up some, Don't like two routes to spawn(), also would be nice if cmd
//   was run in shell for things like alias. Some shell completion would be nice too but probably
//   not here.

#[allow(unused)]
fn new_path() -> Result<String> {
	let called_as = env::args()
		.next()
		.ok_or_else(|| eyre!("Problem getting command called as"))?;
	let full_path = Path::new(&called_as).absolutize()?;
	let calling_dir = full_path
		.parent()
		.ok_or_else(|| eyre!("Problem getting calling directory"))?;

	debug!("Found directory {:?} as callers pwd", &calling_dir);

	let env_path = env::var("PATH").unwrap();
	let path_vec: Vec<&str> = env_path
		.split(':')
		.filter(|r| Path::new(r) != calling_dir)
		.collect();

	Ok(path_vec.join(":"))
}

pub async fn run_and_proxy(mut cmds: Vec<String>, config_file: PathBuf) -> Result<i32> {
	let cmd = cmds.remove(0);
	let args = cmds;

	let (tx_port, rx_port) = channel();
	let env: &'static _ = Box::leak(Box::new(crate::state::State::new(&config_file)?));

	let res = match &env.setup.socks_address {
		Some(_) => {
			debug!(
				"Found address {} for this context. Trying to use it.",
				&env.setup.socks_address.unwrap()
			);
			tokio::select! {
				res = run_cmd(&cmd, args, rx_port, env.setup.no_proxy.clone()) => res,
				_ = proxy::create_proxy(env, tx_port) => bail!("proxy error"),
				_ = ctrl_c() => Ok(1),
			}
		},
		None => {
			debug!("No proxy address found, running command directly.");
			Ok(tokio::process::Command::new(cmd)
				// .env("PATH", new_path()?)
				.args(args)
				.spawn()?
				.wait()
				.await?
				.code()
				.unwrap_or(127))
		},
	};

	res
}

async fn run_cmd(
	run_cmd: &str,
	args: Vec<String>,
	rx_port: Receiver<u16>,
	no_proxy: String,
) -> Result<i32> {
	let http_proxy_addr = format!("http://127.0.0.1:{}", rx_port.await?);

	// TODO: need to play with ENV variables, some progs won't work without HTTPS proxy setup
	//   which it doesn't use as https proxy but still requires, we shouldn't need the no proxy
	//   stuff becasue of the ability to specify dest that are routed in config
	let mut cmd = tokio::process::Command::new(run_cmd)
		.args(args)
		// .env("PATH", new_path()?)
		.env("HTTP_PROXY", &http_proxy_addr)
		.env("HTTPS_PROXY", &http_proxy_addr)
		.env("http_proxy", &http_proxy_addr)
		// .env("https_proxy", &http_proxy_addr)
		.env("NO_PROXY", &no_proxy)
		.env("no_proxy", &no_proxy)
		.spawn()?;

	let res = cmd.wait().await?.code().unwrap_or(127);

	debug!("exited: {}", res);

	Ok(res)
}
