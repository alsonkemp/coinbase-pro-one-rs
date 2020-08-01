#!feature(async_closure)]

// STD IMPORTS
use std::time::{SystemTime, UNIX_EPOCH};

// CRATE IMPORTS
use async_std::{task, sync::{channel, Receiver, Sender}};
use async_tungstenite::{async_std::{ConnectStream, connect_async}, WebSocketStream,
                        tungstenite::{protocol::Message as TMessage}};
use crypto::{hmac::Hmac, mac::Mac};
use futures_util::{stream::SplitSink, SinkExt, TryStreamExt, FutureExt, StreamExt};
use reqwest::blocking::{Client};
use serde_json;
use serde_json::Value;

// LOCAL IMPORTS
use crate::structs::*;
use crate::errors::{CBProError};
use async_std::pin::Pin;
use futures_util::stream::SplitStream;
use std::borrow::BorrowMut;

const CHANNEL_SIZE: usize = 128;
const USER_AGENT: &str = concat!("coinbase-pro-one-rs/", env!("CARGO_PKG_VERSION"));

pub type FnReceiveFn = dyn Fn(Message);

#[derive(Copy, Clone, Debug)]
pub struct Credentials<'a> {
    key: &'a str,
    secret: &'a str,
    passphrase:&'a str
}

pub struct Conduit<'a> {
    last_time: u64,
    // The Coinbase REST endpoint.  Needed for all RESTy methods.
    base_http_uri: &'a str,
    // Coinbase credentials.
    credentials: Option<Credentials<'a>>,
    // To communicate with the 'user' of the library
    sender:   Sender<Message>,

    to_websocket: Sender<Message>,
    from_websocket: Receiver<Message>
}

