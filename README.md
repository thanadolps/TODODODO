TODODODO microservices project as part of software architecture course.

## How to run (for now)

1. Install rust: https://rustup.rs/
2. Install tools: `cargo install sqlx-cli mprocs cargo-watch`
3. Run `docker compose up`
4. Run `mprocs`

## Services crates

These are binary crates which represent each services.

**Note: Most services has `/docs` endpoint for openapi documentation.**

### [task_service](task_service)

Stores and manages tasks.

### [notification_service](notification_service)

Receive notification requests from other services and send them to users.

### [viz_service](viz_service)

Store and manage score and metrics data to create visualizations for users.

### [account_service](account_service)

Store and manage user accounts, generated JWT tokens for authentication. also managed community groups.

## Libraries crates

These are library crates which are not themself services, but are common utilities for services.

### [gengrpc](gengrpc)

Contain protobuf definitions, and generate grpc code for other services.

## Database

Main database is TimescaleDB (Postgres), defined in [docker-compose.yaml](docker-compose.yaml). For decoupling, Each service has its independent and seperated user and schema.

[docker-entrypoint-initdb.d](docker-entrypoint-initdb.d) contains the initialization scripts for the database user and schema.

## Adding migration

1. `cd` into service directory
2. run `sqlx migrate add -r <migration_name>`
3. write migration up and down

(see [sqlx-cli](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#enable-building-in-offline-mode-with-query) for more command)

## Common crates

(in [Cargo.toml](Cargo.toml))

- [`sqlx`](https://lib.rs/crates/sqlx) - database access
- [`poem`](https://lib.rs/crates/poem) - web framework
  - [`poem-openapi`](https://lib.rs/crates/poem-openapi) - openapi support ([example](https://github.com/poem-web/poem/tree/master/examples/openapi))
  - [`poem-grpc`](https://lib.rs/crates/poem-grpc) - grpc support
- [`serde`](https://lib.rs/crates/serde) - serialization and deserialization
- [`tokio`](https://lib.rs/crates/tokio) - async runtime
- [`tracing`](https://lib.rs/crates/tracing) - logging and metrics framework
  - [`tracing-subscriber`](https://lib.rs/crates/tracing-subscriber) - tracing utilities
- [`color-eyre`](https://lib.rs/crates/color-eyre) - colorful error handling
- [`dotenvy`](https://lib.rs/crates/dotenvy) - `.env` support
- [`envy`](https://lib.rs/crates/envy) - typesafe environment variables loading
