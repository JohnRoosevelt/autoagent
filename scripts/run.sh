#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

if [ ! -f .env ]; then
    echo "未找到 .env。请先执行：cp .env.example .env，然后填写 AGENT_API_KEY。" >&2
    exit 1
fi

set -a
. ./.env
set +a

: "${AGENT_API_KEY:?请先在 .env 中设置 AGENT_API_KEY}"
: "${AGENT_BASE_URL:=https://openrouter.ai/api/v1}"
: "${AGENT_MODEL:=openrouter/free}"

export AGENT_BASE_URL AGENT_API_KEY AGENT_MODEL
exec cargo r -q