impl Conduit<'static> {
    /// Creates a new Conduit
    pub async fn new(http_uri: &'static str, ws_uri: &'static str, _creds: Option<Credentials<'static>>) ->
    (Conduit<'static>, Receiver<Message>) {
        dbg!("Conduit.new: {:?} {:?} {:?}", http_uri, ws_uri, _creds);
        // Creates a new Conduit
        let credentials = if _creds.is_some() {
            let creds = _creds.unwrap();
            Some(Credentials {
                key: creds.key,
                secret: creds.secret,
                passphrase: creds.passphrase
            })
        } else {
            None
        };

        match connect_async(ws_uri).await {
            Err(e) => panic!("tungstenite: Failed to connect...: {:?}", e),
            Ok((ws, _)) => {
                dbg!("Conduit: WebSocket handshake has been successfully completed...");

                let (sender, receiver) = channel::<Message>(CHANNEL_SIZE);
                let (_to, to) = channel::<Message>(CHANNEL_SIZE);
                let (from, _from) = channel::<Message>(CHANNEL_SIZE);
                dbg!("Conduit: _websocket_handler: spawning...");
                task::spawn(async move {
                    dbg!("Conduit: _websocket_handler: spawned...");
                    _websocket_handler(Pin::new(Box::new(ws)),
                                       Pin::new(Box::new(from)),
                                       Pin::new(Box::new(to))).await;
                });
                (Conduit {
                    last_time: _timestamp(),
                    base_http_uri: http_uri,
                    credentials: credentials.clone(),
                    sender,
                    to_websocket: _to,
                    from_websocket: _from
                }, receiver)
            }
        }
    }


    pub fn sign(&self, ts: u64, method: reqwest::Method, uri: &str, body: &str) -> Option<String> {
        if self.credentials.is_none() {
            return None
        } else {
            let key = base64::decode(self.credentials.clone().unwrap().secret)
                .expect("base64::decode secret");
            let mut mac = Hmac::new(crypto::sha2::Sha256::new(), &key);
            mac.input((ts.to_string() + method.as_str() + uri + body).as_bytes());
            Some(base64::encode(&mac.result().code()))
        }
    }


    /// Subscribe a Conduit to the Coinbase WS endpoint.
    pub async fn subscribe(& mut self, channels: &[Channel]) {
        let subscribe = Subscribe {
            channels: channels.to_vec(),
            auth: _auth(self.credentials)
        };

        let msg = Message::Subscribe(subscribe);
        dbg!("Conduit: subscription: sending {:?}", serde_json::to_string(&msg).unwrap());
        self.to_websocket.send(msg).await;
    }

    /// **Core Requests**
    ///
    ///
    ///
    async fn _request(& mut self, method: reqwest::Method, path: &str, body: Option<String>, _type: &str) {
        dbg!("Conduit: _request:", &method, &path, &body);
        let mut req = Client::new()
            .request(method.clone(), &format!("{}{}", self.base_http_uri, path))
            .header("User-Agent", USER_AGENT)
            .header("Content-Type", "Application/JSON");
        if body.is_some() {
            let _body = body.unwrap();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("leap-second")
                .as_secs();
            let sign = _sign(&self.credentials, timestamp, method, path, &_body);
            let creds = self.credentials.unwrap();
            req = req.header("CB-ACCESS-KEY", creds.key)
                .header("CB-ACCESS-SIGN", sign.unwrap())
                .header("CB-ACCESS-PASSPHRASE", creds.passphrase)
                .header("CB-ACCESS-TIMESTAMP", &timestamp.to_string());
            req = req.body(_body);
        }

        let resp = Client::new().execute(req.build().unwrap());
        let msg = if resp.is_err() {
            Message::InternalError(CBProError::Http(resp.err().unwrap().to_string()))
        } else {
            convert_http(resp.unwrap(), _type).await
        };
        self.sender.send(msg).await;
    }

    async fn _get(&mut self, _type: &str, uri: &str) {
        self._request(reqwest::Method::GET, uri, None, _type).await;
    }
    async fn _post(&mut self, _type: &str, uri: &str, body: Option<String>) {
        self._request(reqwest::Method::POST, uri, body, _type).await;
    }


    /// **Core Requests**
    ///
    ///
    ///
    pub async fn heartbeat(& mut self) {
        dbg!("Conduit: heartbeat...");
        self.subscribe(&[Channel::WithProduct{
            name: ChannelType::Heartbeat,
            product_ids:vec!("BTC-USD".to_string())}]).await
    }

    pub async fn products(&mut self) {
        dbg!("Conduit: products sent...");
        self._get("Products", "/products").await;
    }
    pub async fn status(& mut self) {
        dbg!("Conduit: status...");
        self.subscribe(&[Channel::Name(
            ChannelType::Status
        )]).await;
    }
    pub async fn time(&mut self) {
        dbg!("Conduit: time sent...");
        self._get("Time", "/time").await;
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
            self.call_get("/ accounts")
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
            dbg!("REQ: {:?}", request);

            self.client
                .request(request)
                .map_err(CBError::Http)
                .and_then(|res| res.into_body().concat2().map_err(CBError::Http))
                .and_then(|body| {
                    dbg!("RES: {:?}", body);
                    let res = serde_json::from_slice(&body).map_err(|e| {
                        serde_json::from_slice(&body)
                            .map(CBError::Coinbase)
                            .unwrap_or_elgg/se(|_| {
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

async fn convert_http(resp: reqwest::blocking::Response, _type: &str) -> Message {
    let txt = resp.text().unwrap();
    let mut v: Value = serde_json::from_str(txt.as_str()).unwrap();
    v["type"] = serde_json::Value::String(_type.to_string());
    serde_json::from_value(v).unwrap_or_else(|e| {
        Message::InternalError(CBProError::Serde(e.to_string()))
    })
}


pub fn _timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("leap-second")
        .as_secs()
}

pub fn _sign(&credentials: &Option<Credentials>, ts: u64, method: reqwest::Method, uri: &str, body: &str) -> Option<String> {
    if credentials.is_none() {
        return None
    } else {
        let key = base64::decode(credentials.clone().unwrap().secret)
            .expect("base64::decode secret");
        let mut mac = Hmac::new(crypto::sha2::Sha256::new(), &key);
        mac.input((ts.to_string() + method.as_str() + uri + body).as_bytes());
        Some(base64::encode(&mac.result().code()))
    }
}

pub fn _auth(credentials: Option<Credentials<'static>>) -> Option<Auth> {
    match credentials{
        Some(c) => {
            dbg!("Conduit: calculating auth...");
            let ts = _timestamp();
            let signature = _sign(
                &credentials,
                ts,
                reqwest::Method::GET,
                "/users/self/verify",
                "");
            Some(
                Auth {
                    signature: signature.unwrap(),
                    key: c.key.to_string(),
                    passphrase: c.passphrase.to_string(),
                    timestamp: ts.to_string()
                }
            )
        },
        None => {
            dbg!("Conduit: **not** calculating auth... ");
            None
        }
    }
}

pub async fn _websocket_handler(    ws:   Pin<Box<WebSocketStream<ConnectStream>>>,
                                mut from: Pin<Box<Sender<Message>>>,
                                mut to:   Pin<Box<Receiver<Message>>>) {

    async fn handle_outgoing(
            _ws_write: &mut SplitSink<Pin<Box<WebSocketStream<ConnectStream>>>, TMessage>,
            _to:       &mut  Pin<Box<Receiver<Message>>>) {
        let msg = serde_json::to_string(&_to.recv().await.unwrap()).unwrap();
        dbg!("Conduit. _websocket_handler._handle_outgoing sending{:?}", &msg);
        _ws_write.send(TMessage::Text(msg)).await.unwrap();

    };

    async fn handle_incoming(
          mut _ws_read: &mut SplitStream<Pin<Box<WebSocketStream<ConnectStream>>>>,
              _from:    &mut Pin<Box<Sender<Message>>>) {
        let tmsg = _ws_read.try_next().await;
        dbg!("Conduit. _websocket_handler._handle_incoming: received {:?}", &tmsg);
        let cmsg = match tmsg {
            Ok(Some(TMessage::Text(msg))) =>
                serde_json::from_str(&msg).unwrap_or_else(|e| {
                    Message::InternalError(CBProError::Serde(e.to_string()))
            }),
            o => {
                dbg!("Conduit._websocket_handler.handle_incoming: unknown result {:?}", o);
                Message::None
            }
        };
        if cmsg != Message::None {
            _from.send(cmsg).await;
        }
    }

    dbg!("Conduit. _websocket_handler._handle_outgoing handling...");
    let (mut ws_write, mut ws_read) = ws.split();
    loop {
        futures::future::select(
            handle_outgoing(&mut ws_write, to.borrow_mut()).boxed(),
            handle_incoming(&mut ws_read, from.borrow_mut()).boxed()
        ).await;
    }
}

