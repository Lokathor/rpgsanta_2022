#![cfg_attr(test, feature(is_sorted))]

use std::{fmt::Write, num::NonZeroU32};

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RoomID(NonZeroU32);
impl core::fmt::Debug for RoomID {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let bytes = self.0.get().to_be_bytes();
    f.write_str("RoomID(")?;
    for byte in bytes.iter().copied() {
      f.write_char(byte as char)?;
    }
    f.write_str(")")
  }
}
impl Default for RoomID {
  fn default() -> Self {
    room_id("")
  }
}
pub const fn room_id(s: &str) -> RoomID {
  match NonZeroU32::new(u32::from_be_bytes(match s.as_bytes() {
    [] => [b'.', b'.', b'.', b'.'],
    [a] => [*a, b'.', b'.', b'.'],
    [a, b] => [*a, *b, b'.', b'.'],
    [a, b, c] => [*a, *b, *c, b'.'],
    [a, b, c, d] => [*a, *b, *c, *d],
    _ => panic!("input too long!"),
  })) {
    Some(nz) => RoomID(nz),
    None => panic!(),
  }
}

type StrLit = &'static str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Room {
  pub id: RoomID,
  pub name: StrLit,
  pub description: StrLit,
}
impl Default for Room {
  fn default() -> Self {
    Room::DEFAULT
  }
}
impl Room {
  pub const DEFAULT: Room =
    Room { id: room_id("DEAD"), name: "Default", description: "Default" };
}

pub const ROOM_DB: &[Room] = &[
  Room { id: room_id("c_H8"), name: "Shrine of Resurrection", ..Room::DEFAULT },
  Room { id: room_id("d101"), name: "entry", ..Room::DEFAULT },
  Room { id: room_id("d102"), name: "deadend", ..Room::DEFAULT },
  Room { id: room_id("d103"), name: "deadend", ..Room::DEFAULT },
  Room { id: room_id("d104"), name: "hall-north", ..Room::DEFAULT },
  Room { id: room_id("d105"), name: "room", ..Room::DEFAULT },
  Room { id: room_id("d106"), name: "room", ..Room::DEFAULT },
  Room { id: room_id("d107"), name: "room", ..Room::DEFAULT },
  Room { id: room_id("d108"), name: "room", ..Room::DEFAULT },
  Room { id: room_id("d109"), name: "room", ..Room::DEFAULT },
  Room { id: room_id("d110"), name: "room", ..Room::DEFAULT },
  Room { id: room_id("d111"), name: "hall", ..Room::DEFAULT },
  Room { id: room_id("d112"), name: "hall-turn-west", ..Room::DEFAULT },
  Room { id: room_id("d113"), name: "stairs-down", ..Room::DEFAULT },
  Room { id: room_id("d114"), name: "mini-treasure?", ..Room::DEFAULT },
  Room { id: room_id("d115"), name: "sage?", ..Room::DEFAULT },
  Room { id: room_id("d116"), name: "hall-turn-north", ..Room::DEFAULT },
  Room { id: room_id("d117"), name: "boss-fight", ..Room::DEFAULT },
  Room { id: room_id("d118"), name: "the-cape", ..Room::DEFAULT },
  Room { id: room_id("w_H4"), name: "Zakros Isle", ..Room::DEFAULT },
  Room { id: room_id("w_H8"), name: "Firros", ..Room::DEFAULT },
  Room { id: room_id("w_I4"), name: "Baikal", ..Room::DEFAULT },
  Room { id: room_id("w_I5"), name: "Baikal", ..Room::DEFAULT },
  Room { id: room_id("w_I6"), name: "Torshavn", ..Room::DEFAULT },
  Room { id: room_id("w_I7"), name: "Torshavn", ..Room::DEFAULT },
  Room { id: room_id("w_I8"), name: "Torshavn", ..Room::DEFAULT },
];

#[test]
fn test_room_db_sorted() {
  assert!(ROOM_DB.is_sorted(), "ROOM_DB not sorted! Should be: {:?}", {
    let mut v = Vec::from(ROOM_DB);
    v.sort();
    v
  });
}

#[test]
fn test_all_room_ids_different() {
  use std::collections::HashSet;
  let mut set = HashSet::new();
  for room in ROOM_DB.iter() {
    set.insert(room.id);
  }
  assert_eq!(ROOM_DB.len(), set.len());
}
