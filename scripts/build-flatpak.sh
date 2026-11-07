#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <vortideck.deb> <output-directory>" >&2
  exit 2
fi

repository_root=$(cd "$(dirname "$0")/.." && pwd)
deb_path=$(realpath "$1")
output_directory=$2
manifest="$output_directory/com.vortideck.desktop.yml"

mkdir -p "$output_directory"
cp "$repository_root/packaging/linux/com.vortideck.desktop.metainfo.xml" "$output_directory/"
escaped_deb_path=${deb_path//|/\\|}
sed "s|@DEB_PATH@|$escaped_deb_path|g" \
  "$repository_root/packaging/linux/com.vortideck.desktop.yml.in" > "$manifest"

flatpak-builder --user --force-clean --disable-rofiles-fuse \
  --repo="$output_directory/repository" \
  "$output_directory/build" "$manifest"
flatpak build-bundle "$output_directory/repository" \
  "$output_directory/VortiDeck.flatpak" com.vortideck.desktop
