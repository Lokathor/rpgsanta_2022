#![allow(unused_imports)]

use rpgsanta_2022::GameData;
use std::{
  io::{stdin, stdout, BufRead, Read, Write},
  path::{Path, PathBuf},
};

fn main() {
  let bytes = match load_profile_bytes() {
    Ok(bytes) => bytes,
    Err(why) => {
      println!("Couldn't read profile save file: {why:?}");
      Vec::default()
    }
  };
  let mut game = match GameData::try_from(bytes.as_ref()) {
    Ok(game) => game,
    Err(why) => {
      println!("Couldn't parse save file: {why:?}");
      GameData::default()
    }
  };
  //
  let mut stdin_lock = stdin().lock();
  let mut stdout_lock = stdout().lock();
  let mut in_buf = String::new();
  //
  loop {
    stdout_lock.write(b"$ ").ok();
    stdout_lock.flush().ok();
    stdin_lock.read_line(&mut in_buf).ok();
    in_buf.pop();
    //
    let response = game.process_input(&in_buf);
    stdout_lock.write(response.as_bytes()).ok();
    stdout_lock.write(b"\n\n").ok();
    stdout_lock.flush().ok();
    in_buf.clear();
    let profile_bytes = match Vec::<u8>::try_from(&game) {
      Ok(bytes) => bytes,
      Err(why) => {
        println!("Couldn't serialize profile data: {why}");
        return;
      }
    };
    store_profile_bytes(&profile_bytes).unwrap();
  }
}

fn save_path() -> PathBuf {
  Path::new("save_data").join("local_save.data")
}
fn load_profile_bytes() -> std::io::Result<Vec<u8>> {
  std::fs::read(save_path())
}
fn store_profile_bytes(bytes: &[u8]) -> std::io::Result<()> {
  let p = save_path();
  std::fs::create_dir_all(p.parent().unwrap_or(Path::new(""))).ok();
  std::fs::write(p, bytes)
}
