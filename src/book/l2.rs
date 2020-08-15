
use async_std::{ sync::{ Receiver }};
use ordered_float::OrderedFloat;
use std::{ collections::BTreeMap,
           fmt,
           ops::RangeInclusive };
use uuid::Uuid;

use crate::structs::*;

/*
  WS LEVEL 2
  { "type": "snapshot", "product_id": "BTC-USD", "bids": [["10101.10", "0.45054140"]], "asks": [["10102.55", "0.57753524"]]}
  { "type": "l2update", "product_id": "BTC-USD", "time": "2019-08-14T20:42:27.265Z", "changes": [["buy", "10101.80000000", "0.162567"]]}
 */

type SideBook = BTreeMap<OrderedFloat<f64>, f64>;

/* It's not possible to remove orders if the order are bucketed. */
#[derive(Debug)]
pub struct OrderBook {
    pub product_id: String,
    pub bid_book: SideBook,
    pub ask_book: SideBook,
}

impl OrderBook {
    pub fn new(product_id: String, bucket_size: f64) -> Self {
        Self {
            product_id,
            bid_book: BTreeMap::new(),
            ask_book: BTreeMap::new(),
        }
    }

    pub fn match_product_id(&self, pid: &String) -> bool {
        self.product_id == pid
    }

    pub async fn harvest(&mut self, strm: &Receiver<Message>) -> Option<Message> {
        println!("### HARVEST MAP ###");
        match strm.recv().await.unwrap() {
            Message::WSLevel2(WSLevel2::Snapshot {product_id: pid, bids, asks}) => {
                    self.ingest_snapshot()
            },
            Message::WSLevel2(WSLevel2::L2update {product_id, changes}) => {
                    self.ingest_update(WSLevel2::L2update { product_id, changes })
            },
            msg => {
                debug!("BOOK: default: {:?}", msg);
                Some(msg)
            }
        }
    }

    fn ingest_snapshot(&mut self, snapshot: WSLevel2::Snapshot) -> Option<Message>{
        let WSLevel2::Snapshot {product_id, bids, asks} = snapshot;
        if !self.match_product_id(&product_id) {
            Some(Message::WSLevel2(WSLevel2::Snapshot {product_id, bids, asks}))
        } else {
            bids.iter().map(|item| {
                self._ingest_snapshot(&mut self.bid_book, item);
            });
            None
        }
    }
    fn _ingest_snapshot(&mut self, book: &mut SideBook, item: &Level2SnapshotRecord) {
        if (item.size == 0.0) {
            book.remove(&OrderedFloat(item.price));
        }
        self.bid_book.insert(OrderedFloat(item.price), item.size);
    }

    fn ingest_update(&mut self, update: WSLevel2::L2update {}) -> Option<Message> {
        if !self.match_product_id(&product_id) {
            Some(Message::WSLevel2(WSLevel2::L2update { product_id, changes }))
        } else {
            bids.iter().map(|item| {
                self._ingest_update(item);
            });
            None
        }
    }

