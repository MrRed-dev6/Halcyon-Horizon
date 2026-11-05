#!/usr/bin/env bash
# Halcyon: puts log files onto the ci-logs branch so they can be read without the Actions UI.
# usage: halcyon-publish-log.sh <token> <file>...
set -e
TOKEN="$1"; shift
URL="https://x-access-token:${TOKEN}@github.com/${GITHUB_REPOSITORY}.git"
rm -rf /tmp/logs
if ! git clone -q --depth 1 --branch ci-logs "$URL" /tmp/logs 2>/dev/null; then
  mkdir /tmp/logs
  git -C /tmp/logs init -q -b ci-logs
  git -C /tmp/logs remote add origin "$URL"
fi
for f in "$@"; do cp "$f" /tmp/logs/; done
NAME="${GITHUB_WORKFLOW// /-}"
echo "run ${GITHUB_RUN_ID} on ${GITHUB_SHA} (${GITHUB_WORKFLOW})" > "/tmp/logs/run-${NAME}.txt"
cd /tmp/logs
git config user.name "github-actions"
git config user.email "actions@users.noreply.github.com"
git add -A
git commit -q -m "logs of run ${GITHUB_RUN_ID}" || true
for i in 1 2 3; do
  git push -q origin ci-logs && break
  git pull -q --rebase origin ci-logs || true
done
