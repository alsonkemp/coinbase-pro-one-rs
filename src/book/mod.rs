use crate::structs;
use async_std::sync::Receiver;

pub mod l2;
pub mod l3;
pub mod ticker;
const MAX_SIZE: usize = 1<<20; //1M
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    BidLessAsk,
    MatchUuid,
    Range,
    TestFail
}
