#!/bin/sh
# 从任意目录运行：sh packages/rsille-cli/examples/deploy.sh
# 依赖 rsille、jq、mktemp；仅打印部署计划，不修改集群。
set -eu
example_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
command -v rsille >/dev/null 2>&1 || { echo '请先安装 rsille' >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo '请先安装 jq' >&2; exit 1; }
result=$(mktemp)
trap 'rm -f "$result"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# stdin 是 JSON 管道，stdout 是结果文件；键盘与界面仍使用 /dev/tty。
if cat "$example_dir/services.json" | rsille run "$example_dir/deploy.yaml" --data - >"$result"; then
    :
else
    status=$?
    case "$status" in
        130) echo '已取消；未部署任何服务。' >&2 ;;
        *) echo "界面失败（退出码 $status）；未部署任何服务。" >&2 ;;
    esac
    exit "$status"
fi

jq -e '.action == "submit" and (.values.services | type == "array" and length <= 1)' "$result" >/dev/null
if [ "$(jq '.values.services | length' "$result")" -eq 0 ]; then
    echo '没有选择服务；未执行部署。' >&2
    exit 0
fi
name=$(jq -er '.values.services[0].deployment' "$result")
# 即使返回原始对象，业务端仍需校验白名单，不能 eval 输出。
case "$name" in
    api|worker) printf '部署计划：kubectl rollout restart deployment/%s\n' "$name" ;;
    *) echo '未知部署目标，拒绝执行。' >&2; exit 2 ;;
esac
# 在这里接入自己的鉴权、环境确认与部署逻辑，例如：
# kubectl --context staging rollout restart "deployment/$name"
