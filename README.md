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
