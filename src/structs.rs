use chrono;
use serde::{Deserialize, Serialize};
use std::{ borrow::Cow,
           fmt};

use uuid::Uuid;

// use utils::datetime_from_string;
use crate::utils::f64_from_string;
use crate::utils::f64_nan_from_string;
use crate::utils::f64_opt_from_string;
use crate::utils::usize_from_string;
use crate::utils::uuid_opt_from_string;
use crate::errors;

//////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////
// TYPES AND HELPERS
//////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////

pub type DateTime = chrono::DateTime<chrono::Utc>;
pub fn now() -> DateTime {
    chrono::Utc::now()
}

//////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////
// ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE
// ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE
// ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE
// ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE ALPHABETIZE
// It's hella easier to find stuff that way...  Note: it ain't complete, yet...
//////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Account {
    #[serde(deserialize_with = "f64_from_string")]
    pub available: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub balance: f64,
    pub currency: String,
    #[serde(deserialize_with = "f64_from_string")]
    pub hold: f64,
    pub id: Uuid,
    pub profile_id: Uuid,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AccountHistory {
    #[serde(skip_deserializing)]
    pub _type: AccountHistoryType,
    #[serde(deserialize_with = "f64_from_string")]
    pub amount: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub balance: f64,
    #[serde(flatten)]
    pub details: AccountHistoryDetails, // variants are not not clear
    pub created_at: DateTime,
    #[serde(deserialize_with = "usize_from_string")]
    pub id: usize,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub enum AccountHistoryType {
    Fee,
    Match,
    NotSet,
    Rebate,
    Transfer,
}

impl Default for AccountHistoryType {
    fn default() -> Self {
        AccountHistoryType::NotSet
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", content = "details")]
// #[serde(rename_all = "camelCase")]
pub enum AccountHistoryDetails {
    Fee {
        order_id: Uuid,
        product_id: String,
        #[serde(deserialize_with = "usize_from_string")]
        trade_id: usize,
    },
    Match {
        order_id: Uuid,
        product_id: String,
        #[serde(deserialize_with = "usize_from_string")]
        trade_id: usize,
    },
    Rebate {
        order_id: Uuid,
        product_id: String,
        #[serde(deserialize_with = "usize_from_string")]
        trade_id: usize,
    },
    Transfer {
        transfer_id: Uuid,
        transfer_type: AccountHistoryDetailsTransferType,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
// #[serde(rename_all = "camelCase")]
pub enum AccountHistoryDetailsTransferType {
    Deposit,
    Withdraw,
}

impl<'a> From<&'a AccountHistoryDetails> for AccountHistoryType {
    fn from(item: &'a AccountHistoryDetails) -> Self {
        match item {
            AccountHistoryDetails::Fee { .. } => AccountHistoryType::Fee,
            AccountHistoryDetails::Match { .. } => AccountHistoryType::Match,
            AccountHistoryDetails::Transfer { .. } => AccountHistoryType::Transfer,
            AccountHistoryDetails::Rebate { .. } => AccountHistoryType::Rebate,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AccountHolds {
    #[serde(rename = "ref")]
    pub _ref: Uuid,
    #[serde(rename = "type")]
    pub _type: AccountHoldsType,
    pub amount: f64,
    pub account_id: Uuid,
    pub created_at: DateTime,
    pub id: Uuid,
    pub updated_at: DateTime,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AccountHoldsType {
    Order,
    Transfer,
}


#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Auth {
    pub signature: String,
    pub key: String,
    pub passphrase: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Book<T> {
    pub sequence: usize,
    pub bids: Vec<T>,
    pub asks: Vec<T>,
}

pub trait BookLevel {
    fn level() -> u8;
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct BookRecordL1 {
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
    pub num_orders: usize,
}

impl BookLevel for BookRecordL1 {
    fn level() -> u8 {
        1
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct BookRecordL2 {
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
    pub num_orders: usize,
}

impl BookLevel for BookRecordL2 {
    fn level() -> u8 {
        2
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct BookRecordL3 {
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
    pub order_id: Uuid,
}

impl BookLevel for BookRecordL3 {
    fn level() -> u8 {
        3
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Candle(
    pub usize, // time
    pub f64, // low
    pub f64, // high
    pub f64, // open
    pub f64, // close
    pub f64,   // volume
);

#[derive(Clone, Debug, Deserialize)]
pub struct Credentials {
    pub key: String,
    pub secret: String,
    pub passphrase: String
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Currency {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "f64_from_string")]
    pub min_size: f64,
}


#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Fill {
    pub trade_id: usize,
    pub product_id: String,
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
    pub order_id: Uuid,
    pub created_at: DateTime,
    pub liquidity: FillLiquidity,
    #[serde(deserialize_with = "f64_from_string")]
    pub fee: f64,
    pub settled: bool,
    pub side: OrderSide,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FillLiquidity {
    M,
    T
}

pub enum Granularity {
    M1 = 60,
    M5 = 300,
    M15 = 900,
    H1 = 3600,
    H6 = 21600,
    D1 = 86400,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Level2SnapshotRecord {
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Level2UpdateRecord {
    pub side: OrderSide,
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MarketType {
    Size { size: f64 },
    Funds { funds: f64 },
}

// limit:{"id":"e9d0ff7a-ed50-4040-87a7-c884ae562807","price":"1.12000000","size":"1.00000000","product_id":"BTC-USD","side":"buy","stp":"dc","type":"limit","time_in_force":"GTC","post_only":true,"created_at":"2018-08-23T18:53:42.144811Z","fill_fees":"0.0000000000000000","filled_size":"0.00000000","executed_value":"0.0000000000000000","status":"pending","settled":false}
// market:{"id":"ea565dc3-1656-49d7-bcdb-d99981ce35a7","size":"0.00100000","product_id":"BTC-USD","side":"buy","stp":"dc","funds":"28.2449436100000000","type":"market","post_only":false,"created_at":"2018-08-23T18:43:18.964413Z","fill_fees":"0.0000000000000000","filled_size":"0.00000000","executed_value":"0.0000000000000000","status":"pending","settled":false}
// call:[{"id":"063da13d-6aba-45e1-91ca-89f8514da989","price":"100000.00000000","size":"0.00100000","product_id":"BTC-USD","side":"sell","type":"limit","time_in_force":"GTC","post_only":true,"created_at":"2018-08-24T04:50:01.139098Z","fill_fees":"0.0000000000000000","filled_size":"0.00000000","executed_value":"0.0000000000000000","status":"open","settled":false}]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Message {
    Error {
        message: String,
    },
    #[serde(skip)]
    InternalError(errors::CBProError),
    Interval(DateTime),
    None,
    Time(Time),
    #[serde(rename = "full")]
    WSFull(WSFull),
    #[serde(rename = "heartbeat")]
    WSHeartbeat {
        sequence: usize,
        last_trade_id: usize,
        product_id: String,
        time: DateTime,
    },
    #[serde(rename = "l2update")]
    WSL2update {
        product_id: String,
        time: DateTime,
        changes: Vec<Level2UpdateRecord>,
    },
    #[serde(alias = "snapshot")]
    WSSnapshot {
        product_id: String,
        bids: Vec<Level2SnapshotRecord>,
        asks: Vec<Level2SnapshotRecord>,
    },
    #[serde(rename = "status")]
    WSStatus(WSStatus),
    #[serde(rename = "subscribe")]
    WSSubscribe(WSSubscribe),
    #[serde(rename = "subscriptions")]
    WSSubscriptions {
        channels: Vec<WSChannel>,
    },
    #[serde(rename = "ticker")]
    WSTicker(WSTicker),
}

/*
impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
    }
}
impl From<InputMessage> for Message {
    fn from(msg: InputMessage) -> Self {
        match msg {
            InputMessage::Subscriptions { channels } => Message::Subscriptions { channels },
            InputMessage::Heartbeat {
                :w
                sequence,
                last_trade_id,
                product_id,
                time,
            } => Message::Heartbeat {
                sequence,
                last_trade_id,
                product_id,
                time,
            },
            InputMessage::Ticker(ticker) => Message::Ticker(ticker),
            InputMessage::Snapshot {
                product_id,
                bids,
                asks,
            } => Message::Level2(Level2::Snapshot {
                product_id,
                bids,
                asks,
            }),
            InputMessage::L2update {
                product_id,
                changes,
            } => Message::Level2(Level2::L2update {
                product_id,
                changes,
            }),
            InputMessage::LastMatch(_match) => Message::Match(_match),
            InputMessage::Received(_match) => Message::Full(Full::Received(_match)),
            InputMessage::Open(open) => Message::Full(Full::Open(open)),
            InputMessage::Done(done) => Message::Full(Full::Done(done)),
            InputMessage::Match(_match) => Message::Full(Full::Match(_match)),
            InputMessage::Change(change) => Message::Full(Full::Change(change)),
            InputMessage::Activate(activate) => Message::Full(Full::Activate(activate)),
            InputMessage::Error { message } => Message::Error { message },
            InputMessage::InternalError(err) => Message::InternalError(err),
        }
    }
}
*/



#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Order<'a> {
    #[serde(flatten)]
    pub _type: OrderType,
    #[serde(flatten)]
    client_oid: Option<Uuid>,
    pub created_at: DateTime,
    #[serde(flatten, deserialize_with = "f64_opt_from_string")]
    pub executed_value: Option<f64>,
    #[serde(flatten, deserialize_with = "f64_opt_from_string")]
    pub fill_fees: Option<f64>,
    #[serde(flatten, deserialize_with = "f64_opt_from_string")]
    pub filled_size: Option<f64>,
    pub id: Option<Uuid>,
    pub post_only: bool,
    pub product_id: Cow<'a, str>,
    pub settled: bool,
    pub side: OrderSide,
    #[serde(flatten)]
    pub status: Option<OrderStatus>,
    #[serde(flatten)]
    pub stop: Option<OrderStop>,
    pub stp: Option<String>, // Option because its not in get_orders, but in set_order
}

impl<'a> Order<'a> {
    pub fn buy_market<T: Into<Cow<'a, str>>>(product_id: T, size: f64) -> Self {
        Self::market(product_id, OrderSide::Buy, size)
    }

    pub fn sell_market<T: Into<Cow<'a, str>>>(product_id: T, size: f64) -> Self {
        Self::market(product_id, OrderSide::Sell, size)
    }


    pub fn buy_limit<T: Into<Cow<'a, str>>>(product_id: T, size: f64, price: f64, post_only: bool) -> Self {
        Self::limit(product_id, OrderSide::Buy, size, price, post_only)
    }

    pub fn sell_limit<T: Into<Cow<'a, str>>>(product_id: T, size: f64, price: f64, post_only: bool) -> Self {
        Self::limit(product_id, OrderSide::Sell, size, price, post_only)
    }

    pub fn client_oid(self, client_oid: Uuid) -> Self {
        let client_oid = Some(client_oid);
        Order { client_oid, ..self }
    }

    pub fn stop(self, price: f64, stop_type: OrderStopType) -> Self {
        let stop = Some(OrderStop { stop_price: price, _type: stop_type });
        Order { stop, ..self }
    }

    pub fn stop_loss(self, price: f64) -> Self {
        self.stop(price, OrderStopType::Loss)
    }

    pub fn stop_entry(self, price: f64) -> Self {
        self.stop(price, OrderStopType::Entry)
    }

    pub fn time_in_force(self, time_in_force: OrderTimeInForce) -> Self {
        match self._type {
            OrderType::Limit { price, size, post_only, .. } => {
                let _type = OrderType::Limit { price, size, post_only, time_in_force };
                Order { _type, ..self }
            }
            _ => panic!("time_in_force is for limit orders only")
        }
    }
    pub fn limit<T: Into<Cow<'a, str>>>(
        product_id: T,
        side: OrderSide,
        size: f64,
        price: f64,
        post_only: bool,
    ) -> Self {
        Order {
            _type: OrderType::Limit {
                price,
                size,
                post_only,
                time_in_force: OrderTimeInForce::GTC,
            },
            client_oid: None,
            created_at: now(),
            executed_value: None,
            fill_fees: None,
            filled_size: None,
            id: None,
            post_only,
            product_id: product_id.into(),
            settled: false,
            side,
            status: None,
            stop: None,
            stp: None,
        }
    }
    pub fn market<T: Into<Cow<'a, str>>>(
        product_id: T,
        side: OrderSide,
        size: f64,
    ) -> Self {
        Order {
            _type: OrderType::Market {
                size,
                funds: 0.0,
            },
            client_oid: None,
            created_at: now(),
            executed_value: None,
            fill_fees: None,
            filled_size: None,
            id: None,
            post_only: false,
            product_id: product_id.into(),
            settled: false,
            side,
            status: None,
            stop: None,
            stp: None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderStatus {
    Open,
    Done,
    Pending,
    Active,
    Rejected,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let res = match self {
            OrderStatus::Open => "open",
            OrderStatus::Done => "done",
            OrderStatus::Pending => "pending",
            OrderStatus::Active => "active",
            OrderStatus::Rejected => "rejected",
        };
        write!(f, "{}", res)
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct OrderStop {
    stop_price: f64,
    #[serde(rename = "stop")]
    _type: OrderStopType,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderStopType {
    Loss,
    Entry,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "time_in_force")]
pub enum OrderTimeInForce {
    GTC,
    GTT {
        cancel_after: OrderTimeInForceCancelAfter,
    },
    IOC,
    FOK,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderTimeInForceCancelAfter {
    Min,
    Hour,
    Day,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum OrderType {
    Limit {
        post_only: bool,
        #[serde(deserialize_with = "f64_from_string")]
        price: f64,
        #[serde(deserialize_with = "f64_from_string")]
        size: f64,
        #[serde(flatten)]
        time_in_force: OrderTimeInForce,
    },
    Market {
        #[serde(default)]
        #[serde(deserialize_with = "f64_from_string")]
        size: f64,
        //        #[serde(deserialize_with = "f64_opt_from_string")]
        //        funds: Option<f64>
        #[serde(default)]
        #[serde(deserialize_with = "f64_from_string")]
        funds: f64,
    },
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Product {
    pub id: String,
    pub base_currency: String,
    pub quote_currency: String,
    #[serde(deserialize_with = "f64_from_string")]
    pub base_min_size: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub base_max_size: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub quote_increment: f64,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Reason {
    Filled,
    Canceled,
}


#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Stats24H {
    #[serde(deserialize_with = "f64_from_string")]
    pub open: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub high: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub low: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub volume: f64,
}


#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct StatusCurrency {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "f64_from_string")]
    pub min_size: f64,
    pub status: String,
    pub funding_account_id: String,
    pub status_message: String,
    #[serde(deserialize_with = "f64_from_string")]
    pub max_precision: f64,
    pub convertible_to: Vec<String>,
    pub details: serde_json::Value
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StopType {
    Entry,
    Exit,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Time {
    pub iso: String,
    pub epoch: f64,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct TrailingVolume {
    pub product_id: String,
    #[serde(deserialize_with = "f64_from_string")]
    pub exchange_volume: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub volume: f64,
    pub recorded_at: DateTime,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Trade {
    pub time: DateTime,
    pub trade_id: usize,
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
    pub side: OrderSide,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum WSChannel {
    Name(WSChannelType),
    WithProduct {
        name: WSChannelType,
        product_ids: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub enum WSChannelType {
    Full,
    Heartbeat,
    Level2,
    Matches,
    Status,
    Ticker,
    User
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub enum WSFull {
    Activate(WSFullActivate),
    Change(WSFullChange),
    Done(WSFullDone),
    Match(WSFullMatch),
    Open(WSFullOpen),
    Received(WSFullReceived),
}

impl WSFull {
    pub fn price(&self) -> Option<&f64> {
        match self {
            WSFull::Activate(WSFullActivate { .. }) => None,
            WSFull::Change(WSFullChange { price, .. }) => price.as_ref(),
            WSFull::Done(WSFullDone::Limit { price, .. }) => Some(price),
            WSFull::Done(WSFullDone::Market { .. }) => None,
            WSFull::Match(WSFullMatch { price, .. }) => Some(price),
            WSFull::Open(WSFullOpen { price, .. }) => Some(price),
            WSFull::Received(WSFullReceived::Limit { price, .. }) => Some(price),
            WSFull::Received(WSFullReceived::Market { .. }) => None,
        }
    }

    pub fn time(&self) -> Option<&DateTime> {
        match self {
            WSFull::Activate(WSFullActivate { .. }) => None,
            WSFull::Change(WSFullChange { time, .. }) => Some(time),
            WSFull::Done(WSFullDone::Limit { time, .. }) => Some(time),
            WSFull::Done(WSFullDone::Market { time, .. }) => Some(time),
            WSFull::Match(WSFullMatch { time, .. }) => Some(time),
            WSFull::Open(WSFullOpen { time, .. }) => Some(time),
            WSFull::Received(WSFullReceived::Limit { time, .. }) => Some(time),
            WSFull::Received(WSFullReceived::Market { time, .. }) => Some(time),
        }
    }

    pub fn sequence(&self) -> Option<&usize> {
        match self {
            WSFull::Activate(WSFullActivate { .. }) => None,
            WSFull::Change(WSFullChange { sequence, .. }) => Some(sequence),
            WSFull::Done(WSFullDone::Limit { sequence, .. }) => sequence.as_ref(),
            WSFull::Done(WSFullDone::Market { sequence, .. }) => Some(sequence),
            WSFull::Match(WSFullMatch { sequence, .. }) => Some(sequence),
            WSFull::Open(WSFullOpen { sequence, .. }) => Some(sequence),
            WSFull::Received(WSFullReceived::Limit { sequence, .. }) => Some(sequence),
            WSFull::Received(WSFullReceived::Market { sequence, .. }) => Some(sequence),
        }
    }
}#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct WSFullActivate {
    pub product_id: String,
    #[serde(deserialize_with = "f64_from_string")]
    pub timestamp: f64,
    pub order_id: Uuid,
    pub stop_type: StopType,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub funds: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub taker_fee_rate: f64,
    pub private: bool,
    pub user_id: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "uuid_opt_from_string")]
    pub profile_id: Option<Uuid>,
}
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum WSFullDone {
    Limit {
        time: DateTime,
        product_id: String,
        sequence: Option<usize>,
        #[serde(deserialize_with = "f64_from_string")]
        price: f64,
        order_id: Uuid,
        reason: Reason,
        side: OrderSide,
        #[serde(deserialize_with = "f64_from_string")]
        remaining_size: f64,
        user_id: Option<String>,
        #[serde(default)]
        #[serde(deserialize_with = "uuid_opt_from_string")]
        profile_id: Option<Uuid>,
    },
    Market {
        time: DateTime,
        product_id: String,
        sequence: usize,
        order_id: Uuid,
        reason: Reason,
        side: OrderSide,
    },
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct WSFullChange {
    pub time: DateTime,
    pub sequence: usize,
    pub order_id: Uuid,
    pub product_id: String,
    #[serde(deserialize_with = "f64_from_string")]
    pub new_size: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub old_size: f64,
    #[serde(default)]
    #[serde(deserialize_with = "f64_opt_from_string")]
    pub new_funds: Option<f64>,
    #[serde(default)]
    #[serde(deserialize_with = "f64_opt_from_string")]
    pub old_funds: Option<f64>,
    #[serde(default)]
    #[serde(deserialize_with = "f64_opt_from_string")]
    pub price: Option<f64>,
    pub side: OrderSide,
    pub user_id: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "uuid_opt_from_string")]
    pub profile_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct WSFullMatch {
    pub sequence: usize,
    pub maker_order_id: Uuid,
    pub taker_order_id: Uuid,
    pub maker_user_id: Option<String>,
    pub maker_profile_id: Option<Uuid>,
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    pub product_id: String,
    #[serde(default)]
    #[serde(deserialize_with = "uuid_opt_from_string")]
    pub profile_id: Option<Uuid>,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
    pub side: OrderSide,
    pub taker_user_id: Option<String>,
    pub taker_profile_id: Option<Uuid>,
    pub time: DateTime,
    pub trade_id: usize,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct WSFullOpen {
    pub order_id: Uuid,
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    pub product_id: String,
    #[serde(default)]
    #[serde(deserialize_with = "uuid_opt_from_string")]
    pub profile_id: Option<Uuid>,
    #[serde(deserialize_with = "f64_from_string")]
    pub remaining_size: f64,
    pub sequence: usize,
    pub side: OrderSide,
    pub time: DateTime,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "order_type")]
#[serde(rename_all = "camelCase")]
pub enum WSFullReceived {
    Limit {
        product_id: String,
        sequence: usize,
        order_id: Uuid,
        #[serde(deserialize_with = "uuid_opt_from_string")]
        client_oid: Option<Uuid>,
        #[serde(deserialize_with = "f64_from_string")]
        size: f64,
        #[serde(deserialize_with = "f64_from_string")]
        price: f64,
        side: OrderSide,
        user_id: Option<String>,
        #[serde(default)]
        #[serde(deserialize_with = "uuid_opt_from_string")]
        profile_id: Option<Uuid>,
        time: DateTime,
    },
    Market {
        #[serde(deserialize_with = "uuid_opt_from_string")]
        client_oid: Option<Uuid>,
        #[serde(default)]
        #[serde(deserialize_with = "f64_opt_from_string")]
        funds: Option<f64>,
        order_id: Uuid,
        product_id: String,
        sequence: usize,
        side: OrderSide,
        time: DateTime,
    },
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum WSLevel2 {
    Snapshot {
        product_id: String,
        bids: Vec<Level2SnapshotRecord>,
        asks: Vec<Level2SnapshotRecord>,
    },
    L2update {
        product_id: String,
        time: DateTime,
        changes: Vec<Level2UpdateRecord>,
    },
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct WSStatus {
    pub currencies: Vec<StatusCurrency>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename = "subscribe")]
pub struct WSSubscribe {
    pub channels: Vec<WSChannel>,
    #[serde(flatten)]
    pub auth: Option<Auth>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum WSTicker {
    Full {
        #[serde(deserialize_with = "f64_nan_from_string")]
        best_ask: f64,
        #[serde(deserialize_with = "f64_nan_from_string")]
        best_bid: f64,
        #[serde(deserialize_with = "f64_from_string")]
        last_size: f64,
        #[serde(deserialize_with = "f64_from_string")]
        price: f64,
        product_id: String,
        sequence: usize,
        side: OrderSide,
        time: DateTime,
        trade_id: usize,
    },
    Empty {
        sequence: usize,
        product_id: String,
        #[serde(deserialize_with = "f64_nan_from_string")]
        price: f64,
    },
}

impl WSTicker {
    pub fn price(&self) -> &f64 {
        match self {
            WSTicker::Full { price, .. } => price,
            WSTicker::Empty { price, .. } => price
        }
    }

    pub fn time(&self) -> Option<&DateTime> {
        match self {
            WSTicker::Full { time, .. } => Some(time),
            WSTicker::Empty { .. } => None,
        }
    }

    pub fn sequence(&self) -> &usize {
        match self {
            WSTicker::Full { sequence, .. } => sequence,
            WSTicker::Empty { sequence, .. } => sequence
        }
    }

    pub fn bid(&self) -> Option<&f64> {
        match self {
            WSTicker::Full { best_bid, .. } => Some(best_bid),
            WSTicker::Empty { .. } => None,
        }
    }

    pub fn ask(&self) -> Option<&f64> {
        match self {
            WSTicker::Full { best_ask, .. } => Some(best_ask),
            WSTicker::Empty { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn test_order_builder() {
        let o = Order::buy_limit("BTC-USD", 10.0, 100.0, true);
        assert!(o.client_oid.is_none());

        match &o._type {
            OrderType::Limit { time_in_force: OrderTimeInForce::GTC, .. } => assert!(true),
            _ => assert!(false)
        }

        let o = Order::buy_limit("BTC-USD", 10.0, 100.0, true)
            .client_oid(Uuid::nil())
            .stop_loss(99.0)
            .time_in_force(OrderTimeInForce::GTC);
        assert!(o.client_oid.is_some());
        assert!(o.stop.is_some());

        match &o._type {
            OrderType::Limit { time_in_force: OrderTimeInForce::GTC, .. } => assert!(true),
            _ => assert!(false)
        }
    }

    #[derive(Debug, Serialize)]
    enum Coin { AAA, BBB }

    #[derive(Serialize)]
    struct Pair { a: Coin, b: Coin }

    impl<'a> From<Pair> for Order<'a> {
        fn from(pair: Pair) -> Self {
            Order::buy_market(format!("{:?}-{:?}", pair.a, pair.b), 10.0)
        }
    }

    #[test]
    fn test_order_from() {
        let p = Pair { a: Coin::AAA, b: Coin::BBB };
        let order_owned: Order = p.into();
        assert_eq!(order_owned.product_id, "AAA-BBB");
        let order_str: Order = Order::buy_market("AAA-BBB", 10.0);
        assert_eq!(order_str.product_id, "AAA-BBB");
    }


    #[test]
    fn test_parse_numbers() {
        #[derive(Debug, Deserialize, Serialize)]
        struct S {
            #[serde(deserialize_with = "f64_from_string")]
            a: f64,
            #[serde(deserialize_with = "f64_from_string")]
            b: f64,
            #[serde(deserialize_with = "f64_nan_from_string")]
            c: f64,
            #[serde(deserialize_with = "f64_opt_from_string")]
            d: Option<f64>,
            #[serde(deserialize_with = "f64_opt_from_string")]
            e: Option<f64>,
            #[serde(deserialize_with = "f64_opt_from_string")]
            f: Option<f64>,
            #[serde(default)]
            #[serde(deserialize_with = "f64_opt_from_string")]
            j: Option<f64>,
        }

        let json = r#"{
            "a": 5.5,
            "b":"5.5",
            "c":"",
            "d":"5.6",
            "e":5.6,
            "f":""
            }"#;
        let s: S = serde_json::from_str(json).unwrap();

        assert_eq!(5.5, s.a);
        assert_eq!(5.5, s.b);
        assert!(s.c.is_nan());
        assert_eq!(Some(5.6), s.d);
        assert_eq!(Some(5.6), s.e);
        assert_eq!(None, s.f);
        assert_eq!(None, s.j);
    }

    #[test]
    fn test_change_without_price() {
        let json = r#"{ "type" : "change", "side" : "sell", "old_size" : "7.53424298",
            "new_size" : "4.95057246", "order_id" : "0f352cbb-98a8-48ce-9dc6-3003870dcfd1",
            "product_id" : "BTC-USD", "sequence" : 7053090065,
            "time" : "2018-09-25T13:30:57.550000Z" }"#;

        let m: Message = serde_json::from_str(json).unwrap();
        let str = format!("{:?}", m);
        assert!(str.contains("product_id: \"BTC-USD\""));
    }

    #[test]
    fn test_canceled_order_done() {
        let json = r#"{"type": "done", "side": "sell", "order_id": "d05c295b-af2e-4f5e-bfa0-55d93370c450",
                       "reason":"canceled","product_id":"BTC-USD","price":"10009.17000000","remaining_size":"0.00973768",
                       "user_id":"0fd194ab8a8bf175a75f8de5","profile_id":"fa94ac51-b20a-4b16-bc7a-af3c0abb7ec4",
                       "time":"2019-08-21T22:10:15.190000Z"}"#;
        let m: Message = serde_json::from_str(json).unwrap();
        let str = format!("{:?}", m);
        assert!(str.contains("product_id: \"BTC-USD\""));
        assert!(str.contains("user_id: Some"));
        assert!(str.contains("profile_id: Some"));
    }

    #[test]
    fn test_canceled_order_without_auth() {
        let json = r#"{"type": "done", "side": "sell", "order_id": "d05c295b-af2e-4f5e-bfa0-55d93370c450",
                       "reason":"canceled","product_id":"BTC-USD","price":"10009.17000000","remaining_size":"0.00973768",
                       "time":"2019-08-21T22:10:15.190000Z"}"#;
        let m: Message = serde_json::from_str(json).unwrap();
        let str = format!("{:?}", m);
        assert!(str.contains("product_id: \"BTC-USD\""));
        assert!(str.contains("user_id: None"));
        assert!(str.contains("profile_id: None"));
    }

    #[test]
    fn test_parse_uuid() {
        #[derive(Debug, Deserialize, Serialize)]
        struct S {
            #[serde(deserialize_with = "uuid_opt_from_string")]
            uuid: Option<Uuid>
        }

        let json = r#"{
            "uuid":"2fec40ac-525b-4192-871a-39d784945055"
            }"#;
        let s: S = serde_json::from_str(json).unwrap();

        assert_eq!(s.uuid, Some(Uuid::from_str("2fec40ac-525b-4192-871a-39d784945055").unwrap()));

        let json = r#"{
            "uuid":""
            }"#;
        let s: S = serde_json::from_str(json).unwrap();

        assert!(s.uuid.is_none());
    }
}

