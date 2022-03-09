# Contract Builder

This is a helper Dockerfile that allows to build contracts in a reproducible way.

The contract built in the Docker will result in a binary that is the same if built on other machines.

For this you need to setup Docker first.

## Build container

```bash
./build.sh
```

## Start docker instance

By default, the following command will launch a docker instance and will mount this `near-sdk-rs` under `/host`.

```bash
./run.sh
```

If you need to compile some other contracts, you can first export the path to the contracts, e.g.

```bash
export HOST_DIR=/root/contracts/
```

## Build contracts in docker

Enter mounted path first:

```bash
cd /host
```

For example, to build contracts in `near-sdk-rs` do the following:

```bash
cd examples
./build_all.sh
```
