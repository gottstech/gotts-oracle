// Copyright 2019 The Gotts Developers
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

#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
use clap::{App, ArgMatches};
use config::{GlobalConfig, ServerConfig};
use gotts_oracle_alphavantage as alphavantage;
use gotts_oracle_api as api;
use gotts_oracle_config as config;
use gotts_oracle_lib::{Error, LMDBBackend, OracleInst};
use gotts_oracle_util::init_logger;
use gotts_oracle_util::Mutex;

use colored::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn info_strings() -> (String, String) {
	(
		format!(
			"This is Gotts-Oracle version {}{}, built for {} by {}.",
			built_info::PKG_VERSION,
			built_info::GIT_VERSION.map_or_else(|| "".to_owned(), |v| format!(" (git {})", v)),
			built_info::TARGET,
			built_info::RUSTC_VERSION,
		)
		.to_string(),
		format!(
			"Built with profile \"{}\", features \"{}\".",
			built_info::PROFILE,
			built_info::FEATURES_STR,
		)
		.to_string(),
	)
}

fn log_build_info() {
	let (basic_info, detailed_info) = info_strings();
	warn!("{}", basic_info);
	debug!("{}", detailed_info);
}

fn main() {
	let exit_code = real_main();
	std::process::exit(exit_code);
}

fn real_main() -> i32 {
	let yml = load_yaml!("gotts_oracle.yml");
	let args = App::from_yaml(yml)
		.version(built_info::PKG_VERSION)
		.get_matches();

	// Deal with configuration file creation
	match args.subcommand() {
		("server", Some(server_args)) => {
			// If it's just a server config command, do it and exit
			if let ("config", Some(_)) = server_args.subcommand() {
				config_command_server(config::config::SERVER_CONFIG_FILE_NAME);
				return 0;
			}
		}
		_ => {}
	}

	let oracle_config = Some(config::initial_setup_server().unwrap_or_else(|e| {
		panic!("Error loading server configuration: {}", e);
	}));

	if let Some(mut config) = oracle_config.clone() {
		let l = config.members.as_mut().unwrap().logging.clone().unwrap();
		init_logger(Some(l));

		if let Some(file_path) = &config.config_file_path {
			warn!(
				"Using configuration file at {}",
				file_path.to_str().unwrap()
			);
		} else {
			warn!("Node configuration file not found, using default");
		}
	}

	log_build_info();

	// Execute subcommand
	match args.subcommand() {
		// server commands and options
		("server", Some(server_args)) => server_command(Some(server_args), oracle_config.unwrap()),

		// clean command
		("clean", _) => {
			let db_root_path = oracle_config.unwrap().members.unwrap().server.db_root;
			println!("Cleaning oracle data directory: {}", db_root_path);
			match std::fs::remove_dir_all(db_root_path) {
				Ok(_) => 0,
				Err(_) => 1,
			}
		}

		// If nothing is specified, try to just use the config file instead
		// this could possibly become the way to configure most things
		// with most command line options being phased out
		_ => server_command(None, oracle_config.unwrap()),
	}
}

/// Create a config file in the current directory
fn config_command_server(file_name: &str) {
	let mut default_config = GlobalConfig::default();
	let current_dir = std::env::current_dir().unwrap_or_else(|e| {
		panic!("Error creating config file: {}", e);
	});
	let mut config_file_name = current_dir.clone();
	config_file_name.push(file_name);
	if config_file_name.exists() {
		panic!(
			"{} already exists in the current directory. Please remove it first",
			file_name
		);
	}
	default_config.update_paths(&current_dir);
	default_config
		.write_to_file(config_file_name.to_str().unwrap())
		.unwrap_or_else(|e| {
			panic!("Error creating config file: {}", e);
		});

	println!(
		"{} file configured and created in current directory",
		file_name
	);
}

/// Handles the server part of the command line, mostly running, starting and
/// stopping the Gotts Oracle server. Processes all the command line
/// arguments to build a proper configuration and runs Gotts Oracle with that
/// configuration.
fn server_command(server_args: Option<&ArgMatches<'_>>, global_config: GlobalConfig) -> i32 {
	// just get defaults from the global config
	let server_config = global_config.members.as_ref().unwrap().server.clone();

	if let Some(a) = server_args {
		match a.subcommand() {
			("run", _) => {
				start_server(server_config);
			}
			("", _) => {
				println!("Subcommand required, use 'gotts_oracle help server' for details");
			}
			(cmd, _) => {
				println!(":: {:?}", server_args);
				panic!(
					"Unknown server command '{}', use 'gotts_oracle help server' for details",
					cmd
				);
			}
		}
	} else {
		start_server(server_config);
	}
	0
}

fn start_server(config: ServerConfig) {
	let oracle =
		instantiate_oracle(config.clone(), "alpha_vantage").expect("instantiate_oracle failed");

	//the api key integrated here is just for demo, with very limited access,
	// please claim your own api key and set it as an environment variable before running.
	// the free api key can be requested here: https://www.alphavantage.co/support/#api-key
	//
	let default_alpha_vantage_api_key = "2BY6TAJHCM9Z7HQT";
	let alpha_vantage_api_key = config
		.alpha_vantage_api_key
		.unwrap_or_else(|| default_alpha_vantage_api_key.to_string());
	if alpha_vantage_api_key == default_alpha_vantage_api_key {
		println!(
			"\n{} the default api key hardcoded is just for demo with very limited access.\
			 \nplease claim your own api key from https://www.alphavantage.co/support/#api-key\
			 \nand then set it into the config: gotts-oracle.toml.",
			"warning!".to_string().bright_red(),
		);
	}

	//create alphavantage client
	let shared_client = Arc::new(alphavantage::Client::new(alpha_vantage_api_key.as_str()));

	//start api server
	let oracle_bind_address = config.api_http_addr.clone();
	println!(
		"\ngotts oracle is serving on {}",
		oracle_bind_address.bright_green()
	);
	let res = api::start_rest_apis(oracle, shared_client.clone(), oracle_bind_address, None);

	if let Ok(handle) = res {
		handle.join().expect("The thread being joined has panicked");
	}

	// Just kill process for now, otherwise the process
	// hangs around until sigint because the API server
	// currently has no shutdown facility
	warn!("Shutting down...");
	thread::sleep(Duration::from_millis(1000));
	warn!("Shutdown complete.");
}

/// Helper to create an instance of the LMDB oracle
pub fn instantiate_oracle(
	oracle_config: ServerConfig,
	account: &str,
) -> Result<Arc<Mutex<dyn OracleInst>>, Error> {
	let db_oracle = LMDBBackend::new(oracle_config.clone())?;
	info!("An Oracle instance instantiated for {}", account);
	Ok(Arc::new(Mutex::new(db_oracle)))
}
