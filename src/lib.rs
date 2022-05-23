use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameData {
  _priv: (),
}

impl TryFrom<&'_ [u8]> for GameData {
  type Error = Box<bincode::ErrorKind>;
  fn try_from(value: &'_ [u8]) -> Result<Self, Self::Error> {
    bincode::deserialize(value)
  }
}
impl TryFrom<&'_ GameData> for Vec<u8> {
  type Error = Box<bincode::ErrorKind>;
  fn try_from(value: &'_ GameData) -> Result<Self, Self::Error> {
    bincode::serialize(value)
  }
}

impl GameData {
  pub fn new() -> Self {
    Self { _priv: () }
  }
}
