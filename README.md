# bot

A telegram bot that persists the dialogs into the PostgreSQL database. The code is written with async Rust ([tokio](https://docs.rs/tokio/latest/tokio/) runtime) and relies on [teloxide](https://docs.rs/teloxide/latest/teloxide/) (for telegram bot API) and [deadpool](https://docs.rs/deadpool/latest/deadpool/) (for PostgreSQL communication).

## PostgreSQL schema

The schema is versioned with [Pyrseas](https://pyrseas.readthedocs.io/en/latest/) and stored in the [db.yaml](https://github.com/darthunix/bot/blob/main/bot-core/src/db.yaml) file. To generate the SQL file from the YAML file, run:

```bash
createdb bot
yamltodb bot bot-core/src/db.yaml > migration.sql
psql -f migration.sql bot
```

## Build

```bash
cargo build --release
```

## Run

First export your telegram bot token:

```bash
export TELOXIDE_TOKEN=...
```

Then run the bot:

```bash
RUST_LOG="debug" PG_USER=$USER PG_POOL_CAPACITY=2 ./target/release/bot
```

You can configure the PostgreSQL connection settings with the environment variables listed in the [config_from_env()](https://github.com/darthunix/bot/blob/main/bot-core/src/postgres.rs#L12) function.
