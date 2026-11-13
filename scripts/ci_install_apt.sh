#!/usr/bin/env bash
set -euo pipefail

. /etc/os-release
mirror=https://mirrors.edge.kernel.org/ubuntu
curl --fail --location --connect-timeout 10 --max-time 30 --output /dev/null \
  "${mirror}/dists/${VERSION_CODENAME}/InRelease"
printf '%s\n' "$mirror" | sudo tee /etc/apt/apt-mirrors.txt

apt_options=(-o Acquire::Retries=2 -o Acquire::http::Timeout=15 -o Acquire::https::Timeout=15)
sudo apt-get update -o APT::Update::Error-Mode=any "${apt_options[@]}"
sudo apt-get install -y "${apt_options[@]}" "$@"
