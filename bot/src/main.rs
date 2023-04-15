extern crate pretty_env_logger;
use bot_core::storage::PostgresClient;

use teloxide::{
    dispatching::{dialogue::InMemStorage, HandlerExt, UpdateFilterExt},
    dptree,
    prelude::{Dialogue, Dispatcher},
    requests::Requester,
    types::{Message, Update},
    Bot,
};

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceiveFullName,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    log::info!("Starting bot...");

    let bot = Bot::from_env();

    match PostgresClient::new("localhost", 5432, "darthunix", "bot").await {
        Ok(_) => {
            log::info!("Connection to PG data base was established");
        }
        Err(e) => {
            log::error!("Error connecting to database: {}", e);
            return;
        }
    };

    Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<State>, State>()
            .branch(dptree::case![State::Start].endpoint(start))
            .branch(dptree::case![State::ReceiveFullName].endpoint(receive_full_name)),
    )
    .dependencies(dptree::deps![InMemStorage::<State>::new()])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Let's start! What's your full name?")
        .await?;
    dialogue.update(State::ReceiveFullName).await?;
    Ok(())
}

async fn receive_full_name(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let report = format!("Your full name is {}.", text);
            bot.send_message(msg.chat.id, report).await?;
            dialogue.update(State::ReceiveFullName).await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }

    Ok(())
}
