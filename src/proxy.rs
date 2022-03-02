use std::convert::Infallible;
use std::net::{SocketAddr, TcpListener};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, Server};
use log::{debug, trace};
use stable_eyre::eyre::{bail, eyre, Result};
use tokio::net::tcp::{ReadHalf, WriteHalf};
use tokio::net::{lookup_host, TcpStream};
use tokio::sync::oneshot::Sender;
use tokio::try_join;

use crate::state::State;

pub type HttpClient = Client<hyper::client::HttpConnector>;

pub async fn create_proxy(env: &'static State, tx_port: Sender<u16>) -> Result<()> {
	let client = HttpClient::new();

	let make_service = make_service_fn(move |_| {
		let client = client.clone();
		async move { Ok::<_, Infallible>(service_fn(move |req| proxy(env, client.clone(), req))) }
	});

	let listener = TcpListener::bind("127.0.0.1:0")?;
	let port = listener.local_addr()?.port();
	debug!("Listening on http://127.0.0.1:{}", port);

	if tx_port.send(port).is_err() {
		bail!("Async tasks failed to comm");
	}

	let server = Server::from_tcp(listener)?.serve(make_service);

	if let Err(e) = server.await {
		bail!("server error: {}", e);
	}

	Ok(())
}

#[derive(Clone, Copy)]
pub struct Socks5Proxy(pub SocketAddr);

impl Socks5Proxy {
	pub async fn connect(&self, auth: &str, req: Request<Body>) {
		match tokio_socks::tcp::Socks5Stream::connect(self.0, &*auth).await {
			Ok(mut stream) => tunnel(auth, req, stream.split()).await,
			Err(e) => debug!("{}: socks5 connect error: {:?}", auth, e),
		}
	}
}

async fn proxy(
	config: &'static State,
	client: HttpClient,
	req: Request<Body>,
) -> Result<Response<Body>> {
	trace!("req: {:?}", req);

	if Method::CONNECT == req.method() {
		// Received an HTTP request like:
		// ```
		// CONNECT www.domain.com:443 HTTP/1.1
		// Host: www.domain.com:443
		// Proxy-Connection: Keep-Alive
		// ```
		//
		// When HTTP method is CONNECT we should return an empty body
		// then we can eventually upgrade the connection and talk a new protocol.
		//
		// Note: only after client received an empty body with STATUS_OK can the
		// connection be upgraded, so we can't return a response inside
		// `on_upgrade` future.
		let uri = req.uri();
		let auth = uri
			.authority()
			.ok_or_else(|| eyre!("CONNECT host is not socket addr: {:?}", uri))?;
		let host = auth.host().as_bytes();
		let auth = auth.to_string();
		let to = Socks5Proxy(config.setup.socks_address.unwrap());

		if config.setup.proxy.contain_host(host) {
			debug!("PROXY {}", auth);
			tokio::task::spawn(async move { to.connect(&auth, req).await });
		} else {
			tokio::task::spawn(async move {
				match lookup_host(&auth).await {
					Err(e) => debug!("lookup host {} error: {:?}.", auth, e),
					Ok(mut host) => {
						match host.next() {
							None => debug!("lookup host {} error: no ip found.", auth),
							Some(ip) => {
								// TODO: Should be cached
								if config.setup.proxy.contain_ip(&ip.ip()) {
									debug!("PROXY {}", auth);
									to.connect(&auth, req).await;
								} else {
									debug!("DIRECT {}", auth);
									match TcpStream::connect(&*auth).await {
										Ok(mut stream) => tunnel(&auth, req, stream.split()).await,
										Err(e) => {
											debug!("{}: direct connect error: {:?}", auth, e)
										},
									}
								}
							},
						}
					},
				}
			});
		};

		Ok(Response::new(Body::empty()))
	} else {
		Ok(client.request(req).await?)
	}
}

// Create a TCP connection to host:port, build a tunnel between the connection and
// the upgraded connection
async fn tunnel(
	auth: &str,
	req: Request<Body>,
	(mut server_rd, mut server_wr): (ReadHalf<'_>, WriteHalf<'_>),
) {
	match hyper::upgrade::on(req).await {
		Ok(upgraded) => {
			// Proxying data
			let amounts = {
				let (mut client_rd, mut client_wr) = tokio::io::split(upgraded);

				let client_to_server = tokio::io::copy(&mut client_rd, &mut server_wr);
				let server_to_client = tokio::io::copy(&mut server_rd, &mut client_wr);

				try_join!(client_to_server, server_to_client)
			};

			// Print message when done
			match amounts {
				Ok((from_client, from_server)) => {
					debug!(
						"{}: client wrote {} bytes and received {} bytes",
						auth, from_client, from_server
					);
				},
				Err(e) => {
					debug!("{}: tunnel error: {}", auth, e);
				},
			};
		},
		Err(e) => debug!("{}: upgrade error: {}", auth, e),
	}
}
