from cargo_ipk import cargo_package, create_repository
import argparse
import os

parser = argparse.ArgumentParser(
    description='creates an ipk package repository')
parser.add_argument(
    'output')
parser.add_argument(
    'www')
parser.add_argument(
    '-m','--manifest_dir', default=".")
parser.add_argument(
    '-b', '--binary_path' )
args = parser.parse_args()

os.makedirs(args.output, exist_ok=True)

ronaldos_webserver = cargo_package("ronaldos-webserver", args.manifest_dir,
                                   args.binary_path)
# ronaldos_webserver.postinst(f"#!/bin/sh\n\
#     logger \"starting {ronaldos_webserver.name}\"\n \
#     {ronaldos_webserver.name} -d start") 
# ronaldos_webserver.prerm(f"#!/bin/sh\n\
#     logger \"stopping {ronaldos_webserver.name}\"\n\
#     {ronaldos_webserver.name} -d stop")
ronaldos_webserver.install_dir(args.www,
                               "opt/share/ronaldos-webserver/www")
uacme_renew = cargo_package("uacme-renew", args.manifest_dir, args.binary_path)
uacme_renew.add_dependency("uacme")

create_repository([ronaldos_webserver, uacme_renew], args.output)
