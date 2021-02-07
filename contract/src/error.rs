use std::io;

use crate::CID;

#[derive(Debug)]
pub enum Error {
  ArtCIDNotFound(CID),
}