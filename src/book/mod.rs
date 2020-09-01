use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::structs;
use crate::utils::*;

pub mod l2;
pub mod l3;
pub mod ticker;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Book<T> {
    pub sequence: usize,
    pub bids: Vec<T>,
    pub asks: Vec<T>,
}

pub trait BookLevel {
    fn level() -> u8;
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct BookRecordL1 {
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
    pub num_orders: usize,
}

impl BookLevel for BookRecordL1 {
    fn level() -> u8 {
        1
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct BookRecordL2 {
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
    pub num_orders: usize,
}

impl BookLevel for BookRecordL2 {
    fn level() -> u8 {
        2
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct BookRecordL3 {
    #[serde(deserialize_with = "f64_from_string")]
    pub price: f64,
    #[serde(deserialize_with = "f64_from_string")]
    pub size: f64,
    pub order_id: Uuid,
}

impl BookLevel for BookRecordL3 {
    fn level() -> u8 {
        3
    }
}

#[derive(Debug)]
pub enum Error {
    BidLessAsk,
    MatchUuid,
    Range,
    TestFail
}

pub trait MsgHarvester {
    fn harvest(&mut self, msg: structs::Message) -> Option<structs::Message>;
}
pub fn harvest(msg: structs::Message, harvesters: Vec<&mut Box<dyn MsgHarvester>>) -> Option<structs::Message> {
    harvesters.into_iter().fold(Some(msg), |msg, mut h| {
        match msg {
            Some(m) => { h.harvest(m) },
            None => None
        }
    })
}

