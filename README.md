# ?

This is one out of four parts of [Miku Notes]()

Application parts:
- [Auth service](https://github.com/kuromii5/miku-notes-auth)
- [Data service](https://github.com/kutoru/miku-notes-data) (this repo)
- [Gateway service](https://github.com/kutoru/miku-notes-gateway)
- [Frontend](https://github.com/kinokorain/miku-notes-frontend)

This repo is the service that directly interacts with the user-generated data

# How to run

First, make sure that you:
- have cloned the submodule in the `./proto` directory
- have the [protoc](https://grpc.io/docs/protoc-installation) binary on your path
- have created and filled out your [.env configuration](#env)
- have a Postgres database launched and set up according to your .env configuration

If necessary, you can run the database migrations using [sqlx cli](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md). You can install it with
```
cargo install sqlx-cli --no-default-features --features native-tls,postgres
```
Note that the migration command below must be executed in the project's root directory, as it is looking for a .env file with a `DATABASE_URL` value inside. The command to run migrations is
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
