// globals
use chrono::SecondsFormat;
// use futures::Future as FFuture;
use futures::{TryFutureExt, FutureExt};
use hmac::*;
use hyper::{Body as HBody, Request as HRequest};
use hyper::Client as HClient;
use hyper::client::HttpConnector;
use hyper::header::HeaderValue;
use hyper::Method;
// use hyper::rt;
use hyper_tls::HttpsConnector;
use serde::Deserialize;
use serde_json;
// use std::cell::RefCell;
use std::fmt::Debug;
use std::time::{SystemTime, UNIX_EPOCH};
// use tokio::runtime::Runtime;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as TMessage;
use url;
use uuid::Uuid;

use errors::Error;
use errors::CBError;
use structs::*;

pub type Stream = Box<dyn futures::Stream<Item=Message>>;

pub struct Client {
    client: HClient<HttpsConnector<HttpConnector>>,
    stream: Stream,
    key: String,
    passphrase: String,
    secret: String,
    uri: String,
}

type Result = hyper::client::ResponseFuture;

impl Client {
    const USER_AGENT: &'static str =
        concat!("coinbase-pro-rs/", env!("CARGO_PKG_VERSION"));
    pub fn new(
        uri: &str,
        key: &str, secret: &str, passphrase: &str,
        product_ids: &[&str],
        channels: &[ChannelType],
    ) -> impl futures::Stream<Item=Message> {
        let https = HttpsConnector::new().unwrap();
        let client = Client::builder()
            .build::<_, HBody>(https);
        let uri = uri.to_string();
        let url = url::Url::parse(uri.into()).unwrap();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("leap-second")
            .as_secs();
        let signature = Client::sign(secret, timestamp, Method::GET, "/users/self/verify", "");

        let auth = Auth {
            signature,
            key: key.to_string(),
            passphrase: passphrase.to_string(),
            timestamp: timestamp.to_string(),
        };

        let subscribe = Subscribe {
            _type: SubscribeCmd::Subscribe,
            product_ids: product_ids.into_iter().map(|x| x.to_string()).collect(),
            channels: channels
                .to_vec()
                .into_iter()
                .map(|x| Channel::Name(x))
                .collect::<Vec<_>>(),
            auth: Some(auth),
        }.into();

        let stream = connect_async(url)
            .map_err(Error::Connect)
            .and_then(move |(ws_stream, _)| {
                debug!("WebSocket handshake has been successfully completed");
                let (sink, stream) = ws_stream.split();

                sink.send(TMessage::Text(subscribe))
                    .map_err(Error::Send)
                    .and_then(|_| {
                        debug!("subsription sent");
                        let stream = stream
                            .filter(|msg| msg.is_text())
                            .map_err(Error::Read)
                            .map(convert_msg);
                        Ok(stream)
                    })
            }).flatten_stream();

        Self {
            uri: String::from(uri),
            stream,
            client,
            key: key.to_string(),
            secret: secret.to_string(),
            passphrase: passphrase.to_string(),
        }
    }

    /// **Core Requests**
    ///
    ///
    ///
    fn call<U>(&self, method: Method, uri: &str, body_str: &str) -> Result
        where
            U: Send + 'static,
            for<'de> U: serde::Deserialize<'de>
    {
        self._pub
            .call(self.request(method, uri, body_str.to_string()))
    }

    fn call_feature<U>(
        &self,
        method: Method,
        uri: &str,
        body_str: &str,
    ) -> impl hyper::rt::Executor<Item=U, Error=CBError>
        where
                for<'de> U: serde::Deserialize<'de>, {
        self._pub
            .call_future(self.request(method, uri, body_str.to_string()))
    }
    pub fn call_future<U>(
        &self,
        request: HRequest<HBody>,
    ) -> impl futures::Future<Item=U, Error=CBError>
        where
                for<'de> U: serde::Deserialize<'de>,
    {
        debug!("REQ: {:?}", request);

        self.client
            .request(request)
            .map_err(CBError::Http)
            .and_then(|res| res.into_body().concat2().map_err(CBError::Http))
            .and_then(|body| {
                debug!("RES: {:?}", body);
                let res = serde_json::from_slice(&body).map_err(|e| {
                    serde_json::from_slice(&body)
                        .map(CBError::Coinbase)
                        .unwrap_or_else(|_| {
                            let data = String::from_utf8(body.to_vec()).unwrap();
                            CBError::Serde { error: e, data }
                        })
                })?;
                Ok(res)
            })
    }

