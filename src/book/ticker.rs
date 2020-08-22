use crate::structs;

//{ best_ask: 11920.0, best_bid: 11919.99, last_size: 0.00142629, price: 11920.0,
//  product_id: "BTC-USD", sequence: 15773761736, side: Buy, time: 2020-08-17T11:58:19.609702Z, trade_id: 100359154 })
pub struct Ticker {
    product_id: String,
    best_ask: f64,
    best_bid: f64,
    last_size: f64,
    price: f64,
    sequence: usize,
    side: structs::OrderSide,
    time: structs::DateTime,
    trade_id: usize
}
impl Ticker {
    pub fn new(product_id: String) -> Self {
        Self {
            product_id,
            best_ask: 0.0,
            best_bid: 0.0,
            last_size: 0.0,
            price: 0.0,
            sequence: 0,
            side: structs::OrderSide::Buy,
            time: structs::now(),
            trade_id: 0
        }
    }
}

impl super::MsgHarvester for Ticker {
    fn harvest(&mut self, msg: structs::Message) -> Option<structs::Message> {
        match msg {
            structs::Message::WSTicker(
                structs::WSTicker::Full { best_ask, best_bid, last_size, price, product_id, sequence, side, time, trade_id }) => {
                self.best_ask = best_ask;
                self.best_bid = best_bid;
                self.last_size = last_size;
                self.price = price;
                self.product_id = product_id;
                self.sequence = sequence;
                self.side = side;
                self.time = time;
                self.trade_id = trade_id;
                None
            },
            structs::Message::WSTicker(
                structs::WSTicker::Empty { sequence, product_id, price }) => {
                self.sequence = sequence;
                self.product_id = product_id;
                self.price = price;
                debug!("Harvested ticker...");
                None
            }
            _ => Some(msg)
        }
    }
}
