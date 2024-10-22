#!/bin/bash
work_dir=/home/builder/gh-action

echo "::group::Copying files from /workspace to $work_dir"
mkdir -p $work_dir
cp -rfv /workspace/* $work_dir

cd $work_dir
updpkgsums
makepkg --printsrcinfo >.SRCINFO
makepkg -Cd

sudo cp -f PKGBUILD /workspace
sudo cp -f .SRCINFO /workspace
sudo cp  *.zst /workspace
echo "::endgroup::"
