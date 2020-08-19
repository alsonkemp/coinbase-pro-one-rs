#!feature(async_closure)]

use async_std::sync::{Arc, Mutex};
use async_std::{ task, stream::interval, sync::{ channel, Receiver, Sender }};
use async_tungstenite::{ async_std::{ connect_async },
                         tungstenite::{protocol::Message as TMessage }};
use crypto::{hmac::Hmac, mac::Mac};
use futures::{SinkExt, StreamExt};
use futures_util::{FutureExt};
use reqwest::blocking::{Client};
use serde_json;
use serde_json::Value;
use std::{time::{ Duration, SystemTime, UNIX_EPOCH }};

// LOCAL IMPORTS
use crate::structs::*;
use crate::errors::{CBProError};

const CHANNEL_SIZE: usize = 128;
const USER_AGENT: &str = concat!("coinbase-pro-one-rs/", env!("CARGO_PKG_VERSION"));

pub struct Conduit<'a> {
    last_time: u64,
    // The Coinbase REST endpoint.  Needed for all RESTy methods.
    base_http_uri: &'a str,
    // Coinbase credentials.
    credentials: Option<Credentials>,
    // To communicate with the 'user' of the library
    to_mailbox:   Arc<Mutex<Sender<Message>>>,

    to_websocket: Arc<Mutex<Sender<Message>>>,
}

impl Conduit<'static> {
    /// Creates a new Conduit
    pub async fn new(http_uri: &'static str, ws_uri: &'static str, _creds: Option<Credentials>)
                     -> (Conduit<'static>, Receiver<Message>) {
        debug!("Conduit.new: {:?} {:?} {:?}", http_uri, ws_uri, _creds);
        // Creates a new Conduit
        let credentials = if _creds.is_some() {
            let creds = _creds.unwrap();
            Some(Credentials {
                key: creds.key,
                secret: creds.secret,
                passphrase: creds.passphrase
            })
        } else {
            Option::None
        };

        let (_to_mailbox, mailbox) = channel::<Message>(CHANNEL_SIZE);
        let to_mailbox = Arc::new(Mutex::new(_to_mailbox));
        let (to_websocket, __to_websocket) = channel::<Message>(CHANNEL_SIZE);
        let _to_websocket = Arc::new(Mutex::new(__to_websocket));
        debug!("ConduitWebsocket: starting...");
        handle_websocket(
            ws_uri,
           to_mailbox.clone(),
           Arc::clone(&_to_websocket));

        (Self {
            last_time: _timestamp(),
            base_http_uri: http_uri,
            credentials: credentials.clone(),
            to_mailbox,
            to_websocket: Arc::new(Mutex::new(to_websocket)),
        }, mailbox)
    }

    pub fn sign(&self, ts: u64, method: reqwest::Method, uri: &str, body: &str) -> Option<String> {
        if self.credentials.is_none() {
            Option::None
        } else {
            let key = base64::decode(self.credentials.clone().unwrap().secret)
                .expect("base64::decode secret");
            let mut mac = Hmac::new(crypto::sha2::Sha256::new(), &key);
            mac.input((ts.to_string() + method.as_str() + uri + body).as_bytes());
            Some(base64::encode(&mac.result().code()))
        }
    }

    //////////////////////////////////////////////////////////////
    /// Subscribe a Conduit to the Coinbase WS endpoint.
    pub async fn subscribe(&mut self, channels: &[WSChannel]) {
        let subscribe = WSSubscribe {
            channels: channels.to_vec(),
            auth: _auth(self.credentials.clone())
        };

        let msg = Message::WSSubscribe(subscribe);
        debug!("Conduit: subscription: sending {:?}", serde_json::to_string(&msg).unwrap());
        self.to_websocket.lock().await.send(msg).await;
    }

    /// **Core Requests**
    ///
    ///
    ///
    async fn _request(&mut self, method: reqwest::Method, path: &str, body: Option<String>, _type: &str) {
        debug!("Conduit: _request: {:?} {:?} {:?}", &method, &path, &body);
        let mut req = Client::new()
            .request(method.clone(), &format!("{}{}", self.base_http_uri, path))
            .header("User-Agent", USER_AGENT)
            .header("Content-Type", "Application/JSON");
        if body.is_some() {
            let _body = body.unwrap();
            let timestamp = _timestamp();
            let sign = _sign(self.credentials.clone(), timestamp, method, path, &_body);
            let creds = self.credentials.clone().unwrap();
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
            let _resp = resp.unwrap().text().unwrap();
            debug!("RESP: {:?}", &_resp);
            convert_http(_resp, _type).await
        };
        self.to_mailbox.lock().await.send(msg).await;
    }

    async fn _get(&mut self, _type: &str, uri: &str) {
        self._request(reqwest::Method::GET, uri, Option::None, _type).await;
    }
    async fn _post(&mut self, _type: &str, uri: &str, body: Option<String>) {
        self._request(reqwest::Method::POST, uri, body, _type).await;
    }


    /// **Core Requests**
    ///
    ///
    ///
    pub async fn heartbeat(&mut self) {
        debug!("Conduit: heartbeat...");
        self.subscribe(&[WSChannel::WithProduct {
            name: WSChannelType::Heartbeat,
            product_ids: vec!("BTC-USD".to_string())
        }]
        ).await
    }

    pub async fn level2(&mut self) {
        debug!("Conduit: level2...");
        self.subscribe(
            &[WSChannel::WithProduct {
                name: WSChannelType::Level2,
                product_ids: vec!("BTC-USD".to_string()) }]
        ).await
    }

    pub async fn products(&mut self) {
        debug!("Conduit: products sent...");
        self._get("products", "/products").await;
    }

    pub async fn status(&mut self) {
        debug!("Conduit: status...");
        self.subscribe(&[WSChannel::Name(WSChannelType::Status)]).await;
    }

    pub async fn ticker(&mut self, product_ids: Vec<String>) {
        debug!("Conduit: ticker...");
        self.subscribe(
            &[WSChannel::WithProduct { name: WSChannelType::Ticker, product_ids }]
        ).await;
    }

    pub async fn time(&mut self) {
        debug!("Conduit: time sent...");
        self._get("time", "/time").await;
    }
    pub fn interval(&mut self, millis: u64) {
        let to_mailbox = self.to_mailbox.clone();
        task::spawn((|| async move{
            interval(Duration::from_millis(millis)).for_each(|_| async {
                to_mailbox.lock().await.send(Message::Interval(now())).await;
            }).await;
        })());
    }
}


