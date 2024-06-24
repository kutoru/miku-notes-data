# ?

This is one out of four parts of [Miku Notes]()

Application parts:
- [Auth service](https://github.com/kuromii5/sso-auth)
- [Data service](https://github.com/kutoru/miku-notes-data) (this repo)
- [Gateway service](https://github.com/kutoru/miku-notes-gateway)
- [Frontend](https://github.com/kinokorain/Miku-notes-frontend)

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
SERVICE_ADDR=127.0.0.1:55055
MAX_FILE_CHUNK_SIZE_IN_MB=10
```
Where:
- `DATABASE_URL` is the usual Postgres url
- `SERVICE_ADDR` is the address that this service will run on
- `MAX_FILE_CHUNK_SIZE_IN_MB` is an unsigned int that will become the maximum allowed size for recieved file chunks in gRPC messages in Megabytes (10 is fine)
