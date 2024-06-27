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

After that you can do the usual `cargo run` in the root directory

# .env

The .env file should be located in the root directory and have the following contents:
```
DATABASE_URL=postgres://postgres:admin@localhost:5432/databasename?schema=public
GRPC_SERVICE_ADDR=127.0.0.1:5050
FILE_SERVICE_ADDR=127.0.0.1:6060
MAX_FILE_CHUNK_SIZE_IN_MB=10
```
Where:
- `DATABASE_URL` is the usual Postgres url
- `GRPC_SERVICE_ADDR` is the address that the gRPC routes of this service will run on
- `FILE_SERVICE_ADDR` is the address that the file managing routes (such as upload and download) of this service will run on
- `MAX_FILE_CHUNK_SIZE_IN_MB` is an unsigned int that will become the maximum allowed size for received file chunks in gRPC messages in megabytes (10 is fine)
