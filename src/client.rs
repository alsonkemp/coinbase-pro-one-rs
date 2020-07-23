#!feature(async_closure)]

// STD IMPORTS
use std::time::{SystemTime, UNIX_EPOCH};

// CRATE IMPORTS
use async_std::net::TcpStream;
use async_std::sync::MutexGuard;
use async_std::pin::Pin;
use async_std::sync::Mutex;
use async_tungstenite::{async_std::connect_async, WebSocketStream,
                        tungstenite::{protocol::Message as TMessage,
                                      error::Error as TError } };
use futures::{stream::{SplitSink}, SinkExt, StreamExt};
use futures::channel::mpsc::{UnboundedSender, UnboundedReceiver, unbounded};

use crypto::{hmac::Hmac, mac::Mac};
use reqwest;
use reqwest::{Request, Client};
use serde_json;

// LOCAL IMPORTS
use crate::structs::*;
use crate::errors::{CBProError};
use std::ops::Deref;

const USER_AGENT: &str = concat!("coinbase-pro-one-rs/", env!("CARGO_PKG_VERSION"));

pub type FnReceiveFn = dyn Fn(Message);
#[derive(Clone, Debug)]
pub struct Credentials {
    key: String,
    secret: String,
    passphrase: String
}
pub struct Conduit<'a> {
    base_http_uri: &'a str,
    credentials: Option<Credentials>,
    client: reqwest::Client,
    // Private endpoints
    pub sender:     UnboundedSender<Message>,
    pub receiver:   Mutex<&'a mut UnboundedReceiver<Message>>,
    websocket: Mutex<SplitSink<WebSocketStream<TcpStream>, TMessage>>
}

impl <'a> Conduit<'a> {
    pub fn _auth(&self) -> Option<Auth> {
        let ts = self._timestamp();
        if self.credentials.is_some() {
            debug!("Calculating auth...");

            let signature = self.sign(
                                            ts,
                                            reqwest::Method::GET,
                                            "/users/self/verify",
                                            "");
            let creds = self.credentials.clone().unwrap();
            Some(
                Auth {
                    signature: signature.unwrap(),
                    key: creds.key.to_string(),
                    passphrase: creds.passphrase.to_string(),
                    timestamp: ts.to_string()
                }
            )
        } else {
            debug!("**Not** calculating auth... ");
            None
        }
    }

    pub fn _timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("leap-second")
            .as_secs()
    }

    pub fn sign(&self, ts: u64, method: reqwest::Method, uri: &str, body: &str) -> Option<String> {
        if self.credentials.is_none() {
            return None
        } else {
            let key = base64::decode(self.credentials.clone().unwrap().secret.as_str())
                                  .expect("base64::decode secret");
            let mut mac = Hmac::new(crypto::sha2::Sha256::new(), &key);
            mac.input((ts.to_string() + method.as_str() + uri + body).as_bytes());
            Some(base64::encode(&mac.result().code()))
        }
   }

    /// Creates a new Conduit
    pub async fn new(http_uri: &'static str, ws_uri: &'static str, _creds: Option<Credentials>) -> Conduit<'a>{
        debug!("Auth params: _creds={:?}", _creds);
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

        let (sender, mut _receiver) = unbounded::<Message>();
        let recieiver= Mutex::new(&mut _receiver);
        match connect_async(ws_uri).await {
            Err(e) => panic!("tungstenite: Failed to connect...: {:?}", e),
            Ok((ws, r)) => {
                debug!("WebSocket handshake has been successfully completed: {:?}", r);

                let (ws_out, ws_in) = ws.split();
                ws_in.for_each(|r| async {
                    _handle_ws_resp(sender, r).await
                });
                Conduit {
                    base_http_uri: http_uri,
                    client: reqwest::Client::new(),
                    credentials: credentials.clone(),
                    sender,
                    receiver,
                    websocket: Mutex::new(ws_out)
                }
            }
        }
    }



    pub async fn heartbeat(& mut self) {
        self.subscribe(&[Channel::Name(ChannelType::Heartbeat)]).await

    }

    /// Subscribe a Conduit to the Coinbase WS endpoint.
    pub async fn subscribe(& mut self, channels: &[Channel]) {
        let _subscribe = Subscribe {
            _type: SubscribeCmd::Subscribe,
            channels: channels.to_vec(),
            auth: self._auth()
        };

        let subscribe = serde_json::to_string(&_subscribe).unwrap();
        self.websocket.lock().await.send(TMessage::Text(subscribe)).await.unwrap();
        debug!("Subscription sent");
   }


    /// **Core Requests**
    ///
    ///
    ///

    async fn _request(&self, method: reqwest::Method, path: &str, body: Option<String>) {
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
            let sign = self.sign(timestamp, method, path, &_body);
            let creds = self.credentials.as_ref().unwrap();
            req = req.header("CB-ACCESS-KEY", &creds.key)
                .header("CB-ACCESS-SIGN", sign.unwrap())
                .header("CB-ACCESS-PASSPHRASE", &creds.passphrase)
                .header("CB-ACCESS-TIMESTAMP", &timestamp.to_string());
            req = req.body(_body);
        }
        self._handle_http_request(req.build().unwrap()).await;
    }

    async fn _handle_http_request(&self, req: Request) {
        let resp = self.client.execute(req).await;
        let msg = if resp.is_err() {
            Message::InternalError(CBProError::Http(resp.err().unwrap().to_string()))
        } else {
            convert_http(resp.unwrap()).await
        };
        match self.sender.unbounded_send(msg) {
            Ok(_) => {},
            Err(e) => {println!("Conduit._request error: {:?}", e);}
        };
    }

    async fn _get(&self, uri: &str) { self._request(reqwest::Method::GET, uri, None).await; }
    async fn _post(&self, uri: &str, body: Option<String>) {
        self._request(reqwest::Method::POST, uri, body).await;
    }


    /// **Core Requests**
    ///
    ///
    ///
    pub async fn products(&self) { self._get("/products").await; }
    pub async fn time(&self) {
        self._get("/time").await;
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


async fn _handle_ws_resp(sender: UnboundedSender<Message>, r: Result<TMessage, TError>) {
    let msg = match r {
        Ok(tmsg) => Some(convert_ws(tmsg)),
        Err(e) => {
            debug!("_handle_ws_resp._handle_ws_msg: {_e}", _e= e);
            None
        }
    };
    if msg.is_some() {
        match sender.lock().unbounded_send(msg.unwrap()) {
            Ok(_) => {},
            Err(e) => debug!("_handle_ws_resp: {_e}", _e = e),
        };
    }
}

async fn convert_http(resp: reqwest::Response) -> Message {
    let txt = resp.text().await.unwrap();
    serde_json::from_str(txt.as_str()).unwrap_or_else(|e| {
        Message::InternalError(CBProError::Serde(e, txt))
    })
}

fn convert_ws(t_msg: TMessage) -> Message {
    match t_msg {
        TMessage::Text(msg) => serde_json::from_str(&msg).unwrap_or_else(|e| {
            Message::InternalError(CBProError::Serde(e, msg.into()))
        }),
        _ => unreachable!()
    }
}

