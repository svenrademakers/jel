#!/bin/bash
set -e
package_name=$(cargo get --root http_server -n)
authors=$(cargo get --root http_server -a)
description=$(cargo get --root http_server -d)
version=$(cargo get --root http_server version)
architecture="aarch64-3.10"
install_prefix="opt/sbin"
www_install_prefix="opt/share/ronaldo_www"

function create_control_file() {
    mkdir -p "$IPK_DIR/CONTROL"
    echo "Package: $package_name
Version: $version
Architecture: $architecture
Maintainer: $authors
Description: $description
Priority: optional
Installed-Size: $(du -s $IPK_DIR/DATA | awk '{print $1; exit}')
" > "$IPK_DIR/CONTROL/control"
}

function create_postinst() {
    echo "#!/bin/sh
/$install_prefix/$package_name --conf /opt/etc/$package_name/conf.yaml
" > "$IPK_DIR/CONTROL/postinst"
}

function create_prerm() {
    echo "#!/bin/sh
pid=\$(pidof "$package_name")
if [[ \$pid ]]; then
    kill \$pid
fi
" >  "$IPK_DIR/CONTROL/prerm"
}

function package_data() {
    package_binary="$RUST_OUT/$package_name"
    install_dir="$IPK_DIR/DATA/$install_prefix"
    mkdir -p "$install_dir"

    echo "copying $package_binary to $install_dir"
    cp "$package_binary" "$install_dir"

    www_install_path="$IPK_DIR/DATA/$www_install_prefix"   
    www_source="http_server/www/."
    echo "copying $www_source to $www_install_path"
    mkdir -p "$www_install_path"
    cp -r "$www_source" "$www_install_path"
}

function create_ipk() {
    ipk_package_name="${package_name}_$version.$architecture.ipk"
    echo 2.0 > "$IPK_DIR/debian-binary"
    pushd "$IPK_DIR/CONTROL/"
    tar --group=$GROUP --owner=$OWNER -czf ../control.tar.gz ./*
    popd

    pushd "$IPK_DIR/DATA/"
    tar --group=$GROUP --owner=$OWNER -czf ../data.tar.gz ./*
    popd

    pushd "$IPK_DIR"
    tar --group=$GROUP --owner=$OWNER -czf ../$ipk_package_name ./debian-binary ./data.tar.gz ./control.tar.gz 
    popd
    echo "created $PWD/$ipk_package_name"
    rm -rf "$IPK_DIR"
}

function create_package_repository() {
    mkdir $REPOSITORY_DIR
    cd $REPOSITORY_DIR
    mv ../*.ipk .
    opkg-make-index . > Packages
    echo "created $REPOSITORY_DIR"
}

package_data
create_control_file
create_postinst
create_prerm
chmod -R 755 "$IPK_DIR"
chmod -R 744 "$IPK_DIR/DATA/$www_install_prefix"
create_ipk
create_package_repository
