//! This module provides functionality for communicating with the coinbase API.

use async_trait::async_trait;
use client::BaseClient;
use ecbt_exchange::info::*;
use ecbt_exchange::shared::{timestamp_mills, timestamp_to_iso8601_datetime, Result};
use ecbt_exchange::*;
use ecbt_exchange::{
    errors::EcbtError,
    model::{
        AskBid, Balance, CancelAllOrdersRequest, CancelOrderRequest, Candle, EcbtOrderRequest,
        GetHistoricRatesRequest, GetHistoricTradesRequest, GetOrderHistoryRequest, GetOrderRequest,
        GetPriceTickerRequest, Liquidity, OpenMarketOrderRequest, Order, OrderBookRequest,
        OrderBookResponse, OrderCanceled, OrderStatus, OrderType, Paginator, Side, Ticker,
        TimeInForce, Trade, TradeHistoryRequest,
    },
};
use std::convert::TryFrom;
use time::Duration;
use transport::Transport;

pub mod client;
mod coinbase_content_error;
mod coinbase_credentials;
mod coinbase_parameters;
pub mod model;
mod transport;

pub use crate::client::stream::CoinbaseWebsocket;
pub use coinbase_content_error::CoinbaseContentError;
pub use coinbase_credentials::CoinbaseCredentials;
pub use coinbase_parameters::CoinbaseParameters;
use ecbt_exchange::exchange::Environment;
use ecbt_exchange::model::market_pair::MarketPair;
pub use ecbt_exchange::shared;

#[derive(Clone)]
pub struct Coinbase {
    pub exchange_info: ExchangeInfo,
    pub client: BaseClient,
}

#[async_trait]
impl Exchange for Coinbase {
    type InitParams = CoinbaseParameters;
    type InnerClient = BaseClient;

    async fn new(parameters: Self::InitParams) -> Result<Self> {
        let coinbase = match parameters.credentials {
            Some(credentials) => Coinbase {
                exchange_info: ExchangeInfo::new(),
                client: BaseClient {
                    transport: Transport::with_credential(
                        &credentials.api_key,
                        &credentials.api_secret,
                        &credentials.passphrase,
                        parameters.environment == Environment::Sandbox,
                    )?,
                },
            },
            None => Coinbase {
                exchange_info: ExchangeInfo::new(),
                client: BaseClient {
                    transport: Transport::new(parameters.environment == Environment::Sandbox)?,
                },
            },
        };

        coinbase.refresh_market_info().await?;
        Ok(coinbase)
    }

    fn inner_client(&self) -> Option<&Self::InnerClient> {
        Some(&self.client)
    }
}

#[async_trait]
impl ExchangeInfoRetrieval for Coinbase {
    async fn retrieve_pairs(&self) -> Result<Vec<MarketPairInfo>> {
        self.client.products().await.map(|v| {
            v.into_iter()
                .map(|product| MarketPairInfo {
                    symbol: product.id,
                    base: product.base_currency,
                    quote: product.quote_currency,
                    base_increment: product.base_increment,
                    quote_increment: product.quote_increment,
                    min_base_trade_size: None,
                    min_quote_trade_size: None,
                })
                .collect()
        })
    }

    async fn refresh_market_info(&self) -> Result<Vec<MarketPairHandle>> {
        self.exchange_info
            .refresh(self as &dyn ExchangeInfoRetrieval)
            .await
    }

    async fn get_pair(&self, name: &MarketPair) -> Result<MarketPairHandle> {
        let name = crate::model::MarketPair::from(name.clone()).0;
        self.exchange_info.get_pair(&name)
    }
}

#[async_trait]
impl ExchangeMarketData for Coinbase {
    async fn order_book(&self, req: &OrderBookRequest) -> Result<OrderBookResponse> {
        self.client
            .book::<model::BookRecordL2, _>(req.market_pair.clone())
            .await
            .map(Into::into)
    }

    async fn get_price_ticker(&self, req: &GetPriceTickerRequest) -> Result<Ticker> {
        self.client
            .ticker(req.market_pair.clone())
            .await
            .map(Into::into)
    }

