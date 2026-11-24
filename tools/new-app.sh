#!/usr/bin/env bash
# Start a new Rust service product from this repo's full layered skeleton.
#
#   tools/new-app.sh billing ~/Documents/projects/billing
#
# Copies the committed tree (git archive HEAD), so the new product keeps AGENTS.md,
# CLAUDE.md, tools/ (scaffolding + hook + arch-check), the Jenkinsfile and the arch-qube
# gate. Copying single crates into a fresh `cargo new` loses all of that (AGENTS.md §1).
set -euo pipefail

[ $# -eq 2 ] || { echo "usage: $0 <app-name> <dest-dir>" >&2; exit 1; }
APP="$1"
DEST="$2"
[[ "$APP" =~ ^[a-z][a-z0-9]*(-[a-z0-9]+)*$ ]] || { echo "app-name: lower-case letters/digits, words joined by '-' (it becomes crate names <app>-core, <app>_core…)" >&2; exit 1; }
[ "$APP" != arcana ] || { echo "app-name must differ from the template's 'arcana'" >&2; exit 1; }
[ ! -e "$DEST" ] || { echo "$DEST already exists — refusing to overwrite" >&2; exit 1; }
command -v perl >/dev/null || { echo "perl is required" >&2; exit 1; }

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
git -C "$ROOT" diff --quiet HEAD -- AGENTS.md tools/ || echo "note: uncommitted changes in AGENTS.md/tools are NOT copied (git archive HEAD)" >&2

mkdir -p "$DEST"
git -C "$ROOT" archive HEAD | tar -x -C "$DEST"
SRC_SHA="$(git -C "$ROOT" rev-parse --short HEAD)"
APP_US="$(printf '%s' "$APP" | tr '-' '_')"
cd "$DEST"

# ---- Product identity. Every place the template name is load-bearing:
#  1. crate/package names  arcana-<x>  (Cargo.toml, Cargo.lock, -p flags, Dockerfiles, sonar)
#     and their Rust paths arcana_<x>   (use statements, docs)
#  2. binary / image / compose service  arcana-server
#  3. CI identities: Jenkins APP_NAME + image "rust-app", coverage image "rust-coverage",
#     Sonar key/name, and the per-host CI container/network names arcana-ci-rust-*,
#     arcana-arch-qube-*, kind clusters arcana-ci-* (the Jenkins post step deletes every
#     cluster matching its prefix) — two products on one Jenkins host must not share them.
# Not renamed on purpose: the ARCANA_ config env prefix (config loader contract), proto
# package names, and arcana.boo (the registry / arch-qube image host).
CRATES="$(perl -ne 'print "$1\n" if m{^\s*"(?:crates|plugins)/arcana-([a-z0-9-]+)",}' Cargo.toml)"
[ -n "$CRATES" ] || { echo "new-app: no arcana-* workspace members found in Cargo.toml" >&2; exit 1; }
# tools/ is excluded: these scripts describe the template and must keep working in the product.
TEXT_FILES="$(grep -rIl -e arcana -e rust-app -e 'Rust App' -e rust-coverage . --exclude-dir=.git --exclude-dir=tools || true)"
for c in $CRATES; do
    c_us="$(printf '%s' "$c" | tr '-' '_')"
    # (?<![\w.-]) keeps arcana.boo/arcana/... and words like "arcana-cloud-rust" intact.
    printf '%s\n' "$TEXT_FILES" | xargs perl -pi -e "s/(?<![\\w.-])arcana-${c}(?![\\w-])/${APP}-${c}/g; s/(?<![\\w])arcana_${c_us}(?!\\w)/${APP_US}_${c_us}/g"
done
printf '%s\n' "$TEXT_FILES" | xargs perl -pi -e "s/arcana-ci-rust-/${APP}-ci-/g; s/arcana-ci-kind-rust/${APP}-ci-kind/g; s/arcana-ci-\\\$\\(/${APP}-ci-\\\$(/g; s/grep arcana-ci\b/grep ${APP}-ci-/g; s/arcana-arch-qube-/${APP}-arch-qube-/g; s/arcana-rust-coverage-/${APP}-coverage-/g; s/\\brust-coverage\\b/${APP}-coverage/g; s/\\brust-app\\b/${APP}/g; s/Rust App/${APP}/g"
for c in $CRATES; do
    for d in crates plugins; do
        if [ -d "$d/arcana-$c" ]; then mv "$d/arcana-$c" "$d/$APP-$c"; fi
    done
done
# (file list re-read: the crate directories were just renamed)
# Fail loudly if any crate-name token survived (same token rule as the rename above).
LEFT="$(grep -rIl --exclude-dir=.git --exclude-dir=tools --exclude=CHANGELOG.md arcana . | xargs perl -ne 'print "$ARGV:$.: $_" if /(?<![\w.-])arcana[-_](core|server|service|repository|rest|grpc|proto|security|config|jobs)(?![\w-])/; close ARGV if eof' || true)"
if [ -n "$LEFT" ]; then
    echo "new-app: template crate names still present after rename:" >&2
    printf '%s\n' "$LEFT" | head >&2
    exit 1
fi

# ---- arch-qube layer mapping. Its bundled rust profile recognises the template's crates by
# name ("arcana-rest/", "arcana-grpc/" -> controller, "arcana-core/" -> core). After the rename
# those paths match nothing, so the REST/gRPC crates would scan as unlayered and fail "Backend
# Layer Chain" (measured 2026-11-23 on a renamed copy: score 99.8 but FAIL). layer_paths restores
# the same mapping for the new names; service/repository/dao already match by suffix.
cat > arch-qube.yaml <<EOT
# Read by arch-qube (tools/arch-check.sh and the Jenkins stage). Restores the template's
# crate -> layer mapping for the renamed crates; see tools/new-app.sh.
layer_paths:
  controller: ["${APP}-rest", "${APP}-grpc"]
  core: ["${APP}-core"]
EOT

# ---- Fresh history: version 0.1.0, empty changelog.
rm -f CHANGELOG.md
printf '{\n  ".": "0.1.0"\n}\n' > .release-please-manifest.json

cat >> AGENTS.md <<EOT

## Origin

Created from arcana-cloud-rust @ \`${SRC_SHA}\` by \`tools/new-app.sh\` (crates renamed
\`arcana-*\` → \`${APP}-*\`). The layers, rules and definition of done above apply unchanged.
Remove template features you do not need through \`di.rs\` / \`router.rs\`, not by flattening.
EOT

git init -q
tools/install-hooks.sh >/dev/null
git add -A
# --no-verify only for this import commit: every file counts as "added" here, so the hook
# would judge the template's own legacy root files (crates/*-jobs/src/queue.rs, …) as new
# flat files. The template itself passes arch-check; every later commit in the product goes
# through the hook.
git -c user.name="${GIT_AUTHOR_NAME:-new-app}" -c user.email="${GIT_AUTHOR_EMAIL:-new-app@localhost}" \
    commit -q --no-verify -m "chore: start ${APP} from arcana-cloud-rust ${SRC_SHA}"

echo "created $DEST (${APP}) from arcana-cloud-rust ${SRC_SHA}; hooks: $(git config core.hooksPath)"
echo "next: cd $DEST && tools/new-feature.sh <Name>   # one vertical slice per feature"
