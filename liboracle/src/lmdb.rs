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

use chrono::{DateTime, Utc};
use std::cell::RefCell;
use std::{fs, path};

use super::error::Error;
use super::oracle_store::{self, option_to_not_found, Store};
use super::types::{ExchangePriceBatch, OracleBackend};
use crate::alphavantage::ExchangeRateResult;
use byteorder::{BigEndian, WriteBytesExt};
use gotts_oracle_config::ServerConfig;

const SEP: u8 = b':';
pub const DB_DIR: &'static str = "db";

const EXCHANGE_RATE_PREFIX: u8 = 'e' as u8;

/// Build a db key from a prefix and a byte vector identifier.
pub fn to_key(prefix: u8, k: &mut Vec<u8>) -> Vec<u8> {
	let mut res = Vec::with_capacity(k.len() + 2);
	res.push(prefix);
	res.push(SEP);
	res.append(k);
	res
}

/// Build a db key from a prefix and a byte vector identifier and numeric identifier
pub fn to_key_i64(prefix: u8, k: &mut Vec<u8>, val: i64) -> Vec<u8> {
	let mut res = Vec::with_capacity(k.len() + 10);
	res.push(prefix);
	res.push(SEP);
	res.append(k);
	res.write_i64::<BigEndian>(val).unwrap();
	res
}

/// test to see if database files exist in the current directory. If so,
/// use a DB backend for all operations
pub fn oracle_db_exists(config: ServerConfig) -> bool {
	let db_path = path::Path::new(&config.db_root).join(DB_DIR);
	db_path.exists()
}

pub struct LMDBBackend {
	db: Store,
	config: ServerConfig,
}

impl LMDBBackend {
	pub fn new(config: ServerConfig) -> Result<Self, Error> {
		let db_path = path::Path::new(&config.db_root).join(DB_DIR);
		fs::create_dir_all(&db_path).expect("Couldn't create Oracle backend directory!");

		let store = Store::new(db_path.to_str().unwrap(), None, Some(DB_DIR), None)?;

		let res = LMDBBackend {
			db: store,
			config: config.clone(),
		};
		Ok(res)
	}

	/// Just test to see if database files exist in the current directory. If
	/// so, use a DB backend for all operations
	pub fn exists(config: ServerConfig) -> bool {
		let db_path = path::Path::new(&config.db_root).join(DB_DIR);
		db_path.exists()
	}
}

impl OracleBackend for LMDBBackend {
	fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = ExchangeRateResult> + 'a> {
		Box::new(self.db.iter(&[EXCHANGE_RATE_PREFIX]).unwrap().map(|o| o.1))
	}

	fn iter_id<'a>(&'a self, id: &str) -> Box<dyn Iterator<Item = ExchangeRateResult> + 'a> {
		let key = to_key(EXCHANGE_RATE_PREFIX, &mut id.as_bytes().to_vec());
		Box::new(self.db.iter(&key).unwrap().map(|o| o.1))
	}

	fn get(&self, id: &str) -> Result<ExchangeRateResult, Error> {
		let key = to_key(EXCHANGE_RATE_PREFIX, &mut id.as_bytes().to_vec());
		option_to_not_found(self.db.get_ser(&key), &format!("Key Id: {}", id)).map_err(|e| e.into())
	}

	fn batch<'a>(&'a mut self) -> Result<Box<dyn ExchangePriceBatch + 'a>, Error> {
		Ok(Box::new(Batch {
			_store: self,
			db: RefCell::new(Some(self.db.batch()?)),
		}))
	}
}

/// An atomic batch in which all changes can be committed all at once or
/// discarded on error.
pub struct Batch<'a> {
	_store: &'a LMDBBackend,
	db: RefCell<Option<oracle_store::Batch<'a>>>,
}

#[allow(missing_docs)]
impl<'a> ExchangePriceBatch for Batch<'a> {
	fn save(
		&mut self,
		date: DateTime<Utc>,
		exchange_rate: ExchangeRateResult,
	) -> Result<(), Error> {
		// Save the exchange rate data to the db.
		{
			let mut fromto = exchange_rate.from.clone();
			fromto.push('2');
			fromto.push_str(&exchange_rate.to);
			let key = to_key_i64(
				EXCHANGE_RATE_PREFIX,
				&mut fromto.as_bytes().to_vec(),
				date.timestamp(),
			);
			self.db
				.borrow()
				.as_ref()
				.unwrap()
				.put_ser(&key, &exchange_rate)?;
		}

		Ok(())
	}

	fn get(&self, id: &str) -> Result<ExchangeRateResult, Error> {
		let key = to_key(EXCHANGE_RATE_PREFIX, &mut id.as_bytes().to_vec());
		option_to_not_found(
			self.db.borrow().as_ref().unwrap().get_ser(&key),
			&format!("Key ID: {}", id),
		)
		.map_err(|e| e.into())
	}

	fn iter(&self) -> Box<dyn Iterator<Item = ExchangeRateResult>> {
		Box::new(
			self.db
				.borrow()
				.as_ref()
				.unwrap()
				.iter(&[EXCHANGE_RATE_PREFIX])
				.unwrap()
				.map(|o| o.1),
		)
	}

	fn iter_id(&self, id: &str) -> Box<dyn Iterator<Item = ExchangeRateResult>> {
		let key = to_key(EXCHANGE_RATE_PREFIX, &mut id.as_bytes().to_vec());
		Box::new(
			self.db
				.borrow()
				.as_ref()
				.unwrap()
				.iter(&key)
				.unwrap()
				.map(|o| o.1),
		)
	}

	fn delete(&mut self, id: &str, date: DateTime<Utc>) -> Result<(), Error> {
		// Delete the exchange rate data.
		{
			let key = to_key_i64(
				EXCHANGE_RATE_PREFIX,
				&mut id.as_bytes().to_vec(),
				date.timestamp(),
			);
			let _ = self.db.borrow().as_ref().unwrap().delete(&key);
		}

		Ok(())
	}

	fn commit(&self) -> Result<(), Error> {
		let db = self.db.replace(None);
		db.unwrap().commit()?;
		Ok(())
	}
}