    fn call_get<U>(&self, uri: &str, body: Option<&str>) -> Result
        where
            U: Send + 'static,
            for<'de> U: serde::Deserialize<'de> {
        let bd = str_or_blank(body);
        self.call(Method::GET, uri, bd)
    }
    fn call_post<U>(&self, uri: &str, body: Option<&str>) -> Result
        where
            U: Send + 'static,
            for<'de> U: serde::Deserialize<'de> {
        let bd = str_or_blank(body);
        self.call(Method::POST, uri, bd)
    }

    //   from python
    //POST /orders HTTP/1.1
    //Host: localhost:3000
    //User-Agent: python-requests/2.13.0
    //Accept-Encoding: gzip, deflate
    //Accept: */*
    //Connection: keep-alive
    //Content-Length: 92
    //Content-Type: Application/JSON
    //CB-ACCESS-SIGN: Hy8vbkj3r/XoaT46oQveZs8OIl6zX/xRR6lKTSvfxuk=
    //CB-ACCESS-TIMESTAMP: 1535003621.005189
    //CB-ACCESS-KEY: 1d0dc0f7b4e808d430b95d8fed7df3ea
    //CB-ACCESS-PASSPHRASE: sandbox
    //
    //{"product_id": "BTC-USD", "side": "buy", "type": "limit", "price": "100.00", "size": "0.01"}
    fn request(&self, method: Method, _uri: &str, body_str: String) -> HRequest<HBody> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("leap-second")
            .as_secs();

        let uri: hyper::Uri = (self._pub.uri.to_string() + _uri).parse().unwrap();

        let mut req = HRequest::builder();
        req.method(&method);
        req.uri(uri);

        let sign = Self::sign(&self.secret, timestamp, method, _uri, &body_str);

        req.header("User-Agent", Self::USER_AGENT);
        req.header("Content-Type", "Application/JSON");
        //        req.header("Accept", "*/*");
        req.header("CB-ACCESS-KEY", HeaderValue::from_str(&self.key).unwrap());
        req.header("CB-ACCESS-SIGN", HeaderValue::from_str(&sign).unwrap());
        req.header(
            "CB-ACCESS-TIMESTAMP",
            HeaderValue::from_str(&timestamp.to_string()).unwrap(),
        );
        req.header(
            "CB-ACCESS-PASSPHRASE",
            HeaderValue::from_str(&self.passphrase).unwrap(),
        );

