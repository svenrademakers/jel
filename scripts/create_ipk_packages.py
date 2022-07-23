from cargo_ipk import cargo_package, create_repository
import argparse
import os

parser = argparse.ArgumentParser(
    description='creates an ipk package repository')
parser.add_argument(
    'output')
args = parser.parse_args()

os.makedirs(args.output, exist_ok=True)

ronaldos_webserver = cargo_package("ronaldos-webserver")
# ronaldos_webserver.postinst(f"#!/bin/sh\n\
#     logger \"starting {ronaldos_webserver.name}\"\n \
#     {ronaldos_webserver.name} -d start") 
# ronaldos_webserver.prerm(f"#!/bin/sh\n\
#     logger \"stopping {ronaldos_webserver.name}\"\n\
#     {ronaldos_webserver.name} -d stop")
ronaldos_webserver.install_dir("ronaldos_webserver/www",
                               "opt/share/ronaldos-webserver/www")
uacme_renew = cargo_package("uacme-renew")
uacme_renew.add_dependency("uacme")

create_repository([ronaldos_webserver, uacme_renew], args.output)
