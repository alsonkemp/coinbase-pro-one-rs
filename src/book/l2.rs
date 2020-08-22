use ordered_float::OrderedFloat;
use std::{ collections::BTreeMap };

use crate::structs;


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
    pub latest_time: structs::DateTime
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
            latest_time: structs::now()
        }
    }

    pub fn match_product_id(&self, pid: &String) -> bool {
        self.product_id == *pid
    }


    fn ingest_snapshot(&mut self,
                       product_id: String,
                       bids: Vec<structs::Level2SnapshotRecord>,
                       asks: Vec<structs::Level2SnapshotRecord>) -> Option<structs::Message> {
        if !self.match_product_id(&product_id) {
            Some(structs::Message::WSSnapshot {product_id, bids, asks})
        } else {
            let _ = bids.iter().map(|item| {
                self.bid_book.ingest(item.price, item.size);
            }).collect::<Vec<_>>();
            let _ = asks.iter().map(|item| {
                self.ask_book.ingest(item.price, item.size);
            }).collect::<Vec<_>>();
            None
        }
    }

    fn ingest_updates(&mut self,
                      product_id: String,
                      time: structs::DateTime,
                      changes: Vec<structs::Level2UpdateRecord>) -> Option<structs::Message> {
        if !self.match_product_id(&product_id) {
            debug!("product_ids don't match: {:?} {:?}", self.product_id, product_id);
            Some(structs::Message::WSL2update { product_id, time, changes })
        } else if self.latest_time > time {
            //Too early...
            debug!("Update too early... {:?}", time);
            None
        } else {
            let _ = changes.iter().map(|item| {
                if item.side == structs::OrderSide::Buy {
                    self.bid_book.ingest(item.price, item.size);
                } else if item.side == structs::OrderSide::Sell {
                    self.ask_book.ingest(item.price, item.size);
                }}).collect::<Vec<_>>();
            self.latest_time = time;
            None
        }
    }
}

impl super::MsgHarvester for OrderBook {
    fn harvest(&mut self, msg: structs::Message) -> Option<structs::Message> {
        match msg {
            structs::Message::WSSnapshot{product_id, bids, asks} => {
                self.ingest_snapshot(product_id, bids, asks)
            },
            structs::Message::WSL2update{product_id, time, changes} => {
                self.ingest_updates(product_id, time, changes)
            },
            msg => {
                Some(msg)
            }
        }
    }
}
