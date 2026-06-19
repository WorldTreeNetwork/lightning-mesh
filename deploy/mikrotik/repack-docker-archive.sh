#!/usr/bin/env bash
# Repack a buildkit `type=docker` image tar (OCI blobs/ layout, gzip layers)
# into the CLASSIC `docker save` layout that RouterOS /container imports:
#   manifest.json            -> Config: "<hash>.json", Layers: ["<id>/layer.tar"]
#   <confighash>.json        (image config)
#   <layerdiffid>/layer.tar  (UNCOMPRESSED tar), plus VERSION + json
#
# Why: RouterOS rejects gzip-compressed blobs/sha256 layers with
# "could not load next layer". This produces the uncompressed per-layer layout
# it expects, without re-running the (slow, flaky) Docker build.
#
# Usage: repack-docker-archive.sh <in.tar> <out.tar> [repo:tag]
set -euo pipefail

SRC="${1:?usage: repack-docker-archive.sh <in.tar> <out.tar> [repo:tag]}"
OUT="${2:?missing out.tar}"
REPOTAG="${3:-mjolnir-meshd:armv7}"

# Absolutize OUT — it is written from inside a `cd "$WORK/out"` subshell below,
# so a relative path would resolve against the wrong directory.
mkdir -p "$(dirname "$OUT")"
OUT="$(cd "$(dirname "$OUT")" && pwd)/$(basename "$OUT")"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/src" "$WORK/out"

tar -xf "$SRC" -C "$WORK/src"
MANIFEST="$WORK/src/manifest.json"
[ -f "$MANIFEST" ] || { echo "no manifest.json in $SRC" >&2; exit 1; }

# Pull Config + Layers blob paths out of the manifest (no jq dependency).
CONFIG_BLOB="$(grep -oE '"Config":"[^"]+"' "$MANIFEST" | head -1 | sed -E 's/.*"Config":"([^"]+)".*/\1/')"
LAYER_BLOBS="$(grep -oE '"Layers":\[[^]]*\]' "$MANIFEST" | grep -oE 'blobs/sha256/[a-f0-9]+')"

CONFIG_HASH="$(basename "$CONFIG_BLOB")"
cp "$WORK/src/$CONFIG_BLOB" "$WORK/out/${CONFIG_HASH}.json"

LAYER_PATHS=()
PARENT=""
for BLOB in $LAYER_BLOBS; do
  # Decompress if gzip; otherwise copy through.
  if gzip -t "$WORK/src/$BLOB" 2>/dev/null; then
    gunzip -c "$WORK/src/$BLOB" > "$WORK/layer.tmp"
  else
    cp "$WORK/src/$BLOB" "$WORK/layer.tmp"
  fi
  DIFFID="$(shasum -a 256 "$WORK/layer.tmp" | cut -d' ' -f1)"
  mkdir -p "$WORK/out/$DIFFID"
  mv "$WORK/layer.tmp" "$WORK/out/$DIFFID/layer.tar"
  printf '1.0' > "$WORK/out/$DIFFID/VERSION"
  printf '{"id":"%s"%s}' "$DIFFID" "${PARENT:+,\"parent\":\"$PARENT\"}" > "$WORK/out/$DIFFID/json"
  PARENT="$DIFFID"
  LAYER_PATHS+=("$DIFFID/layer.tar")
done

# Rebuild manifest.json in classic form.
LAYERS_JSON="$(printf '"%s",' "${LAYER_PATHS[@]}" | sed 's/,$//')"
printf '[{"Config":"%s.json","RepoTags":["%s"],"Layers":[%s]}]\n' \
  "$CONFIG_HASH" "$REPOTAG" "$LAYERS_JSON" > "$WORK/out/manifest.json"

# Pack as a plain ustar archive (no pax/AppleDouble cruft that can trip RouterOS).
( cd "$WORK/out" && COPYFILE_DISABLE=1 tar --format ustar -cf "$OUT" manifest.json "${CONFIG_HASH}.json" */ )

echo "repacked -> $OUT"
