#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

lazy_static! {
    static ref HASHMAP: Mutex<HashMap<u64, Vec<String>>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
}

use dotenv::dotenv;
use std::env;

use openai_api_rs::v1::api::Client as OpenaiClient;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionRequest};
use openai_api_rs::v1::error::APIError;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

async fn openai(text: String, list: Vec<String>) -> Result<String, APIError> {
    let mut messages = vec![];
    for (_i, item) in list.iter().enumerate() {
        messages.push(chat_completion::ChatCompletionMessage {
            role: chat_completion::MessageRole::assistant,
            content: item.to_string(),
        });
    }
    messages.push(chat_completion::ChatCompletionMessage {
        role: chat_completion::MessageRole::user,
        content: text,
    });
    let openai_api_key = get_env("OPENAI_TOKEN");
    let client = OpenaiClient::new(openai_api_key);
    let req = ChatCompletionRequest {
        model: chat_completion::GPT3_5_TURBO.to_string(),
        messages,
    };
    let result = client.chat_completion(req).await;
    match result {
        Ok(result) => Ok(result.choices[0].message.content.to_string()),
        Err(e) => Err(e),
    }
}

fn set_to_map(key: u64, value: Vec<String>) {
    let mut map_guard: MutexGuard<HashMap<u64, Vec<String>>> = HASHMAP.lock().unwrap();
    map_guard.insert(key, value);
}

fn get_from_map(key: u64) -> Vec<String> {
    let map_guard: MutexGuard<HashMap<u64, Vec<String>>> = HASHMAP.lock().unwrap();
    map_guard.get(&key).unwrap_or(&vec![]).to_vec()
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {why:?}");
            }
        }
        if msg.content.starts_with("!ai ") {
            let text = msg.content.split(' ').nth(1).unwrap_or("").to_string();
            let mut list = get_from_map(msg.channel_id.into());
            let result = openai(text, list.clone()).await;
            let text = match result {
                Ok(text) => text,
                Err(e) => format!("Error: {e:?}"),
            };
            if list.len() > 100 {
                list.remove(0);
            }
            list.push(text.clone());
            set_to_map(msg.channel_id.into(), list);
            if let Err(why) = msg.channel_id.say(&ctx.http, text).await {
                println!("Error sending message: {why:?}");
            }
        }
    }
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = get_env("DISCORD_TOKEN");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

fn get_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("Expected a {key} in the environment"))
}
