# Ronaldo Streaming

Rust https server including ronaldo's own specific http services written in rust as well.

## Build

On every commit to master the CI builds a .ipk package and pushes it to the package registery.
Currently only builds aarch64.3.10 target
[![Http Server workflow](https://github.com/svenrademakers/jel/actions/workflows/main.yml/badge.svg?branch=master)](https://github.com/svenrademakers/jel/actions/workflows/main.yml)

## Install

the application can be installed via the opkg package manager. In order to do so add the following private opkg repository:

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