    fn _ingest_update(&mut self, item: &Level2UpdateRecord) {
        item.
        if (item.size == 0.0) {
            self.bid_book.remove(&OrderedFloat(item.price));
        }
        self.bid_book.insert(OrderedFloat(item.price), item.size);
    }

//     /// get current bid
//     pub fn bid(&self) -> Option<f64> {
//         if self.bid == std::usize::MIN {
//             None
//         } else {
//             Some(self.bid as f64 / 100.0)
//         }
//     }
//
//     /// get current ask
//     pub fn ask(&self) -> Option<f64> {
//         if self.ask == std::usize::MAX {
//             None
//         } else {
//             Some(self.ask as f64 / 100.0)
//         }
//     }
//
//     /// get last match
//     pub fn __match(&self) -> Option<f64> {
//         if self._match == 0 {
//             None
//         } else {
//             Some(self._match as f64 / 100.0)
//         }
//     }
//
//
//     fn side(&self, range: RangeInclusive<usize>) -> Vec<f64> {
//         self.book[range].iter()
//             .map(|x| x.iter().map(|x| x.0).sum())
//             .collect::<Vec<_>>()
//     }
//
//     /// get size of top sz bids (includes empty)
//     pub fn bids(&self, sz: usize) -> Vec<f64> {
//         self.side((self.bid + 1 - sz)..=self.bid)
//     }
//
//     /// get size of low sz bids (includes empty)
//     pub fn asks(&self, sz: usize) -> Vec<f64> {
//         self.side(self.ask..=self.ask+sz-1)
//     }
//
//     /// reload OrderBook from full bids and asks L3
//     pub fn reload(&mut self, bids: Vec<BookRecord>, asks: Vec<BookRecord>) -> Result<()> {
//         self.bid = std::usize::MIN;
//         self.ask = std::usize::MAX;
//         self.book.iter_mut().map(|x| *x = VecDeque::new()).count();
//
//         bids.into_iter()
//             .try_for_each(|rec| self.open(Side::Buy, rec))?;
//         asks.into_iter()
//             .try_for_each(|rec| self.open(Side::Sell, rec))?;
//         Ok(())
//     }
//
//     fn get_idx(&self, price: f64) -> Result<usize> {
//         let p_idx = (price * 100.0).round() as usize;
//         if p_idx >= self.book.len() {
//             Err(Error::Range)
//         } else {
//             Ok(p_idx)
//         }
//     }
//
//     /// open order
//     pub fn open(&mut self, side: Side, rec: BookRecord) -> Result<()> {
//         let p_idx = self.get_idx(rec.price)?;
//         match side {
//             Side::Buy if p_idx > self.bid => self.bid = p_idx,
//             Side::Sell if p_idx < self.ask => self.ask = p_idx,
//             _ => (),
//         }
//         assert!(self.bid < self.ask, "bid >= ask ({} >= {}) on {}", self.bid, self.ask, rec.id);
//         self.book[p_idx].push_back((rec.size, rec.id));
//         Ok(())
//     }
//
//     /// match order
//     pub fn _match(&mut self, price: f64, size: f64, id: Uuid) -> Result<()> {
//         let p_idx = self.get_idx(price)?;
//
//         if self.book[p_idx].is_empty() || id != self.book[p_idx][0].1 {
//             return Err(Error::MatchUuid);
//         }
//         let sz_round = {
//             let sz = &mut self.book[p_idx][0].0;
//             *sz -= size;
//             (*sz * 100.0).round()
//         };
//         if sz_round == 0.0 {
//             self.book[p_idx].pop_front();
//             self.check_ask_bid(p_idx);
//         }
//         self._match = p_idx;
//         Ok(())
//     }
//
//     /// done order
//     pub fn done(&mut self, price: f64, id: Uuid) -> Result<()> {
//         let p_idx = self.get_idx(price)?;
//         self.book[p_idx].retain(|&(_, it_id)| it_id != id);
//         self.check_ask_bid(p_idx);
//         Ok(())
//     }
//
//     /// change order
//     pub fn change(&mut self, price: f64, new_size: f64, id: Uuid) -> Result<()> {
//         let p_idx = self.get_idx(price)?;
//         if new_size == 0.0 {
//             self.done(price, id).unwrap_or_default();
//         } else {
//             self.book[p_idx].iter_mut().for_each(|(it_size, it_id)| {
//                 if *it_id == id {
//                     *it_size = new_size;
//                 }
//             })
//         }
//         Ok(())
//     }
//
//     fn check_ask_bid(&mut self, p_idx: usize) {
//         if p_idx == self.bid {
//             while self.book[self.bid].len() == 0 {
//                 self.bid -= 1;
//             }
//         }
//
//         if p_idx == self.ask {
//             while self.book[self.ask].len() == 0 {
//                 self.ask += 1;
//             }
//         }
//     }
//
//     /// open test order
//     pub fn open_test(&mut self, side: Side, price: f64) -> Result<()> {
//         Self::open(self, side, BookRecord{price, size:0.001, id: Uuid::nil()})
//     }
//
//     /// done test order
//     pub fn done_test(&mut self, price: f64) -> Result<()> {
//         Self::done(self, price, Uuid::nil())
//     }
//
//     /// test is test order works
//     pub fn test_order(&mut self, side: Side, price: f64) -> Result<()> {
//         let bid_or_ask = match side {
//             Side::Buy => self.bid,
//             Side::Sell => self.ask
//         };
//         let p_idx = self.get_idx(price)?;
//         if p_idx == bid_or_ask {
//             Self::_match(self, price, 0.001, Uuid::nil())
//         } else {
//             Err(Error::TestFail)
//         }
//     }
//
// }
//
// impl fmt::Display for OrderBook {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         if self.bid == std::usize::MIN || self.ask == std::usize::MAX {
//             return write!(f, "OB: empty");
//         }
//         let size = 20;
//
//         let round_lambda = |x: f64| (x*1000.0).round()/1000.0;
//
//         let bids = self.bids(size).into_iter().map(round_lambda).map(|x| x.to_string()).collect::<Vec<_>>().join(",");
//         let asks = self.asks(size).into_iter().map(round_lambda).map(|x| x.to_string()).collect::<Vec<_>>().join(",");
//         let bid = self.bid as f64 / 100.0;
//         let ask = self.ask as f64 / 100.0;
//         write!(f, "OB: {} | {:.2}   {:.2} | {}", bids, bid, ask, asks)
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_display() {
//         let mut ob = OrderBook::new();
//         ob.reload(
//             vec![
//                 BookRecord {
//                     price: 3994.96,
//                     size: 0.3,
//                     id: Uuid::new_v4(),
//                 },
//                 BookRecord {
//                     price: 3995.0,
//                     size: 0.5,
//                     id: Uuid::new_v4(),
//                 },
//             ],
//             vec![
//                 BookRecord {
//                     price: 4005.0,
//                     size: 0.4,
//                     id: Uuid::new_v4(),
//                 },
//                 BookRecord {
//                     price: 4005.02,
//                     size: 0.2,
//                     id: Uuid::new_v4(),
//                 },
//             ],
//         ).unwrap_or_default();
//
//         ob.open(
//             Side::Buy,
//             BookRecord {
//                 price: 3994.96,
//                 size: 0.2,
//                 id: Uuid::new_v4(),
//             },
//         ).unwrap_or_default();
//
//         let str = format!("{}", ob);
//         assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.5,0,0,0,0.5 | 3995.00   4005.00 | 0.4,0,0.2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
//     }
//
//     #[test]
//     fn test_match() {
//         let mut ob = OrderBook::new();
//         let id1 = Uuid::new_v4();
//         let id2 = Uuid::new_v4();
//         ob.reload(
//             vec![
//                 BookRecord {
//                     price: 3994.96,
//                     size: 0.3,
//                     id: id1,
//                 },
//                 BookRecord {
//                     price: 3995.0,
//                     size: 0.5,
//                     id: id2,
//                 },
//             ],
//             vec![
//                 BookRecord {
//                     price: 4005.0,
//                     size: 0.4,
//                     id: Uuid::new_v4(),
//                 },
//                 BookRecord {
//                     price: 4005.02,
//                     size: 0.2,
//                     id: Uuid::new_v4(),
//                 },
//             ],
//         ).unwrap_or_default();
//
//         ob.open(
//             Side::Buy,
//             BookRecord {
//                 price: 3994.96,
//                 size: 0.2,
//                 id: Uuid::new_v4(),
//             },
//         ).unwrap_or_default();
//         ob._match(3995.0, 0.3, id2).unwrap_or_default();
//         let str = format!("{}", ob);
//         assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.5,0,0,0,0.2 | 3995.00   4005.00 | 0.4,0,0.2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
//
//         ob._match(3995.0, 0.2, id2).unwrap_or_default();
//         let str = format!("{}", ob);
//         assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.5 | 3994.96   4005.00 | 0.4,0,0.2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
//
//     }
//
//     #[test]
//     fn test_done() {
//         let mut ob = OrderBook::new();
//         let id1 = Uuid::new_v4();
//         let id2 = Uuid::new_v4();
//         ob.reload(
//             vec![
//                 BookRecord {
//                     price: 3994.96,
//                     size: 0.3,
//                     id: id1,
//                 },
//                 BookRecord {
//                     price: 3995.0,
//                     size: 0.5,
//                     id: id2,
//                 },
//             ],
//             vec![
//                 BookRecord {
//                     price: 4005.0,
//                     size: 0.4,
//                     id: Uuid::new_v4(),
//                 },
//                 BookRecord {
//                     price: 4005.02,
//                     size: 0.2,
//                     id: Uuid::new_v4(),
//                 },
//             ],
//         ).unwrap_or_default();
//
//         ob.done(3994.96, id1).unwrap_or_default();
//
//         let str = format!("{}", ob);
//         assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.5 | 3995.00   4005.00 | 0.4,0,0.2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
//     }
//
//     #[test]
//     fn test_round_output() {
//         let mut ob = OrderBook::new();
//         ob.reload(
//             vec![
//                 BookRecord {
//                     price: 3995.0,
//                     size: 0.3,
//                     id: Uuid::new_v4(),
//                 }
//             ],
//             vec![
//                 BookRecord {
//                     price: 4005.0,
//                     size: 1.0 / 3.0,
//                     id: Uuid::new_v4(),
//                 },
//             ],
//         ).unwrap_or_default();
//
//         let str = format!("{}", ob);
//         assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.3 | 3995.00   4005.00 | 0.333,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
//     }

}
