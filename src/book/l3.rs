
use async_std::{ sync::{ Receiver }};
use ordered_float::OrderedFloat;
use std::{ collections::BTreeMap,
           time::SystemTime };
use uuid::Uuid;

use super::*;
use crate::structs::*;

type SideBook = BTreeMap<OrderedFloat<f64>, f64>;

#[derive(Debug)]
pub struct BookRecord {
    pub id: uuid::Uuid,
    pub price: f64,
    pub size: f64,
}

#[derive(Debug)]
pub struct OrderBook {
    pub product_id: String,
    pub bid_book: SideBook,
    pub ask_book: SideBook,
    pub latest_time: SystemTime,
    pub bucket_size: f64
}

impl OrderBook {
    fn bucket(self, v: f64) -> f64 {
        (v / self.bucket_size).round() * self.bucket_size
    }
}
