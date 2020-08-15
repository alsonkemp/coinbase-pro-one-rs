
use async_std::{ sync::{ Receiver }};
use ordered_float::OrderedFloat;
use std::{ collections::BTreeMap,
           fmt,
           ops::RangeInclusive };
use uuid::Uuid;

use super::*;
use crate::structs::*;

type _Book = BTreeMap<OrderedFloat<f64>, f64>;

#[derive(Debug)]
pub struct BookRecord {
    pub id: uuid::Uuid,
    pub price: f64,
    pub size: f64,
}


fn bucket(self, v: f64) -> f64 {
    (v / self.bucket_size).round() * self.bucket_size
}
