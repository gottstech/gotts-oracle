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

//! Configuration file management

use dirs;
use rand::distributions::{Alphanumeric, Distribution};
use rand::thread_rng;
use std::env;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Read;
use std::path::PathBuf;
use toml;

use crate::comments::insert_comments;
use crate::types::{ConfigError, ConfigMembers, GlobalConfig, ServerConfig};
use crate::util::LoggingConfig;

/// The default file name to use when trying to derive
/// the oracle config file location
pub const SERVER_CONFIG_FILE_NAME: &'static str = "gotts-oracle.toml";
const SERVER_LOG_FILE_NAME: &'static str = "gotts-oracle.log";
const GOTTS_HOME: &'static str = ".gotts";
const GOTTS_CHAIN_DIR: &'static str = "oracle_data";
/// Node API secret
pub const API_SECRET_FILE_NAME: &'static str = ".oracle_api_secret";

fn get_gotts_path() -> Result<PathBuf, ConfigError> {
	// Check if gotts dir exists
	let mut gotts_path = match dirs::home_dir() {
		Some(p) => p,
		None => PathBuf::new(),
	};
	gotts_path.push(GOTTS_HOME);
	// Create if the default path doesn't exist
	if !gotts_path.exists() {
		fs::create_dir_all(gotts_path.clone())?;
	}
	Ok(gotts_path)
}

fn check_config_current_dir(path: &str) -> Option<PathBuf> {
	let p = env::current_dir();
	let mut c = match p {
		Ok(c) => c,
		Err(_) => {
			return None;
		}
	};
	c.push(path);
	if c.exists() {
		return Some(c);
	}
	None
}

/// Create file with api secret
fn init_api_secret(api_secret_path: &PathBuf) -> Result<(), ConfigError> {
	let mut api_secret_file = File::create(api_secret_path)?;
	let api_secret: String = Alphanumeric
		.sample_iter(&mut thread_rng())
		.take(20)
		.collect();
	api_secret_file.write_all(api_secret.as_bytes())?;
	Ok(())
}

/// Check if file contains a secret and nothing else
fn check_api_secret(api_secret_path: &PathBuf) -> Result<(), ConfigError> {
	let api_secret_file = File::open(api_secret_path)?;
	let buf_reader = BufReader::new(api_secret_file);
	let mut lines_iter = buf_reader.lines();
	let first_line = lines_iter.next();
	if first_line.is_none() || first_line.unwrap().is_err() {
		fs::remove_file(api_secret_path)?;
		init_api_secret(api_secret_path)?;
	}
	Ok(())
}

/// Check that the api secret file exists and is valid
fn check_api_secret_file() -> Result<(), ConfigError> {
	let gotts_path = get_gotts_path()?;
	let mut api_secret_path = gotts_path.clone();
	api_secret_path.push(API_SECRET_FILE_NAME);
	if !api_secret_path.exists() {
		init_api_secret(&api_secret_path)
	} else {
		check_api_secret(&api_secret_path)
	}
}

/// Handles setup and detection of paths for node
pub fn initial_setup_server() -> Result<GlobalConfig, ConfigError> {
	check_api_secret_file()?;
	// Use config file if current directory if it exists, .gotts home otherwise
	if let Some(p) = check_config_current_dir(SERVER_CONFIG_FILE_NAME) {
		GlobalConfig::new(p.to_str().unwrap())
	} else {
		// Check if gotts dir exists
		let gotts_path = get_gotts_path()?;

		// Get path to default config file
		let mut config_path = gotts_path.clone();
		config_path.push(SERVER_CONFIG_FILE_NAME);

		// Spit it out if it doesn't exist
		if !config_path.exists() {
			let mut default_config = GlobalConfig::default();
			// update paths relative to current dir
			default_config.update_paths(&gotts_path);
			default_config.write_to_file(config_path.to_str().unwrap())?;
		}

		GlobalConfig::new(config_path.to_str().unwrap())
	}
}

/// Returns the defaults, as strewn throughout the code
impl Default for ConfigMembers {
	fn default() -> ConfigMembers {
		ConfigMembers {
			server: ServerConfig::default(),
			logging: Some(LoggingConfig::default()),
		}
	}
}

impl Default for GlobalConfig {
	fn default() -> GlobalConfig {
		GlobalConfig {
			config_file_path: None,
			members: Some(ConfigMembers::default()),
		}
	}
}

impl GlobalConfig {
	/// Requires the path to a config file
	pub fn new(file_path: &str) -> Result<GlobalConfig, ConfigError> {
		let mut return_value = GlobalConfig::default();
		return_value.config_file_path = Some(PathBuf::from(&file_path));

		// Config file path is given but not valid
		let config_file = return_value.config_file_path.clone().unwrap();
		if !config_file.exists() {
			return Err(ConfigError::FileNotFoundError(String::from(
				config_file.to_str().unwrap(),
			)));
		}

		// Try to parse the config file if it exists, explode if it does exist but
		// something's wrong with it
		return_value.read_config()
	}

	/// Read config
	fn read_config(mut self) -> Result<GlobalConfig, ConfigError> {
		let mut file = File::open(self.config_file_path.as_mut().unwrap())?;
		let mut contents = String::new();
		file.read_to_string(&mut contents)?;
		let decoded: Result<ConfigMembers, toml::de::Error> = toml::from_str(&contents);
		match decoded {
			Ok(gc) => {
				self.members = Some(gc);
				return Ok(self);
			}
			Err(e) => {
				return Err(ConfigError::ParseError(
					String::from(
						self.config_file_path
							.as_mut()
							.unwrap()
							.to_str()
							.unwrap()
							.clone(),
					),
					String::from(format!("{}", e)),
				));
			}
		}
	}

	/// Update paths
	pub fn update_paths(&mut self, gotts_home: &PathBuf) {
		// need to update server chain path
		let mut chain_path = gotts_home.clone();
		chain_path.push(GOTTS_CHAIN_DIR);
		self.members.as_mut().unwrap().server.db_root = chain_path.to_str().unwrap().to_owned();
		let mut secret_path = gotts_home.clone();
		secret_path.push(API_SECRET_FILE_NAME);
		self.members.as_mut().unwrap().server.api_secret_path =
			Some(secret_path.to_str().unwrap().to_owned());
		let mut log_path = gotts_home.clone();
		log_path.push(SERVER_LOG_FILE_NAME);
		self.members
			.as_mut()
			.unwrap()
			.logging
			.as_mut()
			.unwrap()
			.log_file_path = log_path.to_str().unwrap().to_owned();
	}

	/// Serialize config
	pub fn ser_config(&mut self) -> Result<String, ConfigError> {
		let encoded: Result<String, toml::ser::Error> =
			toml::to_string(self.members.as_mut().unwrap());
		match encoded {
			Ok(enc) => return Ok(enc),
			Err(e) => {
				return Err(ConfigError::SerializationError(String::from(format!(
					"{}",
					e
				))));
			}
		}
	}

	/// Write configuration to a file
	pub fn write_to_file(&mut self, name: &str) -> Result<(), ConfigError> {
		let conf_out = self.ser_config()?;
		let conf_out = insert_comments(conf_out);
		let mut file = File::create(name)?;
		file.write_all(conf_out.as_bytes())?;
		Ok(())
	}
}