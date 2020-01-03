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

use self::server_api::ExchangeHandler;
use self::server_api::IndexHandler;

use crate::rest::*;
use crate::router::{Router, RouterError};
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

extern crate gotts_oracle_alphavantage;
use gotts_oracle_alphavantage as alphavantage;

/// Start all server HTTP handlers. Register all of them with Router
/// and runs the corresponding HTTP server.
///
/// Hyper currently has a bug that prevents clean shutdown. In order
/// to avoid having references kept forever by handlers, we only pass
/// weak references. Note that this likely means a crash if the handlers are
/// used after a server shutdown (which should normally never happen,
/// except during tests).
pub fn start_rest_apis(
	client: Arc<alphavantage::Client>,
	addr: String,
	tls_config: Option<TLSConfig>,
) -> Result<thread::JoinHandle<()>, Error> {
	let mut apis = ApiServer::new();
	let router = build_router(client).expect("unable to build API router");

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

pub fn build_router(client: Arc<alphavantage::Client>) -> Result<Router, RouterError> {
	let route_list = vec!["exchange".to_string()];

	let index_handler = IndexHandler { list: route_list };
	let exchange_handler = ExchangeHandler {
		client: Arc::downgrade(&client),
	};

	let mut router = Router::new();

	router.add_route("/", Arc::new(index_handler))?;
	router.add_route("/exchange", Arc::new(exchange_handler))?;

	Ok(router)
}