        req.body(body_str.into()).unwrap()
    }

    /// **Get an Account**
    ///
    /// Get a list of trading accounts
    ///
    /// # API Key Permissions
    /// This endpoint requires either the “view” or “trade” permission.
    pub fn get_accounts(&self) -> Result {
        self.call_get("/accounts")
    }

    /// **Get Account History**
    ///
    /// Information for a single account. Use this endpoint when you know the account_id.
    ///
    /// # API Key Permissions
    /// This endpoint requires either the “view” or “trade” permission.
    ///
    /// # Account Fields
    /// | Field | Description |
    /// | ----- | ----------- |
    /// | id |	Account ID |
    /// | balance |	total funds in the account |
    /// | holds |	funds on hold (not available for use) |
    /// | available |	funds available to withdraw or trade |
    pub fn get_account(&self, account_id: Uuid) -> Result {
        self.call_get(&format!("/accounts/{}", account_id))
    }

    /// **Get Account History**
    /// List account activity. Account activity either increases or decreases your account balance.
    /// Items are paginated and sorted latest first. See the Pagination section for retrieving
    /// additional entries after the first page.
    /// # API Key Permissions
    /// This endpoint requires either the “view” or “trade” permission.
    ///
    /// # Entry Types
    /// | Field | Description |
    /// | ----- | ----------- |
    /// | type |	Entry type indicates the reason for the account change. |
    /// | transfer |	Funds moved to/from Coinbase to Coinbase Pro |
    /// | match |	Funds moved as a result of a trade |
    /// | fee |	Fee as a result of a trade |
    /// | rebate |	Fee rebate as per our fee schedule |
    ///
    /// # Details
    ///
    /// If an entry is the result of a trade (match, fee), the details field will contain additional information about the trade.
    pub fn get_account_hist(&self, id: Uuid) -> Result {
        let f = self
            .call_feature(Method::GET, &format!("/accounts/{}/ledger", id), "")
            .map(|xs: Vec<AccountHistory>| {
                xs.into_iter()
                    .map(|x| AccountHistory {
                        _type: (&x.details).into(),
                        ..x
                    }).collect()
            });

        self._pub.adapter.process(f)
    }

    /// **Get Holds**
    /// Holds are placed on an account for any active orders or pending withdraw requests.
    /// As an order is filled, the hold amount is updated. If an order is canceled, any remaining
    /// hold is removed. For a withdraw, once it is completed, the hold is removed.
    ///
    /// # API Key Permissions
    /// This endpoint requires either the “view” or “trade” permission.
    ///
    /// # Type
    /// The type of the hold will indicate why the hold exists. The hold type is order for holds
    /// related to open orders and transfer for holds related to a withdraw.
    ///
    /// # Ref
    /// The ref field contains the id of the order or transfer which created the hold.
    ///
    pub fn get_account_holds(&self, id: Uuid) -> Result {
        self.call_get(&format!("/accounts/{}/holds", id))
    }

    /// **Make Order**
    /// General function. Can be used to use own generated `Order` structure for order
    pub fn order(&self, order: Order) -> Result {
        let body_str = serde_json::to_string(&order).expect("cannot to_string post body");

        self.call(Method::POST, "/orders", &body_str)
    }

    /// **Buy limit**
    /// Makes Buy limit order
    pub fn buy_limit(
        &self,
        product_id: &str,
        size: f64,
        price: f64,
        post_only: bool,
    ) -> Result {
        self.order(Order::limit(
            product_id,
            OrderSide::Buy,
            size,
            price,
            post_only,
        ))
    }

    /// **Sell limit**
    /// Makes Sell limit order
    pub fn sell_limit(
        &self,
        product_id: &str,
        size: f64,
        price: f64,
        post_only: bool,
    ) -> Result {
        self.order(Order::limit(
            product_id,
            OrderSide::Sell,
            size,
            price,
            post_only,
        ))
    }

    /// **Buy market**
    /// Makes Buy marker order
    pub fn buy_market(&self, product_id: &str, size: f64) -> Result {
        self.order(Order::market(product_id, OrderSide::Buy, size))
    }

    /// **Sell market**
    /// Makes Sell marker order
    pub fn sell_market(&self, product_id: &str, size: f64) -> Result {
        self.order(Order::market(product_id, OrderSide::Sell, size))
    }

    //    pub fn buy<'a>(&self) -> OrderBuilder<'a> {}    // TODO: OrderBuilder

    /// **Cancel an Order**
    ///
    /// Cancel a previously placed order.
    ///
    /// If the order had no matches during its lifetime its record may be purged. This means the order details will not be available with GET /orders/<order-id>.
    /// # API Key Permissions
    /// This endpoint requires the “trade” permission.
    pub fn cancel_order(&self, id: Uuid) -> Result {
        let f = self
            .call_feature(Method::DELETE, dbg!(&format!("/orders/{}", id)), "");

        self._pub.adapter.process(f)
    }

    /// **Cancel all**
    ///
    /// With best effort, cancel all open orders. The response is a list of ids of the canceled orders.
    ///
    /// # API Key Permissions
    /// This endpoint requires the “trade” permission.
    ///
    /// # Query Parameters
    /// | Param |	Default |	Description |
    /// | ----- | --------- | ------------- |
    /// | product_id |	*optional* |	Only cancel orders open for a specific product |
    pub fn cancel_all(&self, product_id: Option<&str>) -> Result {
        let param = product_id
            .map(|x| format!("?product_id={}", x))
            .unwrap_or_default();

        self.call(Method::DELETE, &format!("/orders{}", param), "")
    }

    /// **List Orders**
    ///
    /// List your current open orders. Only open or un-settled orders are returned.
    /// As soon as an order is no longer open and settled, it will no longer appear in the default request.
    ///
    /// # API Key Permissions
    /// This endpoint requires either the “view” or “trade” permission.
    ///
    /// # Query Parameters
    /// | Param 	Default 	Description |
    /// | ------ | -------- | ------------ |
    /// | status |	*open*, *pending*, *active* | 	Limit list of orders to these statuses. Passing all returns orders of all statuses. |
    /// | product_id |	*optional* |	Only list orders for a specific product |
    pub fn get_orders(&self, status: Option<OrderStatus>, product_id: Option<&str>) -> Result {
        // TODO rewrite
        let param_status = status.map(|x| format!("&status={}", x)).unwrap_or_default();
        let param_product = product_id
            .map(|x| format!("&product_id={}", x))
            .unwrap_or_default();
        let mut param = (param_status + &param_product).into_bytes();
        if !param.is_empty() {
            param[0] = b'?';
        }

        self.call_get(&format!("/orders{}", String::from_utf8(param).unwrap()))
    }

    /// **Get an Order**
    ///
    /// Get a single order by order id.
    ///
    /// # API Key Permissions
    /// This endpoint requires either the “view” or “trade” permission.
    ///
    /// If the order is canceled the response may have status code 404 if the order had no matches.
    pub fn get_order(&self, id: Uuid) -> Result {
        self.call_get(&format!("/orders/{}", id))
    }

    /// **List Fills**
    ///
    /// Get a list of recent fills.
    ///
    /// # API Key Permissions
    /// This endpoint requires either the “view” or “trade” permission.
    /// **DEPRECATION NOTICE** - Requests without either order_id or product_id will be rejected after 8/23/18.
    pub fn get_fills(&self, order_id: Option<Uuid>, product_id: Option<&str>) -> Result {
        let param_order = order_id
            .map(|x| format!("&order_id={}", x))
            .unwrap_or_default();
        let param_product = product_id
            .map(|x| format!("&product_id={}", x))
            .unwrap_or_default();
        let mut param = (param_order + &param_product).into_bytes();
        if !param.is_empty() {
            param[0] = b'?';
        }
        self.call_get(&format!("/fills{}", String::from_utf8(param).unwrap()))
    }

    /// **Trailing Volume**
    ///
    /// This request will return your 30-day trailing volume for all products. This is a cached
    /// value that’s calculated every day at midnight UTC.
    ///
    /// #API Key Permissions
    /// This endpoint requires either the “view” or “trade” permission.
    pub fn get_trailing_volume(&self) -> Result {
        self.call_get("/users/self/trailing-volume")
    }

    fn pub_request(&self, uri: &str) -> HRequest<HBody> {
        let uri: hyper::Uri = (self.pub_uri.to_string() + uri).parse().unwrap();

        let mut req = HRequest::get(uri);
        req.header("User-Agent", Self::USER_AGENT);
        req.body(HBody::empty()).unwrap()
    }

    fn get_pub<U>(&self, uri: &str) -> Result
        where
            U: Send + 'static,
            for<'de> U: serde::Deserialize<'de>,
    {
        self.call(self.request(uri))
    }

    pub fn get_time(&self) -> Result {
        self.get_pub("/time")
    }

    pub fn get_products(&self) -> Result {
        self.get_pub("/products")
    }

    pub fn get_book<T>(&self, product_id: &str) -> Result
        where
            T: BookLevel + Debug + 'static,
            T: super::std::marker::Send,
            T: for<'de> Deserialize<'de>,
    {
        self.get_pub(&format!(
            "/products/{}/book?level={}",
            product_id,
            T::level()
        ))
    }

    pub fn get_ticker(&self, product_id: &str) -> Result {
        self.get_pub(&format!("/products/{}/ticker", product_id))
    }

    pub fn get_trades(&self, product_id: &str) -> Result {
        self.get_pub(&format!("/products/{}/trades", product_id))
    }

    pub fn get_candles(
        &self,
        product_id: &str,
        start: Option<DateTime>,
        end: Option<DateTime>,
        granularity: Granularity,
    ) -> Result {
        let param_start = start
            .map(|x| format!("&start={}", x.to_rfc3339_opts(SecondsFormat::Secs, true)))
            .unwrap_or_default();
        let param_end = end
            .map(|x| format!("&end={}", x.to_rfc3339_opts(SecondsFormat::Secs, true)))
            .unwrap_or_default();

        let req = format!(
            "/products/{}/candles?granularity={}{}{}",
            product_id, granularity as usize, param_start, param_end
        );
        self.get_pub(&req)
    }

    pub fn get_stats24h(&self, product_id: &str) -> Result {
        self.get_pub(&format!("/products/{}/stats", product_id))
    }

    pub fn get_currencies(&self) -> Result {
        self.get_pub("/currencies")
    }
    pub fn sign(secret: &str, timestamp: u64, method: Method, uri: &str, body_str: &str) -> String {
        let key = base64::decode(secret).expect("base64::decode secret");
        let mut mac: hmac::Hmac<sha2::Sha256> = hmac::Mac::new_varkey(&key).expect("Hmac::new(key)");
        mac.input((timestamp.to_string() + method.as_str() + uri + body_str).as_bytes());
        base64::encode(&mac.result().code())
    }
}

fn convert_msg(msg: TMessage) -> Message {
    match msg {
        TMessage::Text(str) => serde_json::from_str(&str).unwrap_or_else(|e| {
            Message::InternalError(Error::Serde {
                error: e,
                data: str,
            })
        }),
        _ => unreachable!(), // filtered in stream
    }
}

fn str_or_blank(v: Option<&str>) -> &str {
    match v {
        Some(s) => s,
        None => ""
    }
}
