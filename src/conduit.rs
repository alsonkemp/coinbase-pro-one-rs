#!feature(async_closure)]

use async_std::sync::{Arc, Mutex};
use async_std::{ task, stream::interval, sync::{ channel, Receiver, Sender }};
use async_tungstenite::{ async_std::{ connect_async },
                         tungstenite::{protocol::Message as TMessage }};
use crypto::{hmac::Hmac, mac::Mac};
use futures::{SinkExt, StreamExt};
use futures_util::{FutureExt};
use serde_json;
use std::{time::{ Duration, SystemTime, UNIX_EPOCH }};
use surf;

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

    pub fn sign(&self, method: surf::http_types::Method, uri: &str, body: Option<serde_json::Value>) -> (String, u64) {
        let ts = _timestamp();
        let secret = base64::decode(&self.credentials.clone().unwrap().secret).expect("base64::decode secret");
        let mut mac = Hmac::new(crypto::sha2::Sha256::new(), &secret);
        // Make the borrow_checker happy...
        let mut unwrapped_body: String = match body {
            None => String::from(""),
            Some(v) => v.to_string()
        };
        mac.input((ts.to_string() + method.as_ref() + uri + unwrapped_body.as_str()).as_bytes());
        (base64::encode(&mac.result().code()), ts)
    }

    fn auth(&self, method: surf::http_types::Method, path: &str, body: Option<serde_json::Value>) -> Option<Auth> {
        match &self.credentials {
            Some(c) => {
                debug!("Conduit: calculating auth...");
                let (signature, ts) = self.sign(method, path, body);
                Some(
                    Auth {
                        signature: signature,
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

    //////////////////////////////////////////////////////////////
    /// Subscribe a Conduit to the Coinbase WS endpoint.
    pub async fn subscribe(&mut self, channels: &[WSChannel]) {
        let subscribe = WSSubscribe {
            channels: channels.to_vec(),
            auth: self.auth(surf::http_types::Method::Get, "/users/self/verify", None)
        };

        let msg = Message::WSSubscribe(subscribe);
        debug!("Conduit: subscription: sending {:?}", serde_json::to_string(&msg).unwrap());
        self.to_websocket.lock().await.send(msg).await;
    }

    /// **Core Requests**
    ///
    ///
    ///
    async fn _request(&mut self,
                      method: surf::http_types::Method,
                      serde_type: &str,
                      path: &str,
                      body: Option<serde_json::Value>) {
        debug!("Conduit: _request: {:?} {:?} {:?} {:?}", &method, path, &body, serde_type);
        let mut req =
            surf::Request::new(
                method,
                url::Url::parse(&*format!("{}{}", self.base_http_uri, path)).unwrap()
            ).set_header("User-Agent", USER_AGENT);
        if body.is_some() {
            let auth = self.auth(method, path, body.clone()).unwrap();
            req = req.set_header("CB-ACCESS-KEY", auth.key)
               .set_header("CB-ACCESS-SIGN", auth.signature)
               .set_header("CB-ACCESS-PASSPHRASE", auth.passphrase)
               .set_header("CB-ACCESS-TIMESTAMP", auth.timestamp.to_string())
               .body_json(&body).unwrap();
        }
        // Can't yet convert to a `Message` because we haven't applied the serde_type yet...
        let resp = req.recv_string().await;

        let msg = if !(resp.is_err()) {
            let mut _resp = serde_json::from_str::<serde_json::Value>(resp.unwrap().as_str()).unwrap();
            _resp["type"] = serde_json::Value::String(serde_type.to_string());
            serde_json::from_value(_resp).unwrap_or_else(|e| {
                Message::InternalError(CBProError::Serde(e.to_string()))
            })
        } else {
            Message::InternalError(CBProError::Http(resp.err().unwrap().to_string()))
        };
        self.to_mailbox.lock().await.send(msg).await;
    }

    async fn _get(&mut self, serde_type: &str, uri: &str) {
        self._request(surf::http_types::Method::Get, serde_type, uri, Option::None).await;
    }
    // async fn _post(&mut self, _type: &str, uri: &str, body: Option<String>) {
    //     self._request(surf::http_types::Method::Post, uri, body, _type).await;
    // }


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

    pub async fn level(&mut self, level: Level) {
        debug!("Conduit: level...");
        match level {
            Level::Level2 => self.subscribe(
                        &[WSChannel::WithProduct {
                                name: WSChannelType::Level2,
                                product_ids: vec!("BTC-USD".to_string())
                                }]
                            ).await,
            Level::Level3 => {
                panic!("NOT IMPLEMENTED...");
                self.subscribe(
                    &[WSChannel::WithProduct {
                        name: WSChannelType::Full,
                        product_ids: vec!("BTC-USD".to_string())
                    }]
                ).await;
            }
        };
    }

    pub async fn order_book(&mut self, level: Level) {
        debug!("Conduit: order_book sent...");
        self._get("products", "/products").await;
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
fn handle_websocket(ws_uri:      &'static str,
                    to_mailbox:  Arc<Mutex<Sender<Message>>>,
                    inbox:       Arc<Mutex<Receiver<Message>>>) {
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
                            trace!("handle_incoming: {:?}\n", &msg);
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
fn _timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("leap-second")
        .as_secs()
}

