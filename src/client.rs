
// STD IMPORTS
use std::cell::RefCell;
use std::fmt::Debug;
use std::time::{SystemTime, UNIX_EPOCH};
use std::io;

// CRATE IMPORTS
use async_tungstenite::*;
use async_tungstenite::{async_std::connect_async, tungstenite::Error, tungstenite::Message as TMessage};
use chrono::SecondsFormat;
use futures::{Future, Sink, Stream};
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use hmac::{Hmac, Mac};
use hyper::{Body, Method, Request, Uri, Client};
use hyper::client::HttpConnector;
use hyper::header::HeaderValue;
use hyper_tls::{HttpsConnector, HttpsConnecting};
use serde::Deserialize;
use serde_json;
use url::Url;

// LOCAL IMPORTS
use crate::error::*;
use crate::structs::*;

const USER_AGENT: &str = concat!("coinbase-pro-one-rs/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Channel {
    Name(ChannelType),
    WithProduct {
        name: ChannelType,
        product_ids: Vec<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChannelType {
    Heartbeat,
    Ticker,
    Level2,
    Matches,
    Full,
    User,
}


struct Credentials {
    key: String,
    secret: String,
    passphrase: String
}

pub struct Conduit {
    base_http_uri: &'static str,
    base_ws_uri: &'static str,
    client: Client<HttpsConnector<HttpConnector>>,
    credentials: Option<Credentials>,
    receiver: UnboundedReceiver<Message>,
    sender: UnboundedSender<Message>
}

impl Conduit {
    pub fn timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("leap-second")
            .as_secs()
    }
    pub fn sign(&self, ts: u64, method: Method, uri: &str, body: &str, ) -> Option<String> {
        if self.credentials.is_none() {
            return None
        } else {
            let key = base64::decode(self.credentials.unwrap().secret.as_str())
                                  .expect("base64::decode secret");
            let mut mac: Hmac<sha2::Sha256> = Hmac::new_varkey(&self.credentials.unwrap().key.as_bytes())
                                                .expect("Hmac::new(key)");
            mac.input((ts.to_string() + method.as_str() + uri + body).as_bytes());
            Some(base64::encode(&mac.result().code()))
        }
   }

    /// Creates a new Conduit (with client)
    pub fn new(http_uri: &str, ws_uri: &str, product_ids: &[&str], channels: &[ChannelType],
               key: Option<String>, secret: Option<String>, passphrase: Option<String>)
                    -> Self {
        let url = Url::parse(http_uri).unwrap();
        debug!("Auth params: key={:?} secret={:?} passphrase={:?}", key, secret, passphrase);
        let credentials = if key != None && secret != None && passphrase != None {
            Some(Credentials {
                key: key.unwrap(),
                secret: secret.unwrap(),
                passphrase: passphrase.unwrap()
            })
        } else {
            None
        };

        let https = HttpsConnector::new(4).unwrap();
        let client = Client::builder()
            .keep_alive(true)
            .build::<_, Body>(https);
        let (sender, receiver) = unbounded();

        Self {
            base_http_uri: http_uri,
            base_ws_uri: ws_uri,
            client,
            credentials,
            receiver,
            sender
        }
    }

    /// Subscribe a Conduit to the Coinbase WS endpoint.
    pub fn subscribe(&mut self, product_ids: &[&str], channels: &[ChannelType]) {
        let timestamp = self.timestamp();
        let auth =
                if self.credentials.is_some() {
                    debug!("Calculating auth...");

                    let signature = self.sign(timestamp, Method::GET, "/users/self/verify", "");
                    let creds = self.credentials.unwrap();
                    Some(
                        Auth {
                            signature: signature.unwrap(),
                            key: creds.key.to_string(),
                            passphrase: creds.passphrase.unwrap().to_string(),
                            timestamp: timestamp.to_string()
                        }
                    )
                } else {
                    debug!("**Not** calculating auth... ");
                    None
                };
        let _subscribe = Subscribe {
            _type: SubscribeCmd::Subscribe,
            product_ids: product_ids.into_iter().map(|x| x.to_string()).collect(),
            channels: channels
                .to_vec()
                .into_iter()
                .map(|x| Channel::Name(x))
                .collect::<Vec<_>>(),
            auth: auth.clone(),
        };

        connect_async(self.base_ws_uri)
            .map_err(Error::Connect)
            .and_then(move |(ws_stream, _)| {
                debug!("WebSocket handshake has been successfully completed");
                let (sink, stream) = ws_stream.split();
                let subsribe = serde_json::to_string(&_subscribe).unwrap();

                sink.send(Message::Text(subsribe))
                    .map_err(Error::Send)
                    .and_then(|_| {
                        debug!("Subscription sent");
                        let stream = stream
                            .filter(|msg| msg.is_text())
                            .map_err(Error::Read)
                            .map(|msg| self.sender.send(self.convert_ws_msg(msg)));
                        Ok(stream)
                    })
            }).flatten_stream();
    }

    /// **Core Requests**
    ///
    ///
    ///

    fn request(&self, method: Method, _uri: &str, body: Option<String>) {
        let timestamp = self.timestamp();
        let uri: Uri = (self._pub.uri.to_string() + _uri).parse().unwrap();

        let req;
        let mut builder = Request::builder();
        builder.method(&method);
        builder.uri(uri);


        builder.header("User-Agent", USER_AGENT);
        builder.header("Content-Type", "Application/JSON");
        //        builder.header("Accept", "*/*");
        if (body.is_some()) {
            let _body = body.unwrap();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("leap-second")
                .as_secs();
            let sign = self.sign(timestamp, method, _uri, &_body);
            builder.header("CB-ACCESS-KEY", HeaderValue::from_str(&self.credentials.unwrap().key).unwrap());
            builder.header("CB-ACCESS-SIGN", HeaderValue::from_str(&sign.unwrap()));
            builder.header(
                "CB-ACCESS-PASSPHRASE",
                HeaderValue::from_str(&self.credentials.unwrap().passphrase).unwrap()
            );
        }
        builder.header(
            "CB-ACCESS-TIMESTAMP",
            HeaderValue::from_str(&timestamp.to_string()).unwrap(),
        );
        builder.header(
            "CB-ACCESS-PASSPHRASE",
            HeaderValue::from_str(&self.passphrase).unwrap(),
        );
        if body.is_some() {
            req = builder.body(body.into()).unwrap();
        } else {
            req = builder.body(Body::empty()).unwrap();
        }
        self.client.request(req)
            .map_err(Error::Http)
            .and_then(move |&resp| {
               self.sender.unbounded_send(convert_http_msg(resp.body()))
            });
    }

    fn _get(&self, uri: &str) {
        self._call(Method::GET, uri, None);
    }
    fn _post(&self, uri: &str, body: Option<&str>) {
        let bd = str_or_blank(body);
        self._call(Method::POST, uri, bd);
    }


    /// **Core Requests**
    ///
    ///
    ///
    pub fn time(&self) {
        self._get("/time");
    }

    pub fn products(&self) {
        self._get("/products");
    }

    /*
        pub fn book(&self, product_id: &str, level: u8) {
            self._get(&format!("/products/{}/book?level={}", product_id, level))
        }


        /// **Get an Account**
        ///
        /// Get a list of trading accounts
        ///
        /// # API Key Permissions
        /// This endpoint requires either the “view” or “trade” permission.
        pub fn get_accounts(&self) -> () {
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
        pub fn get_account(&self, account_id: Uuid) -> () {} {
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
    <<<<<<< Updated upstream
        pub fn get_account_hist(&self, id: Uuid) -> Result {
    =======
        pub fn get_account_hist(&self, id: Uuid) -> A::Result
            where
                A: Adapter<Vec<AccountHistory>> + 'static,
        {
    >>>>>>> Stashed changes
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
    <<<<<<< Updated upstream
        pub fn get_account_holds(&self, id: Uuid) -> Result {
    =======
        pub fn get_account_holds(&self, id: Uuid) -> A::Result
            where
                A: Adapter<Vec<AccountHolds>> + 'static,
        {
    >>>>>>> Stashed changes
            self.call_get(&format!("/accounts/{}/holds", id))
        }

        /// **Make Order**
        /// General function. Can be used to use own generated `Order` structure for order
    <<<<<<< Updated upstream
        pub fn order(&self, order: Order) -> Result {
    =======
        pub fn set_order(&self, order: Order) -> A::Result
            where
                A: Adapter<Order> + 'static,
        {
    >>>>>>> Stashed changes
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
    <<<<<<< Updated upstream
            post_only: bool,
        ) -> Result {
            self.order(Order::limit(
    =======
            post_only: bool
        ) -> A::Result
            where
                A: Adapter<Order> + 'static,
        {
            self.set_order(Order::limit(
    >>>>>>> Stashed changes
                product_id,
                OrderSide::Buy,
                size,
                price,
    <<<<<<< Updated upstream
                post_only,
    =======
                post_only
    >>>>>>> Stashed changes
            ))
        }

        /// **Sell limit**
        /// Makes Sell limit order
        pub fn sell_limit(
            &self,
            product_id: &str,
            size: f64,
            price: f64,
    <<<<<<< Updated upstream
            post_only: bool,
        ) -> Result {
            self.order(Order::limit(
    =======
            post_only: bool
        ) -> A::Result
            where
                A: Adapter<Order> + 'static,
        {
            self.set_order(Order::limit(
    >>>>>>> Stashed changes
                product_id,
                OrderSide::Sell,
                size,
                price,
    <<<<<<< Updated upstream
                post_only,
    =======
                post_only
    >>>>>>> Stashed changes
            ))
        }

        /// **Buy market**
        /// Makes Buy marker order
    <<<<<<< Updated upstream
        pub fn buy_market(&self, product_id: &str, size: f64) -> Result {
            self.order(Order::market(product_id, OrderSide::Buy, size))
    =======
        pub fn buy_market(&self, product_id: &str, size: f64) -> A::Result
            where
                A: Adapter<Order> + 'static,
        {
            self.set_order(Order::market(product_id, OrderSide::Buy, size))
    >>>>>>> Stashed changes
        }

        /// **Sell market**
        /// Makes Sell marker order
    <<<<<<< Updated upstream
        pub fn sell_market(&self, product_id: &str, size: f64) -> Result {
            self.order(Order::market(product_id, OrderSide::Sell, size))
    =======
        pub fn sell_market(&self, product_id: &str, size: f64) -> A::Result
            where
                A: Adapter<Order> + 'static,
        {
            self.set_order(Order::market(product_id, OrderSide::Sell, size))
    >>>>>>> Stashed changes
        }

        //    pub fn buy<'a>(&self) -> OrderBuilder<'a> {}    // TODO: OrderBuilder

        /// **Cancel an Order**
        ///
        /// Cancel a previously placed order.
        ///
        /// If the order had no matches during its lifetime its record may be purged. This means the order details will not be available with GET /orders/<order-id>.
        /// # API Key Permissions
        /// This endpoint requires the “trade” permission.
    <<<<<<< Updated upstream
        pub fn cancel_order(&self, id: Uuid) -> Result {
    =======
        pub fn cancel_order(&self, id: Uuid) -> A::Result
            where
                A: Adapter<Uuid> + 'static,
        {
    >>>>>>> Stashed changes
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
    <<<<<<< Updated upstream
        pub fn cancel_all(&self, product_id: Option<&str>) -> Result {
    =======
        pub fn cancel_all(&self, product_id: Option<&str>) -> A::Result
            where
                A: Adapter<Vec<Uuid>> + 'static,
        {
    >>>>>>> Stashed changes
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
    <<<<<<< Updated upstream
        pub fn get_orders(&self, status: Option<OrderStatus>, product_id: Option<&str>) -> Result {
    =======
        pub fn get_orders(&self, status: Option<OrderStatus>, product_id: Option<&str>) -> A::Result
            where
                A: Adapter<Vec<Order>> + 'static,
        {
    >>>>>>> Stashed changes
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
    <<<<<<< Updated upstream
        pub fn get_order(&self, id: Uuid) -> Result {
    =======
        pub fn get_order(&self, id: Uuid) -> A::Result
            where
                A: Adapter<Order> + 'static,
        {
    >>>>>>> Stashed changes
            self.call_get(&format!("/orders/{}", id))
        }

        /// **List Fills**
        ///
        /// Get a list of recent fills.
        ///
        /// # API Key Permissions
        /// This endpoint requires either the “view” or “trade” permission.
        /// **DEPRECATION NOTICE** - Requests without either order_id or product_id will be rejected after 8/23/18.
    <<<<<<< Updated upstream
        pub fn get_fills(&self, order_id: Option<Uuid>, product_id: Option<&str>) -> Result {
    =======
        pub fn get_fills(&self, order_id: Option<Uuid>, product_id: Option<&str>) -> A::Result
            where
                A: Adapter<Vec<Fill>> + 'static,
        {
    >>>>>>> Stashed changes
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
    <<<<<<< Updated upstream
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
    =======
        pub fn get_trailing_volume(&self) -> A::Result
            where
                A: Adapter<Vec<TrailingVolume>> + 'static,
        {
            self.call_get("/users/self/trailing-volume")
        }

        pub fn public(&self) -> &Public<A> {
            &self._pub
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

    impl WSFeed {

    }



    pub struct Public<Adapter> {
        pub(crate) uri: String,
        pub(crate) adapter: Adapter,
        client: Client<HttpsConnector<HttpConnector>>,
    }

    impl<A> Public<A> {
        pub(crate) const USER_AGENT: &'static str =
            concat!("coinbase-pro-rs/", env!("CARGO_PKG_VERSION"));

        fn request(&self, uri: &str) -> Request<Body> {
            let uri: Uri = (self.uri.to_string() + uri).parse().unwrap();

            let mut req = Request::get(uri);
            req.header("User-Agent", Self::USER_AGENT);
            req.body(Body::empty()).unwrap()
        }

        fn get_pub<U>(&self, uri: &str) -> A::Result
            where
                A: Adapter<U> + 'static,
    >>>>>>> Stashed changes
                U: Send + 'static,
                for<'de> U: serde::Deserialize<'de>,
        {
            self.call(self.request(uri))
        }

    <<<<<<< Updated upstream
        pub fn get_time(&self) -> Result {
            self.get_pub("/time")
        }

        pub fn get_products(&self) -> Result {
            self.get_pub("/products")
        }

        pub fn get_book<T>(&self, product_id: &str) -> Result
            where
    =======
        pub(crate) fn call_future<U>(
            &self,
            request: Request<Body>,
        ) -> Result<U>
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

        pub(crate) fn call<U>(&self, request: Request<Body>) -> A::Result
            where
                A: Adapter<U> + 'static,
                U: Send + 'static,
                for<'de> U: serde::Deserialize<'de>,
        {
            self.adapter.process(self.call_future(request))
        }


    <<<<<<< Updated upstream
        pub fn get_ticker(&self, product_id: &str) -> Result {
            self.get_pub(&format!("/products/{}/ticker", product_id))
        }

        pub fn get_trades(&self, product_id: &str) -> Result {
    =======
        pub fn get_ticker(&self, product_id: &str) -> A::Result
            where
                A: Adapter<Ticker> + 'static,
        {
            self.get_pub(&format!("/products/{}/ticker", product_id))
        }

        pub fn get_trades(&self, product_id: &str) -> A::Result
            where
                A: Adapter<Vec<Trade>> + 'static,
        {
    >>>>>>> Stashed changes
            self.get_pub(&format!("/products/{}/trades", product_id))
        }

        pub fn get_candles(
            &self,
            product_id: &str,
            start: Option<DateTime>,
            end: Option<DateTime>,
            granularity: Granularity,
    <<<<<<< Updated upstream
        ) -> Result {
    =======
        ) -> A::Result
            where
                A: Adapter<Vec<Candle>> + 'static,
        {
    >>>>>>> Stashed changes
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

    <<<<<<< Updated upstream
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


        pub fn get_stats24h(&self, product_id: &str) -> A::Result
            where
                A: Adapter<Stats24H> + 'static,
        {
            self.get_pub(&format!("/products/{}/stats", product_id))
        }

        pub fn get_currencies(&self) -> A::Result
            where
                A: Adapter<Vec<Currency>> + 'static,
        {
            self.get_pub("/currencies")
        }
    */
}

fn convert_http_msg(msg: Body) -> Message {
    match msg {
        Message::Text(str) => serde_json::from_str(&str).unwrap_or_else(|e| {
            Message::InternalError(Error::Serde {
                error: e,
                data: str,
            })
        }),
        _ => unreachable!(), // filtered in stream
    }
}

fn convert_ws_msg(msg: TMessage) -> Message {
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
