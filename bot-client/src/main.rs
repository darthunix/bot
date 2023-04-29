extern crate pretty_env_logger;
use bot_core::{
    postgres::{capacity_from_env, config_from_env, PgPool},
    storage::PgStorage,
    user::FullName,
};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use teloxide::{
    dispatching::{HandlerExt, UpdateFilterExt},
    dptree,
    prelude::{Dialogue, Dispatcher},
    requests::Requester,
    types::{ChatKind, ChatPrivate, Message, Update},
    utils::command::BotCommands,
    Bot,
};

type MyDialogue = Dialogue<State, PgStorage>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum State {
    #[default]
    Start,
    RequestLogin,
    RequestFullName,
    IdentifiedUser,
}

#[derive(Clone, BotCommands)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "get your user information.")]
    Get,
    #[command(description = "reset the dialogue.")]
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

    Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, PgStorage, State>()
            .branch(dptree::case![State::Start].endpoint(start))
            .branch(dptree::case![State::RequestLogin].endpoint(request_login))
            .branch(dptree::case![State::RequestFullName].endpoint(request_full_name))
            .branch(
                dptree::case![State::IdentifiedUser]
                    .branch(
                        dptree::entry()
                            .filter_command::<Command>()
                            .endpoint(identified_user),
                    )
                    .branch(dptree::endpoint(invalid_command)),
            ),
    )
    .dependencies(dptree::deps![PgStorage::new(pool)])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}

fn unpack_login(msg: &Message) -> Option<&str> {
    match msg.chat.kind {
        ChatKind::Private(ChatPrivate { ref username, .. }) => username.as_deref(),
        _ => None,
    }
}

fn unpack_name(msg: &Message) -> Option<FullName> {
    match msg.chat.kind {
        ChatKind::Private(ChatPrivate {
            ref first_name,
            ref last_name,
            ..
        }) => {
            let full_name = FullName::try_new(first_name.clone(), last_name.clone());
            full_name
        }
        _ => None,
    }
}

async fn goto_request_login(bot: Bot, dialogue: MyDialogue) -> HandlerResult {
    log::debug!("goto RequestLogin");
    dialogue.update(State::RequestLogin).await?;
    bot.send_message(dialogue.chat_id(), "Please send me your username.")
        .await?;
    Ok(())
}

async fn goto_request_full_name(bot: Bot, dialogue: MyDialogue) -> HandlerResult {
    log::debug!("goto RequestFullName");
    dialogue.update(State::RequestFullName).await?;
    bot.send_message(
        dialogue.chat_id(),
        "Please send me your full name (first and last name).",
    )
    .await?;
    Ok(())
}

async fn goto_identified_user(bot: Bot, dialogue: MyDialogue) -> HandlerResult {
    log::debug!("goto IdentifiedUser");
    dialogue.update(State::IdentifiedUser).await?;
    bot.send_message(
        dialogue.chat_id(),
        "You are identified. Use /get to get your user information and /reset to flush it.",
    )
    .await?;
    Ok(())
}

async fn start(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    storage: Arc<PgStorage>,
) -> HandlerResult {
    log::debug!("Start state");
    if msg.text().is_none() {
        bot.send_message(msg.chat.id, "Say hello to start the dialogue.")
            .await?;
        return Ok(());
    }
    let login: String = match unpack_login(&msg) {
        Some(login) => {
            // We are lucky and have a public username in the chat.
            storage.chat_update(msg.chat.id, login).await?;
            log::info!("User @{login} has logged in");
            String::from(login)
        }
        None => {
            // We have already asked for the username,
            // so we can retrieve it from the database.
            if let Some(login) = storage.login_get(msg.chat.id).await? {
                login
            } else {
                // Let's go to the next state and ask for the username.
                return goto_request_login(bot, dialogue).await;
            }
        }
    };
    match unpack_name(&msg) {
        Some(full_name) => {
            // We are lucky and have a full name in the chat.
            // Let's store it to the database and go to the next state.
            storage.name_update(&login, &full_name).await?;
            log::info!("User @{login} has identified himself as {full_name:?}");
            goto_identified_user(bot, dialogue).await?;
        }
        None => {
            if storage.name_get(&login).await?.is_none() {
                // Let's go to the next state and ask for the full name.
                return goto_request_full_name(bot, dialogue).await;
            }
        }
    };
    Ok(())
}

async fn request_login(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    storage: Arc<PgStorage>,
) -> HandlerResult {
    log::debug!("RequestLogin state");
    match msg.text() {
        Some(login) => {
            // We have received a username, let's store it to the database
            // and go to the next state.
            storage.chat_update(msg.chat.id, login).await?;
            goto_request_full_name(bot, dialogue).await?;
        }
        None => {
            // Let's ask for a username and stay in the same state.
            bot.send_message(msg.chat.id, "Please send me your username.")
                .await?;
        }
    }
    Ok(())
}

async fn request_full_name(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    storage: Arc<PgStorage>,
) -> HandlerResult {
    log::debug!("RequestFullName state");
    match msg.text() {
        Some(full_name) => {
            // We have received a full name, let's store it to the database
            // and go to the next state.
            let full_name = match FullName::try_from_str(full_name) {
                Some(full_name) => full_name,
                None => {
                    // The full name is invalid, let's ask for it again.
                    bot.send_message(msg.chat.id, "Invalid full name.").await?;
                    return Ok(());
                }
            };
            let login: String = match storage.login_get(msg.chat.id).await? {
                Some(login) => login,
                None => {
                    // The username is not in the database, let's return back
                    // to the login request state.
                    return goto_request_login(bot, dialogue).await;
                }
            };
            storage.name_update(&login, &full_name).await?;
            goto_identified_user(bot, dialogue).await?;
        }
        None => {
            // Let's ask for a full name and stay in the same state.
            bot.send_message(msg.chat.id, "Please send me your full name.")
                .await?;
        }
    }
    Ok(())
}

async fn identified_user(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cmd: Command,
    storage: Arc<PgStorage>,
) -> HandlerResult {
    log::debug!("IdentifiedUser state");
    match cmd {
        Command::Get => {
            let login = match storage.login_get(msg.chat.id).await? {
                Some(login) => login,
                None => {
                    // The username is not in the database, let's return back
                    // to the login request state.
                    return goto_request_login(bot, dialogue).await;
                }
            };
            bot.send_message(msg.chat.id, format!("Here is your username: {login}."))
                .await?;
        }
        Command::Reset => {
            dialogue.reset().await?;
            bot.send_message(
                msg.chat.id,
                "Your username was reset. Write any message to start the dialogue again.",
            )
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
