use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameData {
  message_count: u64,
}

impl TryFrom<&[u8]> for GameData {
  type Error = Box<bincode::ErrorKind>;
  fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
    bincode::deserialize(value)
  }
}
impl TryFrom<&GameData> for Vec<u8> {
  type Error = Box<bincode::ErrorKind>;
  fn try_from(value: &GameData) -> Result<Self, Self::Error> {
    bincode::serialize(value)
  }
}

impl GameData {
  pub fn process_input(&mut self, _input: &str) -> String {
    self.message_count += 1;
    format!("{}", self.message_count)
  }
}
