#!/usr/bin/env bash
set -euo pipefail

qpdf_dir="${RUNNER_TEMP:?}/qpdf-12.4.2"
qpdf_zip="${qpdf_dir}.zip"
curl --fail --location --retry 3 \
  https://github.com/qpdf/qpdf/releases/download/v12.4.2/qpdf-12.4.2-bin-linux-x86_64.zip \
  -o "$qpdf_zip"
echo "db367d897829f22c4198ce1094143c9d467bd6ee7dfabc44ba6f02056b24f8b1  $qpdf_zip" | sha256sum --check
unzip -q "$qpdf_zip" -d "$qpdf_dir"
"$qpdf_dir/bin/qpdf" --version
echo "$qpdf_dir/bin" >> "${GITHUB_PATH:?}"
