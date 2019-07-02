# Gotts-oracle

Gotts-Oracle is a Price Feed Oracle. It will include the following features: 

  * Local REST API interface for price query of all necessary pairs of Foreign Exchange Market.
  * Real time price, and the result is the median value among at least 3 data vendors.
  * And for cryptocurrencies price, real time price is the median value from at least 5 exchanges.

## Status

Gotts-Oracle is still under development. Much is left to be done and contributions are welcome.

For the moment, only one data vendor is provided: https://www.alphavantage.co/. But indeed we need more.

## Contributing

Welcome any contribution.

Find us:

* Chat: [Gitter](https://gitter.im/gotts_community/lobby).
* Twitter for Gotts: [@gottstech](https://twitter.com/gottstech)
* Telegram for Gotts: [t.me/gottstech](https://t.me/gottstech)

## Getting Started

### Build and Run
```Bash
cargo build --release

./target/release/gotts-oracle 
```

### Query

For example, to query the price pair between USD and CNY:
```Javascript
$ curl -0 -XGET http://127.0.0.1:8008/exchange?from=USD\&to=CNY
{
  "from": "USD",
  "to": "CNY",
  "rate": 6.8619,
  "date": "2019-07-02T06:50:20Z"
}
```

### Data Vendor API Key/s

The key integrated in the source is just for demo, with very limited access. For any production environment, please get a commercial key for yourself.

## License

Apache License v2.0.