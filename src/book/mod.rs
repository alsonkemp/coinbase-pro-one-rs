use crate::structs;
use std::borrow::BorrowMut;

pub mod l2;
pub mod l3;
pub mod ticker;

#[derive(Debug)]
pub enum Error {
    BidLessAsk,
    MatchUuid,
    Range,
    TestFail
}

pub fn harvest(msg: structs::Message, harvesters: Vec<&mut Box<dyn MsgHarvester>>) -> Option<structs::Message> {
    harvesters.into_iter().fold(Some(msg), |msg, mut h| {
        match msg {
            Some(m) => { h.harvest(m) },
            None => None
        }
    })
}

pub trait MsgHarvester {
    fn harvest(&mut self, msg: structs::Message) -> Option<structs::Message>;
}
