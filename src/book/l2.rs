
use async_std::{ pin::Pin,
                 stream::Stream,
                 sync::{ Receiver }};
use ordered_float::OrderedFloat;
use std::{ collections::BTreeMap };

use crate::structs::*;
use futures_util::stream::BoxStream;

/*
  WS LEVEL 2
  { "type": "snapshot", "product_id": "BTC-USD", "bids": [["10101.10", "0.45054140"]], "asks": [["10102.55", "0.57753524"]]}
  { "type": "l2update", "product_id": "BTC-USD", "time": "2019-08-14T20:42:27.265Z", "changes": [["buy", "10101.80000000", "0.162567"]]}
 */

#[derive(Debug)]
pub struct SideBook(BTreeMap<OrderedFloat<f64>, f64>);

/* It's not possible to remove orders if the order are bucketed. */
#[derive(Debug)]
pub struct OrderBook {
    pub product_id: String,
    pub bid_book: SideBook,
    pub ask_book: SideBook,
    pub latest_time: DateTime
}

impl SideBook {
    fn ingest(&mut self, price: f64, size: f64) {
        if size == 0.0 {
            self.0.remove(&OrderedFloat(price));
        } else {
            self.0.insert(OrderedFloat(price), size);
        }
    }
}

impl OrderBook {
    pub fn new(product_id: String) -> Self {
        Self {
            product_id,
            bid_book: SideBook(BTreeMap::new()),
            ask_book: SideBook(BTreeMap::new()),
            latest_time: now()
        }
    }

    pub fn match_product_id(&self, pid: &String) -> bool {
        self.product_id == *pid
    }

    pub async fn harvest(&mut self, msg: Option<Message>) -> Option<Message> {
        match msg {
            Some(Message::WSSnapshot{product_id, bids, asks}) => {
                    self.ingest_snapshot(product_id, bids, asks)
            },
            Some(Message::WSL2update{product_id, time, changes}) => {
                    self.ingest_updates(product_id, time, changes)
            },
            msg => {
                msg
            }
        }
    }

    fn ingest_snapshot(&mut self, product_id: String, bids: Vec<Level2SnapshotRecord>, asks: Vec<Level2SnapshotRecord>) -> Option<Message> {
        if !self.match_product_id(&product_id) {
            Some(Message::WSSnapshot {product_id, bids, asks})
        } else {
            let _ = bids.iter().map(|item| {
                self.bid_book.ingest(item.price, item.size);
            }).collect::<Vec<_>>();
            let _ = asks.iter().map(|item| {
                self.ask_book.ingest(item.price, item.size);
            }).collect::<Vec<_>>();
            self.latest_time = now();
            None
        }
    }

    fn ingest_updates(&mut self, product_id: String, time: DateTime, changes: Vec<Level2UpdateRecord>) -> Option<Message> {
        if !self.match_product_id(&product_id) {
            debug!("product_ids don't match: {:?} {:?}", self.product_id, product_id);
            Some(Message::WSL2update { product_id, time, changes })
        } else if self.latest_time > time {
            //Too early...
            None
        } else {
            let _ = changes.iter().map(|item| {
                if item.side == OrderSide::Buy {
                    self.bid_book.ingest(item.price, item.size);
                } else if item.side == OrderSide::Sell {
                    self.ask_book.ingest(item.price, item.size);
                }}).collect::<Vec<_>>();
            self.latest_time = now();
            None
        }
    }
}
