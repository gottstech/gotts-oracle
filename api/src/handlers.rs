// Copyright 2018 The Grin Developers
// Modifications Copyright 2019 The Gotts Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod server_api;

use self::server_api::{CompactHandler, ExchangeHandler, IndexHandler, RecentHandler};

use crate::rest::*;
use crate::router::{Router, RouterError};
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

use gotts_oracle_alphavantage as alphavantage;
use gotts_oracle_lib::OracleBackend;
use gotts_oracle_util::Mutex;

/// Start all server HTTP handlers. Register all of them with Router
/// and runs the corresponding HTTP server.
///
/// Hyper currently has a bug that prevents clean shutdown. In order
/// to avoid having references kept forever by handlers, we only pass
/// weak references. Note that this likely means a crash if the handlers are
/// used after a server shutdown (which should normally never happen,
/// except during tests).
pub fn start_rest_apis<T: ?Sized>(
	oracle: Arc<Mutex<T>>,
	client: Arc<alphavantage::Client>,
	addr: String,
	tls_config: Option<TLSConfig>,
) -> Result<thread::JoinHandle<()>, Error>
where
	T: OracleBackend + Send + Sync + 'static,
{
	let mut apis = ApiServer::new();
	let router = build_router(oracle, client).expect("unable to build API router");

	info!("Starting HTTP API server at {}.", addr);
	let socket_addr: SocketAddr = addr.parse().expect("unable to parse socket address");
	let res = apis.start(socket_addr, router, tls_config);
	match res {
		Ok(handle) => Ok(handle),
		Err(e) => {
			error!("HTTP API server failed to start. Err: {}", e);
			Err(ErrorKind::Internal("apis failed to start".to_owned()).into())
		}
	}
}

pub fn build_router<T: ?Sized>(
	oracle: Arc<Mutex<T>>,
	client: Arc<alphavantage::Client>,
) -> Result<Router, RouterError>
where
	T: OracleBackend + Send + Sync + 'static,
{
	let route_list = vec![
		"/v1/exchange".to_string(),
		"/v1/recent".to_string(),
		"/v1/compact".to_string(),
	];

	let index_handler = IndexHandler { list: route_list };
	let exchange_handler = ExchangeHandler::new(oracle.clone(), Arc::downgrade(&client));
	let recent_handler = RecentHandler::new(oracle.clone());
	let compact_handler = CompactHandler::new(oracle.clone());

	let mut router = Router::new();

	router.add_route("/v1/", Arc::new(index_handler))?;
	router.add_route("/v1/exchange", Arc::new(exchange_handler))?;
	router.add_route("/v1/recent", Arc::new(recent_handler))?;
	router.add_route("/v1/compact", Arc::new(compact_handler))?;

	Ok(router)
}