//////////////////////////////////////////////////////////////////////////
// Websocket
//////////////////////////////////////////////////////////////////////////
fn handle_websocket(ws_uri:   &'static str,
                          to_mailbox:     Arc<Mutex<Sender<Message>>>,
                          inbox:          Arc<Mutex<Receiver<Message>>>) {
    task::spawn((|| async move{
        let (_ws, _) = connect_async(ws_uri)
            .await
            .expect("Failed on WS connect");
        debug!("Conduit: WebSocket handshake has been successfully completed...");
        let (mut _ws_write, mut _ws_read) = _ws.split();
        let ws_write = Arc::new(Mutex::new(_ws_write));
        let ws_read = Arc::new(Mutex::new(_ws_read));
        loop {
            futures::future::select(
                // Incoming
                (|| async {
                    let tungstenite_msg = ws_read.lock().await.next().await;
                    let conduit_msg = match tungstenite_msg.unwrap() {
                        Ok(TMessage::Text(msg)) => {
                            serde_json::from_str(&msg).unwrap_or_else(|e| {
                                warn!("Can't decode: {:?}", msg);
                                Message::InternalError(CBProError::Serde(e.to_string()))
                            })},
                        o => {
                            debug!("Tungstenite unwrap error: {:?}\n", o);
                            Message::None
                        }
                    };
                    match &conduit_msg {
                        Message::InternalError(e) => {
                            warn!("InternalError: {:?}", e)
                        },
                        _ => {
                            to_mailbox.lock().await.send(conduit_msg).await
                        }
                    }
                })().boxed(),
                // Outgoing
                (|| async {
                    match Arc::clone(&inbox).lock().await.recv().await {
                        Ok(msg) => {
                            debug!("handle_outgoing: {:?}\n", &msg);
                            let smsg = serde_json::to_string(&msg).unwrap();
                            match ws_write.lock().await.send(TMessage::Text(smsg)).await {
                                Ok(v)  => {println!("OK {:?}", v);},
                                Err(e) => {println!("ERR {:?}", e);}
                            };
                        },
                        Err(e) => {
                            error!("handle_outgoing: Error: {:?}\n", e.to_string());
                        }
                    }
                })().boxed()
            ).await;
        }
    })());
}

//////////////////////////////////////////////////////////////////////////
// HELPERS
//////////////////////////////////////////////////////////////////////////
async fn convert_http(txt: String, _type: &str) -> Message {
    let mut v: Value = serde_json::from_str(txt.as_str()).unwrap();
    v["type"] = serde_json::Value::String(_type.to_string());
    serde_json::from_value(v).unwrap_or_else(|e| {
        Message::InternalError(CBProError::Serde(e.to_string()))
    })
}


fn _timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("leap-second")
        .as_secs()
}

fn _sign(credentials: Option<Credentials>, ts: u64, method: reqwest::Method, uri: &str, body: &str) -> Option<String> {
    if credentials.is_none() {
        Option::None
    } else {
        let key = base64::decode(credentials.clone().unwrap().secret)
            .expect("base64::decode secret");
        let mut mac = Hmac::new(crypto::sha2::Sha256::new(), &key);
        mac.input((ts.to_string() + method.as_str() + uri + body).as_bytes());
        Some(base64::encode(&mac.result().code()))
    }
}

fn _auth(credentials: Option<Credentials>) -> Option<Auth> {
    match credentials.clone() {
        Some(c) => {
            debug!("Conduit: calculating auth...");
            let ts = _timestamp();
            let signature = _sign(
                credentials,
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
            debug!("Conduit: **not** calculating auth... ");
            None
        }
    }
}