    async fn get_historic_rates(&self, req: &GetHistoricRatesRequest) -> Result<Vec<Candle>> {
        let params = model::CandleRequestParams::try_from(req)?;
        self.client
            .candles(req.market_pair.clone(), Some(&params))
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_historic_trades(&self, _req: &GetHistoricTradesRequest) -> Result<Vec<Trade>> {
        unimplemented!("Only implemented for Nash right now");
    }
}

impl From<model::Book<model::BookRecordL2>> for OrderBookResponse {
    fn from(book: model::Book<model::BookRecordL2>) -> Self {
        Self {
            update_id: Some(book.sequence as u64),
            last_update_id: None,
            bids: book.bids.into_iter().map(Into::into).collect(),
            asks: book.asks.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<model::BookRecordL2> for AskBid {
    fn from(bids: model::BookRecordL2) -> Self {
        Self {
            price: bids.price,
            qty: bids.size,
        }
    }
}

impl From<model::Order> for Order {
    fn from(order: model::Order) -> Self {
        let (price, size, order_type) = match order._type {
            model::OrderType::Limit {
                price,
                size,
                time_in_force: _,
            } => (Some(price), size, OrderType::Limit),
            model::OrderType::Market { size, funds: _ } => (None, size, OrderType::Market),
        };

        Self {
            id: order.id,
            market_pair: order.product_id,
            client_order_id: None,
            created_at: Some((timestamp_mills(&order.created_at)) as u64),
            order_type,
            side: order.side.into(),
            status: order.status.into(),
            size,
            price,
            remaining: Some(size - order.filled_size),
            trades: Vec::new(),
        }
    }
}

#[async_trait]
impl ExchangeAccount for Coinbase {
    async fn limit_buy(&self, req: &EcbtOrderRequest) -> Result<Order> {
        let pair = self.get_pair(&req.market_pair).await?.read()?;
        self.client
            .limit_buy(
                pair,
                req.size,
                req.price,
                model::OrderTimeInForce::from(req.time_in_force),
                req.post_only,
            )
            .await
            .map(Into::into)
    }

    async fn limit_sell(&self, req: &EcbtOrderRequest) -> Result<Order> {
        let pair = self.get_pair(&req.market_pair).await?.read()?;
        self.client
            .limit_sell(
                pair,
                req.size,
                req.price,
                model::OrderTimeInForce::from(req.time_in_force),
                req.post_only,
            )
            .await
            .map(Into::into)
    }

    async fn market_buy(&self, req: &OpenMarketOrderRequest) -> Result<Order> {
        let pair = self.get_pair(&req.market_pair).await?.read()?;
        self.client.market_buy(pair, req.size).await.map(Into::into)
    }

    async fn market_sell(&self, req: &OpenMarketOrderRequest) -> Result<Order> {
        let pair = self.get_pair(&req.market_pair).await?.read()?;
        self.client
            .market_sell(pair, req.size)
            .await
            .map(Into::into)
    }

    async fn cancel_order(&self, req: &CancelOrderRequest) -> Result<OrderCanceled> {
        self.client
            .cancel_order(req.id.clone(), req.market_pair.as_deref())
            .await
            .map(Into::into)
    }

    async fn cancel_all_orders(&self, req: &CancelAllOrdersRequest) -> Result<Vec<OrderCanceled>> {
        self.client
            .cancel_all_orders(req.market_pair.clone())
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_all_open_orders(&self) -> Result<Vec<Order>> {
        let params = model::GetOrderRequest {
            status: Some(String::from("open")),
            paginator: None,
            product_id: None,
        };

        self.client
            .get_orders(Some(&params))
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_order_history(&self, req: &GetOrderHistoryRequest) -> Result<Vec<Order>> {
        let req: model::GetOrderRequest = req.into();

        self.client
            .get_orders(Some(&req))
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_trade_history(&self, req: &TradeHistoryRequest) -> Result<Vec<Trade>> {
        let req = req.into();

        self.client
            .get_fills(Some(&req))
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_account_balances(&self, paginator: Option<Paginator>) -> Result<Vec<Balance>> {
        let paginator: Option<model::Paginator> = paginator.map(|p| p.into());

        self.client
            .get_account(paginator.as_ref())
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_order(&self, req: &GetOrderRequest) -> Result<Order> {
        let id = req.id.clone();

        self.client.get_order(id).await.map(Into::into)
    }
}

impl From<model::Account> for Balance {
    fn from(account: model::Account) -> Self {
        Self {
            asset: account.currency,
            free: account.available,
            total: account.balance,
        }
    }
}

impl From<model::Fill> for Trade {
    fn from(fill: model::Fill) -> Self {
        let (buyer_order_id, seller_order_id) = match fill.side.as_str() {
            "buy" => (Some(fill.order_id), None),
            _ => (None, Some(fill.order_id)),
        };

        Self {
            id: fill.trade_id.to_string(),
            buyer_order_id,
            seller_order_id,
            market_pair: fill.product_id,
            price: fill.price,
            qty: fill.size,
            fees: Some(fill.fee),
            side: match fill.side.as_str() {
                "buy" => Side::Buy,
                _ => Side::Sell,
            },
            liquidity: match fill.liquidity.as_str() {
                "M" => Some(Liquidity::Maker),
                "T" => Some(Liquidity::Taker),
                _ => None,
            },
            created_at: fill.created_at.to_string(),
        }
    }
}

impl From<model::Ticker> for Ticker {
    fn from(ticker: model::Ticker) -> Self {
        Self {
            price: Some(ticker.price),
            price_24h: None,
        }
    }
}

impl From<model::Candle> for Candle {
    fn from(candle: model::Candle) -> Self {
        Self {
            time: candle.time * 1000,
            low: candle.low,
            high: candle.high,
            open: candle.open,
            close: candle.close,
            volume: candle.volume,
        }
    }
}

impl TryFrom<&GetHistoricRatesRequest> for model::CandleRequestParams {
    type Error = EcbtError;
    fn try_from(params: &GetHistoricRatesRequest) -> Result<Self> {
        let granularity = u32::try_from(params.interval)?;
        Ok(Self {
            daterange: params.paginator.clone().map(|p| p.into()),
            granularity: Some(granularity),
        })
    }
}

impl From<&GetOrderHistoryRequest> for model::GetOrderRequest {
    fn from(req: &GetOrderHistoryRequest) -> Self {
        Self {
            product_id: req
                .market_pair
                .clone()
                .map(|market| crate::model::MarketPair::from(market).0),
            paginator: req.paginator.clone().map(|p| p.into()),
            status: None,
        }
    }
}

impl From<Paginator> for model::Paginator {
    fn from(paginator: Paginator) -> Self {
        Self {
            after: paginator
                .after
                .map(|s| s.parse::<u64>().expect("Couldn't parse paginator.")),
            before: paginator
                .before
                .map(|s| s.parse::<u64>().expect("Couldn't parse paginator.")),
            limit: paginator.limit,
        }
    }
}

impl From<&Paginator> for model::Paginator {
    fn from(paginator: &Paginator) -> Self {
        Self {
            after: paginator
                .after
                .as_ref()
                .map(|s| s.parse().expect("coinbase page id did not parse as u64")),
            before: paginator
                .before
                .as_ref()
                .map(|s| s.parse().expect("coinbase page id did not parse as u64")),
            limit: paginator.limit,
        }
    }
}

impl From<Paginator> for model::DateRange {
    fn from(paginator: Paginator) -> Self {
        Self {
            start: paginator.start_time.and_then(timestamp_to_iso8601_datetime),
            end: paginator.end_time.and_then(timestamp_to_iso8601_datetime),
        }
    }
}

impl From<&Paginator> for model::DateRange {
    fn from(paginator: &Paginator) -> Self {
        Self {
            start: paginator.start_time.and_then(timestamp_to_iso8601_datetime),
            end: paginator.end_time.and_then(timestamp_to_iso8601_datetime),
        }
    }
}

impl From<TimeInForce> for model::OrderTimeInForce {
    fn from(tif: TimeInForce) -> Self {
        match tif {
            TimeInForce::GoodTillCancelled => model::OrderTimeInForce::GTC,
            TimeInForce::FillOrKill => model::OrderTimeInForce::FOK,
            TimeInForce::ImmediateOrCancelled => model::OrderTimeInForce::IOC,
            TimeInForce::GoodTillTime(duration) => {
                let day: Duration = Duration::days(1);
                let hour: Duration = Duration::hours(1);
                let minute: Duration = Duration::minutes(1);
                if duration == day {
                    model::OrderTimeInForce::GTT {
                        cancel_after: model::CancelAfter::Day,
                    }
                } else if duration == hour {
                    model::OrderTimeInForce::GTT {
                        cancel_after: model::CancelAfter::Hour,
                    }
                } else if duration == minute {
                    model::OrderTimeInForce::GTT {
                        cancel_after: model::CancelAfter::Min,
                    }
                } else {
                    panic!("Coinbase only supports durations of 1 day, 1 hour or 1 minute")
                }
            }
        }
    }
}

impl From<&TradeHistoryRequest> for model::GetFillsReq {
    fn from(req: &TradeHistoryRequest) -> Self {
        Self {
            order_id: req.order_id.clone(),
            paginator: req.paginator.clone().map(|p| p.into()),
            product_id: req
                .market_pair
                .clone()
                .map(|market| crate::model::MarketPair::from(market).0),
        }
    }
}

impl From<model::OrderSide> for Side {
    fn from(req: model::OrderSide) -> Self {
        match req {
            model::OrderSide::Buy => Side::Buy,
            model::OrderSide::Sell => Side::Sell,
        }
    }
}

impl From<model::OrderStatus> for OrderStatus {
    fn from(req: model::OrderStatus) -> OrderStatus {
        match req {
            model::OrderStatus::Active => OrderStatus::Active,
            model::OrderStatus::Done => OrderStatus::Filled,
            model::OrderStatus::Open => OrderStatus::Open,
            model::OrderStatus::Pending => OrderStatus::Pending,
            model::OrderStatus::Rejected => OrderStatus::Rejected,
        }
    }
}
