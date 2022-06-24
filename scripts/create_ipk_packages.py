from cargo_ipk import cargo_package, create_repository
import argparse
import os

parser = argparse.ArgumentParser(
    description='creates an ipk package repository')
parser.add_argument(
    'output')
args = parser.parse_args()

os.makedirs(args.output, exist_ok=True)

ronaldos_webserver = cargo_package("ronaldos_website")
ronaldos_webserver.postinst(f"#!/bin/sh\n\
    logger \"starting {ronaldos_webserver.name}\"\n \
    {ronaldos_webserver.name} -d start")
ronaldos_webserver.prerm(f"#!/bin/sh\n\
    logger \"stopping {ronaldos_webserver.name}\"\n\
    {ronaldos_webserver.name} -d stop")
ronaldos_webserver.install_dir("ronaldos_website/www",
                               "opt/share/ronaldos_website/www")

create_repository([ronaldos_webserver], args.output)
