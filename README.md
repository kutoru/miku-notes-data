# ?

This is one out of four parts of [Miku Notes](https://github.com/kutoru/miku-notes). This part is responsible for direct interaction with the user-generated data, such as creating notes, storing files, updating shelves, etc.

# How to run this service

**It is highly recommended** to run this service along with other parts simultaneously by using docker compose. The instructions for that can be found in the [main repository](https://github.com/kutoru/miku-notes).

With having that said, you could still run the service manually by following the instructions below.

First, make sure that you:
- have cloned the submodule in the `./proto` directory
- have the [protoc](https://grpc.io/docs/protoc-installation) binary on your path
- have created and filled out your [.env configuration](#env)
- have a Postgres database launched and set up according to your .env configuration
- have already ran migrations from the **Auth service**

If necessary, you can run the database migrations using [sqlx cli](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md). You can install it with
```
cargo install sqlx-cli --no-default-features --features native-tls,postgres
```
**Note** that the migration command below must be executed in the project's root directory, as it is looking for a .env file with a `DATABASE_URL` value inside. Also **note** that this command will not work if you haven't yet ran the **Auth service**'s migrations. In any way, the command to run migrations is
```
sqlx migrate run
```
You can also revert migrations with
```
sqlx migrate revert
```

After setting everything up, you can do the usual `cargo run` in the root directory

# .env

The .env file should be located in the root directory and have the following contents:
```
DATABASE_URL=postgres://postgres:admin@localhost:5432/databasename?schema=public
SERVICE_PORT=5050
SERVICE_TOKEN=3san9kyu
MAX_FILE_CHUNK_SIZE=8
```
Where:
- `DATABASE_URL` is the usual Postgres url
- `SERVICE_PORT` is the port that the gRPC routes of this service will run on
- `SERVICE_TOKEN` is a random string that would become the required Authorization token for all incoming requests
- `MAX_FILE_CHUNK_SIZE` is an unsigned int that will become the maximum allowed size for received file chunks in gRPC messages in megabytes
