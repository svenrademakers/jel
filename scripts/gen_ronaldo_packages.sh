#!/bin/bash
#
# Usage:
#   ./gen_ronaldo_packages.sh <output_path>
#
#   When no output path is specified, packages will be written to cwd
#
PACKAGE_NAME="ronaldo-sys"
VERSION="1.0"
NGINX_VERSION="1.27.2"
MAINTAINER="Sven Rademakers <sven.rademakers@gmail.com>"
DESCRIPTION="core video grab system"

OUTPUT_PATH=${1:-$(pwd)}
ROOT_DIR=$(dirname "$(realpath "$0")")/..
STAGING_DIR="/tmp/${PACKAGE_NAME}_${VERSION}"

# append this function to add more files to the packages
stage_files() {
SYSTEMD_ROOT=$STAGING_DIR/etc/systemd/system
BIN=$STAGING_DIR/usr/bin
install -Dm 755 $ROOT_DIR/scripts/roki_open $BIN/roki_open
install -Dm 755 $ROOT_DIR/scripts/roki_broadcast $BIN/roki_broadcast
install -Dm 644 $ROOT_DIR/systemd/rtmp_server.service $SYSTEMD_ROOT/rtmp_server.service
install -Dm 644 $ROOT_DIR/systemd/vnc_server.service $SYSTEMD_ROOT/vnc_server.service
install -Dm 644 $ROOT_DIR/systemd/xvfb.service $SYSTEMD_ROOT/xvfb.service
install -Dm 644 $ROOT_DIR/systemd/ronaldo.target $SYSTEMD_ROOT/ronaldo.target
install -Dm 644 $ROOT_DIR/systemd/nginx.conf $STAGING_DIR/etc/ronaldo/nginx.conf
}

build_deb() {
    local output_dir="/tmp/_ronaldo_deb/${PACKAGE_NAME}_${VERSION}"
    mkdir -p ${output_dir}/DEBIAN
    cat <<EOL > ${output_dir}/DEBIAN/control
Package: $PACKAGE_NAME
Version: $VERSION
Section: base
Priority: optional
Architecture: all
Maintainer: $MAINTAINER
Description: $DESCRIPTION
Depends: nginx, libnginx-mod-rtmp, xvfb, x11vnc, chromium-browser
EOL

cat <<EOL > ${output_dir}/DEBIAN/postinst
#!/bin/bash
systemctl daemon-reload
EOL
chmod 755 ${output_dir}/DEBIAN/postinst

cp -r $STAGING_DIR/* $output_dir

# Build the .deb package
dpkg-deb --build $output_dir
cp "${output_dir}.deb" $OUTPUT_PATH
echo "Package $PACKAGE_NAME v${VERSION} created successfully."
dirname ${output_dir} | xargs rm -rf
}

build_aur() {
    local output_dir="/tmp/_ronaldo_aur/${PACKAGE_NAME}_${VERSION}"
    mkdir -p $output_dir

cat <<EOL > $output_dir/${PACKAGE_NAME}.install
install_rtmp_module() {
    pushd /tmp
    rm -rf /tmp/nginx-rtmp-module
    wget http://nginx.org/download/nginx-${NGINX_VERSION}.tar.gz
    tar -xzf nginx-${NGINX_VERSION}.tar.gz

    git clone --branch v1.2.2 --depth 1 https://github.com/arut/nginx-rtmp-module /tmp/nginx-rtmp-module
    cd nginx-${NGINX_VERSION}
    ./configure --add-module=/tmp/nginx-rtmp-module
    make modules
    make install
}

post_install() {
    echo "Reloading systemd daemon.."
    systemctl daemon-reload
}

post_upgrade() {
    post_install
}

post_remove() {
    post_install
}
EOL

cat <<EOL > $output_dir/PKGBUILD
# Maintainer: $MAINTAINER
pkgname=$PACKAGE_NAME
pkgver=$VERSION
pkgrel=1
pkgdesc="$DESCRIPTION"
arch=('any')
license=('GPL')
depends=('nginx' 'chromium' 'xorg-server-xvfb' 'x11vnc')
optdepends=('nginx-mod-rtmp')
source=("\$pkgname-\$pkgver.tar.gz")
install="$PACKAGE_NAME.install"

package() {
    cp -r --preserve=mode "\$srcdir/src/." "\$pkgdir/"
}
EOL

mkdir -p ${output_dir}/src
cp -r $STAGING_DIR/* ${output_dir}/src

pushd ${output_dir} > /dev/null
tar -czf ${PACKAGE_NAME}-${VERSION}.tar.gz src
makepkg --printsrcinfo > .SRCINFO
makepkg -g >> PKGBUILD
makepkg -c --install
mv ronaldo*.xz $OUTPUT_PATH
popd > /dev/null

echo "AUR package $PACKAGE_NAME v${VERSION} created successfully."
dirname ${output_dir} | xargs rm -rf
}

stage_files

build_deb

if [[ $(cat /etc/os-release | grep "^NAME=") =~ "Arch Linux" ]]; then
    build_aur
fi

rm -rf $STAGING_DIR
