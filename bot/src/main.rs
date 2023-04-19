extern crate pretty_env_logger;
use bot_core::{
    postgres::{capacity_from_env, config_from_env, PgPool},
    storage::PgStorage,
};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use teloxide::{
    dispatching::{HandlerExt, UpdateFilterExt},
    dptree,
    prelude::{Dialogue, Dispatcher},
    requests::Requester,
    types::{Message, Update},
    utils::command::BotCommands,
    Bot,
};

type MyDialogue = Dialogue<State, PgStorage>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum State {
    #[default]
    Start,
    GotUser(String),
}

#[derive(Clone, BotCommands)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "get your user name.")]
    Get,
    #[command(description = "reset your user name.")]
    Reset,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();

    log::info!("Starting bot...");
    let bot = Bot::from_env();

    let config = match config_from_env() {
        Ok(config) => config,
        Err(e) => {
            log::error!("Error reading config: {}", e);
            return;
        }
    };
    let capacity = capacity_from_env();
    let pool = match PgPool::new(config, capacity) {
        Ok(pool) => pool,
        Err(e) => {
            log::error!("Error creating pool: {}", e);
            return;
        }
    };
    match pool.get().await {
        Ok(_) => {
            log::info!("Connection to PG data base was established");
        }
        Err(e) => {
            log::error!("Error connecting to database: {}", e);
            return;
        }
    };
    let pg_storage = PgStorage::new(pool);

    Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, PgStorage, State>()
            .branch(dptree::case![State::Start].endpoint(start))
            .branch(
                dptree::case![State::GotUser(name)]
                    .branch(
                        dptree::entry()
                            .filter_command::<Command>()
                            .endpoint(got_username),
                    )
                    .branch(dptree::endpoint(invalid_command)),
            ),
    )
    .dependencies(dptree::deps![pg_storage])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(name) => {
            dialogue.update(State::GotUser(name.into())).await?;
            let report = format!("Your full name is {name}. Now use /get or /reset.");
            bot.send_message(msg.chat.id, report).await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Please, send me your username.")
                .await?;
        }
    }
    Ok(())
}

async fn got_username(
    bot: Bot,
    dialogue: MyDialogue,
    name: String, // Available from `State::GotUser`.
    msg: Message,
    cmd: Command,
) -> HandlerResult {
    match cmd {
        Command::Get => {
            bot.send_message(msg.chat.id, format!("Here is your username: {name}."))
                .await?;
        }
        Command::Reset => {
            dialogue.reset().await?;
            bot.send_message(msg.chat.id, "Your username was reset.")
                .await?;
        }
    }
    Ok(())
}

async fn invalid_command(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Please, send /get or /reset.")
        .await?;
    Ok(())
}
