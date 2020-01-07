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

pub mod server_api;
pub mod version_api;

use self::server_api::{
	AggregateHandler, CompactHandler, ExchangeHandler, IndexHandler, RecentHandler,
};

use crate::foreign::Foreign;
use crate::foreign_rpc::ForeignRpc;
use crate::rest::{ApiServer, Error, ErrorKind, TLSConfig};
use crate::router::ResponseFuture;
use crate::router::{Router, RouterError};
use crate::web::*;

use easy_jsonrpc_mw::{Handler, MaybeReply};
use futures::future::ok;
use futures::Future;
use hyper::{Body, Request, Response, StatusCode};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Weak;
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
	let mut router = build_router(oracle.clone(), client.clone())?;
	let json_api_handler_v1 = JsonAPIHandlerV1::new(oracle, Arc::downgrade(&client));
	router.add_route("/v1/json", Arc::new(json_api_handler_v1))?;

	let mut apis = ApiServer::new();

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

type NodeResponseFuture = Box<dyn Future<Item = Response<Body>, Error = Error> + Send>;

/// V1 Json API Handler/Wrapper for foreign functions
pub struct JsonAPIHandlerV1<T: ?Sized>
where
	T: OracleBackend + Send + Sync + 'static,
{
	/// Oracle instance
	pub oracle: Arc<Mutex<T>>,
	/// Vendor API client
	pub client: Weak<gotts_oracle_alphavantage::Client>,
}

impl<T: ?Sized> JsonAPIHandlerV1<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	/// Create a new foreign API handler for GET methods
	pub fn new(oracle: Arc<Mutex<T>>, client: Weak<gotts_oracle_alphavantage::Client>) -> Self {
		JsonAPIHandlerV1 { oracle, client }
	}

	fn call_api(
		&self,
		req: Request<Body>,
		api: Foreign<T>,
	) -> Box<dyn Future<Item = serde_json::Value, Error = Error> + Send> {
		Box::new(parse_body(req).and_then(move |val: serde_json::Value| {
			let foreign_api = &api as &dyn ForeignRpc;
			match foreign_api.handle_request(val) {
				MaybeReply::Reply(r) => ok(r),
				MaybeReply::DontReply => {
					// Since it's http, we need to return something. We return [] because jsonrpc
					// clients will parse it as an empty batch response.
					ok(serde_json::json!([]))
				}
			}
		}))
	}

	fn handle_post_request(&self, req: Request<Body>) -> NodeResponseFuture {
		let api = Foreign::new(self.oracle.clone(), self.client.clone());
		Box::new(
			self.call_api(req, api)
				.and_then(|resp| ok(json_response_pretty(&resp))),
		)
	}
}

impl<T: ?Sized> crate::router::Handler for JsonAPIHandlerV1<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	fn post(&self, req: Request<Body>) -> ResponseFuture {
		Box::new(
			self.handle_post_request(req)
				.and_then(|r| ok(r))
				.or_else(|e| {
					error!("Request Error: {:?}", e);
					ok(create_error_response(e))
				}),
		)
	}

	fn options(&self, _req: Request<Body>) -> ResponseFuture {
		Box::new(ok(create_ok_response("{}")))
	}
}

// pretty-printed version of above
fn json_response_pretty<T>(s: &T) -> Response<Body>
where
	T: Serialize,
{
	match serde_json::to_string_pretty(s) {
		Ok(json) => response(StatusCode::OK, json),
		Err(_) => response(StatusCode::INTERNAL_SERVER_ERROR, ""),
	}
}

fn create_error_response(e: Error) -> Response<Body> {
	Response::builder()
		.status(StatusCode::INTERNAL_SERVER_ERROR)
		.header("access-control-allow-origin", "*")
		.header(
			"access-control-allow-headers",
			"Content-Type, Authorization",
		)
		.body(format!("{}", e).into())
		.unwrap()
}

fn create_ok_response(json: &str) -> Response<Body> {
	Response::builder()
		.status(StatusCode::OK)
		.header("access-control-allow-origin", "*")
		.header(
			"access-control-allow-headers",
			"Content-Type, Authorization",
		)
		.header(hyper::header::CONTENT_TYPE, "application/json")
		.body(json.to_string().into())
		.unwrap()
}

/// Build a new hyper Response with the status code and body provided.
///
/// Whenever the status code is `StatusCode::OK` the text parameter should be
/// valid JSON as the content type header will be set to `application/json'
fn response<T: Into<Body>>(status: StatusCode, text: T) -> Response<Body> {
	let mut builder = &mut Response::builder();

	builder = builder
		.status(status)
		.header("access-control-allow-origin", "*")
		.header(
			"access-control-allow-headers",
			"Content-Type, Authorization",
		);

	if status == StatusCode::OK {
		builder = builder.header(hyper::header::CONTENT_TYPE, "application/json");
	}

	builder.body(text.into()).unwrap()
}

pub fn build_router<T: ?Sized>(
	oracle: Arc<Mutex<T>>,
	client: Arc<alphavantage::Client>,
) -> Result<Router, RouterError>
where
	T: OracleBackend + Send + Sync + 'static,
{
	let route_list = vec![
		"/v1/rest/exchange".to_string(),
		"/v1/rest/recent".to_string(),
		"/v1/rest/compact".to_string(),
		"/v1/rest/aggregated".to_string(),
	];

	let index_handler = IndexHandler { list: route_list };
	let exchange_handler = ExchangeHandler::new(oracle.clone(), Arc::downgrade(&client));
	let recent_handler = RecentHandler::new(oracle.clone());
	let compact_handler = CompactHandler::new(oracle.clone());
	let aggregated_handler = AggregateHandler::new(oracle.clone());

	let mut router = Router::new();

	router.add_route("/v1/rest", Arc::new(index_handler))?;
	router.add_route("/v1/rest/exchange", Arc::new(exchange_handler))?;
	router.add_route("/v1/rest/recent", Arc::new(recent_handler))?;
	router.add_route("/v1/rest/compact", Arc::new(compact_handler))?;
	router.add_route("/v1/rest/aggregated", Arc::new(aggregated_handler))?;

	Ok(router)
}
