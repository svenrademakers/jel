# Ronaldo Streaming

The aim of this project is serving live streams over https. It exists out of the following components:
* hyper_rusttls, custom async wrappers that enable fully async tls streams in hyper.
* uacme_renew, application that renews tls certificates on a given interval
* ronaldos_webserver, hyper https webserver that presents the frontend.
  Streams are offered in 3 different bitrates, so the client can adaptively
  switch.

This code base is intended to run on a Asus AX86u.

Some other cool features of this project:
* It compiles a ipkg package index, so you can host your own package repository
* Nix support, sets up an native build environment, compiles everything from
  source on any machine with only one command.

## Build

Building is super easy, only [nix](https://nixos.org/download.html) is required to be installed on your machine.
to build the .ipk packages run the following command:

```bash
nix build .
```

The resulting .ipk packages will be located under 'result/'

Similarly, to build the binaries, run:

```bash
nix build .\#ronaldo-streaming
```

_Currently only builds aarch64.3.10 musl target_
[![Http Server workflow](https://github.com/svenrademakers/jel/actions/workflows/main.yml/badge.svg?branch=master)](https://github.com/svenrademakers/jel/actions/workflows/main.yml)

## Install

On every commit to master the package repository gets updated with the new packages.
To quickly get going, you can add the repository to your opkg package manager. In order to do so add the following private opkg repository:

```bash
echo "src ronaldos_repository  http://svenrademakers.com:81/ronaldos_repository" >> /opt/etc/opkg.conf
opkg update
opkg list ronaldos-webserver
```

## Running

Application works by passing the config path, `--config <config>`, as argument. ( "/opt/etc/ronaldo.cfg" is used on default). This config file is formatted as yaml. Parameters are written as key-value pairs.
The following parameters can be set:
- www_dir: "dir/to/html/root"
- port: 80 #http port, used for redirecting to https 443
- host: "0.0.0.0"
- hostname: "name"
- private_key: "/path/to/private/key
- certificates: "/path/to/certificates"
- verbose: true # log debug, info otherwise
- api_key: # footbal api key
