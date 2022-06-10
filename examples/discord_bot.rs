#![allow(unused_imports)]

use rpgsanta_2022::GameData;
use serenity::{
  async_trait,
  http::Http,
  model::{
    channel::{Channel, Message},
    gateway::Ready,
    id::ChannelId,
  },
  prelude::*,
};
use std::{
  collections::{hash_map::Entry, HashMap},
  path::{Path, PathBuf},
  sync::Arc,
  time::Duration,
};
use tokio::{
  sync::{
    mpsc::{
      channel as mpsc_channel, Receiver as MspcReceiver, Sender as MspcSender,
    },
    RwLock, RwLockReadGuard, RwLockWriteGuard,
  },
  task::spawn as task_spawn,
  time::timeout,
};

macro_rules! log_err {
  ( $x:expr ) => {
    if let Err(why) = $x {
      let file = file!();
      let line = line!();
      println!("{file}:{line} ERROR: {why}");
    }
  };
}

type SessionsMap = Arc<RwLock<HashMap<ChannelId, MspcSender<String>>>>;

/// Manages text adventure sessions over discord.
///
/// Each channel of interaction is a single game session. What we store is a
/// mapping from ChannelID (a `u64`) to a Sender. Each matching Receiver is held
/// by an async task that holds the game state.
///
/// Our data invariant is this: any time you take a lock on the sessions then
/// * EITHER there is an id/sender in the map, in which case the session is
///   "live" and the async task has the latest version of the game data.
/// * OR there is not id/sender in the map, in which case the data on disk is
///   the latest version.
#[derive(Default)]
struct TextBot {
  sessions: SessionsMap,
}

#[inline]
async fn do_one_input(
  input: String, channel_id: ChannelId, game: &mut GameData, http: &Arc<Http>,
) {
  drop(channel_id.broadcast_typing(http).await);
  let response = game.process_input(input);
  //println!("{response}");
  log_err!(channel_id.say(http, response).await);
  let profile_bytes = match Vec::<u8>::try_from(&*game) {
    Ok(bytes) => bytes,
    Err(why) => {
      println!("Couldn't serialize profile data: {why}");
      return;
    }
  };
  log_err!(store_profile_bytes(channel_id, &profile_bytes));
}

async fn perform_game(
  channel_id: ChannelId, mut recver: MspcReceiver<String>, mut game: GameData,
  sessions: SessionsMap, http: Arc<Http>,
) {
  const LIMIT: Duration = Duration::new(60 * 10, 0);

  loop {
    match timeout(LIMIT, recver.recv()).await {
      Ok(Some(input)) => {
        do_one_input(input, channel_id, &mut game, &http).await
      }
      Ok(None) => {
        // This case means the channel was closed? This shouldn't be possible,
        // because no one else should be deleting our Sender from the
        // SessionsMap value.
      }
      Err(_) => {
        break;
      }
    }
  }

  let mut write_lock = sessions.write().await;
  recver.close();
  while let Some(input) = recver.recv().await {
    do_one_input(input, channel_id, &mut game, &http).await
  }
  write_lock.remove(&channel_id);
}

fn save_path_for_id(ChannelId(id): ChannelId) -> PathBuf {
  Path::new("save_data").join(format!("{id}.data"))
}
fn load_profile_bytes(channel_id: ChannelId) -> std::io::Result<Vec<u8>> {
  std::fs::read(save_path_for_id(channel_id))
}
fn store_profile_bytes(
  channel_id: ChannelId, bytes: &[u8],
) -> std::io::Result<()> {
  let p = save_path_for_id(channel_id);
  std::fs::create_dir_all(p.parent().unwrap_or(Path::new(""))).ok();
  std::fs::write(p, bytes)
}

#[async_trait]
impl EventHandler for TextBot {
  async fn ready(&self, _: Context, ready: Ready) {
    let current_user = ready.user;
    let current_name = current_user.name.as_str();
    let current_discriminator = current_user.discriminator;
    println!("Connected as {current_name}#{current_discriminator}!");
  }

  /// This is called for each message the bot sees.
  ///
  /// * When others speak it generates a message
  /// * When the bot speaks it gets events for *its own* messages
  /// * Events are handled async using a thread pool, so multiple messages can
  ///   be in flight at the same time.
  async fn message(&self, ctx: Context, msg: Message) {
    // Ignore all message events from bots, including our own.
    if msg.author.bot {
      return;
    }
    // Currently we only run games within private messages
    let game_is_allowed = match msg.channel(&ctx.http).await {
      Ok(Channel::Private(_)) => true,
      _ => false,
    };
    if !game_is_allowed {
      return;
    }

    //let author = msg.author;
    //let author_name = author.name.as_str();
    //let author_discriminator = author.discriminator;
    //let content = msg.content.as_str();
    //println!("{author_name}#{author_discriminator}$ {content}");

    let channel_id = msg.channel_id;
    let r = self.sessions.read().await;
    if let Some(sender) = r.get(&channel_id) {
      log_err!(sender.send(msg.content).await);
    } else {
      drop(r);
      match self.sessions.write().await.entry(channel_id) {
        Entry::Occupied(o) => {
          let sender = o.get();
          log_err!(sender.send(msg.content).await);
        }
        Entry::Vacant(v) => {
          let (sender, recver) = mpsc_channel(5);
          let ses = Arc::clone(&self.sessions);
          let http = Arc::clone(&ctx.http);
          let bytes = load_profile_bytes(channel_id).unwrap_or_default();
          let game = GameData::try_from(bytes.as_ref()).unwrap_or_default();
          task_spawn(perform_game(channel_id, recver, game, ses, http));
          log_err!(sender.send(msg.content).await);
          v.insert(sender);
        }
      }
    }
  }
}

#[tokio::main]
async fn main() {
  let token =
    std::env::var("DISCORD_TOKEN").expect("Expected a `DISCORD_TOKEN` value");

  let intents = GatewayIntents::GUILD_MESSAGES
    | GatewayIntents::DIRECT_MESSAGES
    | GatewayIntents::MESSAGE_CONTENT;

  let mut client = Client::builder(&token, intents)
    .event_handler(TextBot::default())
    .await
    .expect("Err creating client");

  log_err!(client.start().await);
}
