# rsille CLI

在 Shell 脚本中使用交互式选择器、文本输入、确认提示和表格，也可以通过 YAML 组合表单。无需编写 Rust，交互结果可直接交给脚本处理。

目前支持 Unix 全屏终端，需要前台交互终端（`/dev/tty`）。暂不支持 Windows 和 inline 模式。

## 安装

克隆仓库后，在仓库根目录运行：

```sh
cargo install --path packages/rsille-cli
```

安装需要 Rust 工具链。可执行文件名为 `rsille`：

```sh
rsille --help
```

## 快速开始

### 选择

```sh
rsille select --title '选择环境' dev staging production
```

上下键移动，Enter 提交。默认返回 JSON：

```json
{"action":"submit","values":{"value":"production"}}
```

使用 `--output plain` 获取适合 Shell 变量的文本结果：

```sh
if environment=$(rsille select --output plain dev staging production); then
    printf '选择的环境：%s\n' "$environment"
fi
```

### 输入

```sh
rsille input --label '项目名称'
rsille input --label '版本号' --value '1.0.0' --output plain
```

Enter 提交输入。输入内容可见，不适用于密码。

### 确认

```sh
if rsille confirm --title '继续部署？' > /dev/null; then
    printf '已确认\n'
fi
```

确认返回 `{"action":"submit","values":{"value":true}}`；取消不输出结果。

### 表格

从文件或管道读取 JSON 对象数组：

```sh
printf '[{"name":"api","status":"ready"},{"name":"worker","status":"ready"}]' \
    | rsille table --selectable --columns name,status > selected.json
```

也可以直接查看文件：

```sh
rsille table services.json
```

- 默认只读；添加 `--selectable` 启用单选。
- 上下键移动，Enter 提交当前行或完成查看。
- `--columns` 指定显示的字段；省略时按字段名排序显示所有字段。
- 选择结果保留原始对象，包括未显示的字段。结果始终为数组，空表返回 `[]`。

选择结果示例：

```json
{"action":"submit","values":{"value":[{"name":"api","status":"ready"}]}}
```

## 用 YAML 组合界面

将以下内容保存为 `form.yaml`：

```yaml
version: 1
title: 创建项目
layout:
  column:
    - input:
        id: name
        label: 项目名称
    - select:
        id: environment
        label: 环境
        options: [dev, staging, production]
    - checkbox:
        id: tests
        label: 运行测试
        checked: true
    - row:
        - button:
            label: 创建
            action: submit
        - button:
            label: 取消
            action: cancel
```

运行：

```sh
rsille run form.yaml > result.json
```

提交后，`values` 包含以组件 ID 为键的值：

```json
{"action":"submit","values":{"name":"my-app","environment":"dev","tests":true}}
```

使用 Tab / Shift-Tab 切换焦点，通过提交按钮完成表单。配置只描述界面，不执行 Shell 命令；创建项目或部署服务等操作由调用脚本完成。

### 组件

| 节点 | 字段 |
| --- | --- |
| `row` / `column` | 子节点数组，水平 / 垂直排列 |
| `text` | 显示的字符串 |
| `input` | `id`、`label`；可选 `value`，默认空字符串 |
| `select` | `id`、`label`、非空 `options` 字符串数组；可选 `selected`，默认索引 `0` |
| `checkbox` | `id`、`label`；可选 `checked`，默认 `false` |
| `table` | `rows` 或 `source`；可选 `columns`、`selectable`、`id` |
| `button` | `label`、`action`（`submit` 或 `cancel`） |

配置必须指定 `version: 1`。`title` 可省略，`mode` 默认为 `fullscreen`，也是目前唯一支持的模式。需要返回值的组件必须有非空且唯一的 `id`；未知字段、无效选项索引和重复 ID 会报错。

### 引用外部数据

通过 `--data` 传入 JSON 文件，或使用 `--data -` 从管道读取：

```sh
rsille run deploy.yaml --data services.json > result.json
cat services.json | rsille run deploy.yaml --data - > result.json
```

例如，`services.json`：

```json
{
  "services": [
    {"name":"api","version":"1.2.0"},
    {"name":"worker","version":"1.1.0"}
  ]
}
```

`deploy.yaml`：

```yaml
version: 1
title: 部署服务
layout:
  column:
    - table:
        id: services
        source: data.services
        selectable: true
        columns: [name, version]
    - row:
        - button:
            label: 部署
            action: submit
        - button:
            label: 取消
            action: cancel
```

在表格上按 Enter 选择行，再切换到部署按钮提交。未选择时提交返回空数组：

```json
{"action":"submit","values":{"services":[]}}
```

`source` 支持 `data` 或 `data.services` 这样的对象字段路径，不支持数组索引、表达式或带点的字段名。字段名须以字母或下划线开头，后续字符可为字母、数字或下划线。数据路径必须存在，且表格数据必须是对象数组。指定 `source` 时，其数据替代 `rows`。

完整的调用脚本见 [部署示例](examples/deploy.sh)，配套提供 [界面配置](examples/deploy.yaml) 和 [数据文件](examples/services.json)。脚本需要 `jq`，默认只打印部署计划。

## 在脚本中处理结果

界面使用交互终端，stdout 只输出结果，stderr 输出错误信息。因此输入管道与结果重定向可以同时使用，不会将界面的控制序列写入结果文件。

默认输出单个 JSON 对象，以换行结束：

```json
{"action":"submit","values":{}}
```

- `input`、`select`、`confirm` 和可选择的快捷表格使用 `values.value`。
- `run` 使用组件的 `id` 作为键。
- 只读表格不向 `values` 添加值。
- `--output plain` 仅适用于 `input`、`select`、`confirm`，输出字符串或 `true`，末尾加换行。

| 退出码 | 含义 |
| --- | --- |
| `0` | 提交或完成查看 |
| `1` | 运行失败，例如没有交互终端或结果写出失败 |
| `2` | 参数、配置或数据无效 |
| `130` | 用户取消 |

取消和输入校验失败时 stdout 为空。写出结果失败时可能已有部分输出，脚本应检查退出码，而不只检查文件内容。

```sh
if rsille run form.yaml > result.json; then
    jq '.values' result.json
else
    status=$?
    case "$status" in
        130) printf '已取消\n' >&2 ;;
        *)   printf '界面运行失败（退出码 %s）\n' "$status" >&2 ;;
    esac
fi
```

## 键盘操作

| 按键 | 操作 |
| --- | --- |
| Tab / Shift-Tab | 切换焦点 |
| Esc / Ctrl-C | 取消整个界面 |
| 上下键 | 在选项或表格行之间移动 |
| Enter | 提交快捷命令；在表单中操作当前组件 |
| 空格 | 切换复选框或激活按钮 |

表单中的选择器使用 Enter 或空格展开，再用上下键移动、Enter 确定。文本输入支持方向键、Backspace 和 Delete。

## 输入限制

- 配置与 JSON 数据均须为 UTF-8，每个输入最多 4 MiB；不支持持续数据流。
- 布局和 JSON 数据的最大嵌套深度为 32。
- 布局最多 10,000 个节点；选项、表格行列、JSON 数组元素和对象字段等各集合最多 10,000 项。
- 显示时过滤终端控制字符，选择值和表格结果保留原始数据。`plain` 输出不转义原始值；处理不可信数据时请使用 JSON，不要将结果传给 `eval`。
