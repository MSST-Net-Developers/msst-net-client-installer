# TUI 美化设计文档

**日期：** 2026-06-29  
**状态：** 已批准

## 目标

用 `dialoguer` + `console` 替换现有纯文本 CLI，实现方向键菜单、彩色输出、格式化 banner，同时保持三平台（Linux / macOS / Windows）兼容。

## 依赖

```toml
dialoguer = "0.11"
console   = "0.15"
```

- `dialoguer`：提供 `Select`（方向键菜单）和 `Confirm`（Y/N 确认框）
- `console`：提供 `style()`、`Term`，用于颜色、加粗、清行

## 变更范围

### `Cargo.toml`
新增两条依赖。

### `src/ui.rs`（完整重写）

| 函数 | 说明 |
|------|------|
| `prompt_select(prompt, options)` | 用 `dialoguer::Select` 替换数字输入，方向键 + Enter 选择 |
| `prompt_yn(question)` | 用 `dialoguer::Confirm` 替换 y/n 手输 |
| `print_banner()` | 新增：启动时显示带框标题，青色 |
| `print_section(title)` | 新增：段落标题，加粗白色，前缀 `──` |
| `print_success(msg)` | 新增：绿色 `✓ msg` |
| `print_error(msg)` | 新增：红色 `✗ msg` |
| `print_info(msg)` | 新增：暗色 `· msg`，用于次要信息 |

### `src/main.rs`（局部调整）

- 顶部调用 `ui::print_banner()`
- 各阶段标题（"正在获取版本信息"、"正在配置服务" 等）改用 `ui::print_section()`
- `eprintln!` 错误改用 `ui::print_error()`
- 安装完成的汇总信息改用 `ui::print_success()`

## 交互效果对比

**改前：**
```
=== MSST-Net 客户端安装程序 ===

请选择操作：
  [1] 安装
  [2] 更新
请选择 [1-2]：
```

**改后：**
```
╭─────────────────────────────╮
│   MSST-Net 客户端安装程序      │
╰─────────────────────────────╯

── 请选择操作

❯ 安装
  更新
  卸载
```

## 不在范围内

- 进度条样式（indicatif 已足够）
- 全屏 TUI / 分栏布局
- Windows 特有终端适配（dialoguer/console 已内置处理）
