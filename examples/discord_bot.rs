#![allow(unused_imports)]

use rpgsanta_2022::GameData;
use serenity::{
  async_trait,
  model::{
    channel::{Channel, Message},
    gateway::Ready,
    id::ChannelId,
  },
  prelude::*,
};
use std::{
  collections::{hash_map::Entry, HashMap},
  sync::Arc,
  time::Duration,
};
use tokio::{
  sync::{
    mpsc::{channel as mpsc_channel, Sender as MspcSender},
    RwLock, RwLockReadGuard, RwLockWriteGuard,
  },
  task::spawn as task_spawn,
  time::timeout,
};

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
  sessions: Arc<RwLock<HashMap<ChannelId, MspcSender<String>>>>,
}

#[async_trait]
impl EventHandler for TextBot {
  /// This is called when a shard is booted, and a READY payload is sent by
  /// Discord.
  async fn ready(&self, _: Context, ready: Ready) {
    let my_name = ready.user.name.as_str();
    let my_discriminator = ready.user.discriminator;
    println!("Connected as {my_name}#{my_discriminator}!");
  }

  /// This is called for each message the bot sees.
  ///
  /// * When others speak it generates a message
  /// * When the bot speaks it gets events for *its own* messages
  /// * Events are handled async using a thread pool, so multiple messages can
  ///   be in flight at the same time.
  async fn message(&self, ctx: Context, msg: Message) {
    // Currently the bot only supports private message interactions. All other
    // messages are ignored. This is not a fundamental limit, just a logistical
    // one.
    match msg.channel(&ctx.http).await {
      Ok(Channel::Private(priv_chan)) => {
        let recipient = &priv_chan.recipient;
        let recipient_name = recipient.name.as_str();
        let recipient_discrim = recipient.discriminator;
        let them = format!("{recipient_name}#{recipient_discrim}");
        let them_str = them.as_str();
        let they_spoke = recipient_discrim == msg.author.discriminator
          && recipient_name == msg.author.name.as_str();
        let msg_dir = if they_spoke { ">" } else { "<" };
        let content = msg.content.as_str();
        println!("{them_str}{msg_dir} {content}");
        if !they_spoke {
          return;
        }
        //
        let r: RwLockReadGuard<_> = self.sessions.read().await;
        if let Some(sender) = r.get(&msg.channel_id) {
          // If there's already a live session, we just put the message into the
          // channel.
          if let Err(why) = sender.send(msg.content).await {
            println!("Error putting message into the session channel: {why:?}");
          }
        } else {
          // When a session isn't found we have to drop our reader and upgrade
          // to holding the writer.
          drop(r);
          let channel_id = msg.channel_id;
          let mut w: RwLockWriteGuard<_> = self.sessions.write().await;
          match w.entry(channel_id) {
            Entry::Occupied(mut o) => {
              // It's possible for another event to have made a sender between
              // when we first checked and now, so we might be able to send.
              if let Err(why) = o.get_mut().send(content.to_string()).await {
                println!(
                  "Error putting message into the session channel: {why:?}"
                );
              }
            }
            Entry::Vacant(v) => {
              let (sender, mut recver) = mpsc_channel(5);
              let ses = Arc::clone(&self.sessions);
              let http = Arc::clone(&ctx.http);
              // TODO: read the game on disk while the map is locked.
              let mut game = GameData::default();
              task_spawn(async move {
                let them = them;
                let them_str = them.as_str();
                println!("Spinning up new session for {them_str}.");
                const TEN_MIN: Duration = Duration::new(60 * 10, 0);
                while let Ok(Some(message)) =
                  timeout(TEN_MIN, recver.recv()).await
                {
                  if let Err(_) = channel_id.broadcast_typing(&http).await {
                    // should we report this error?
                  }
                  let game_response = game.process_input(message);
                  if let Err(why) =
                    channel_id.say(&http, game_response.as_str()).await
                  {
                    println!("Error sending message: {why:?}");
                    // TODO: if we can't speak in the channel that's probably a
                    // very bad time.
                  }
                }
                println!("Session for {them_str} timed out, shutting down.");
                let mut writer = ses.write().await;
                recver.close();
                while let Some(message) = recver.recv().await {
                  // There *should* be no messages in the receiver right now,
                  // but it's potentially possible.
                  if let Err(_) = channel_id.broadcast_typing(&http).await {
                    // should we report this error?
                  }
                  let game_response = game.process_input(message);
                  if let Err(why) =
                    channel_id.say(&http, game_response.as_str()).await
                  {
                    println!("Error sending message: {why:?}");
                  }
                }
                writer.remove(&channel_id);
                // TODO: write the game to disk while the map is locked.
              });
              if let Err(why) = sender.send(msg.content).await {
                println!(
                  "Error putting message into the session channel: {why:?}"
                );
              }
              v.insert(sender);
            }
          }
        }

        /*
        if msg.content == "!ping" {
          // Sending a message can fail, due to a network error, an
          // authentication error, or lack of permissions to post in the
          // channel, so log to stdout when some error happens, with a
          // description of it.
          if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
            println!("Error sending message: {why:?}");
          }
        }
        */
      }
      _ => return,
    }
  }
}

#[tokio::main]
async fn main() {
  // Configure the client with your Discord bot token in the environment.
  let token =
    std::env::var("DISCORD_TOKEN").expect("Expected a `DISCORD_TOKEN` value");
  // Set gateway intents, which decides what events the bot will be notified
  // about
  let intents = GatewayIntents::GUILD_MESSAGES
    | GatewayIntents::DIRECT_MESSAGES
    | GatewayIntents::MESSAGE_CONTENT;

  // Create a new instance of the Client, logging in as a bot. This will
  // automatically prepend your bot token with "Bot ", which is a requirement
  // by Discord for bot users.
  let mut client = Client::builder(&token, intents)
    .event_handler(TextBot::default())
    .await
    .expect("Err creating client");

  // Finally, start a single shard, and start listening to events.
  //
  // Shards will automatically attempt to reconnect, and will perform
  // exponential backoff until it reconnects.
  if let Err(why) = client.start().await {
    println!("Client start error: {why:?}");
  }
}
