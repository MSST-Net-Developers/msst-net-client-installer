# TUI 美化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 `dialoguer` + `console` 替换纯文本 CLI，实现方向键菜单、彩色输出和格式化 banner，三平台兼容。

**Architecture:** 所有视觉逻辑集中在 `src/ui.rs`，新增 banner/section/success/error/info 输出函数；`src/main.rs` 仅调用这些函数，不直接使用 console/style API。

**Tech Stack:** `dialoguer 0.11`（交互菜单）、`console 0.15`（颜色/样式）、已有 `indicatif 0.17`（进度条，保持不变）

## Global Constraints

- 三平台兼容（Linux / macOS / Windows）——不得引入任何仅限 Unix 的依赖
- 不清屏（不调用 `Term::clear_screen()`），保留用户滚动历史
- 进度条逻辑（indicatif）保持不变，本次只改交互和输出样式
- 编译目标：`cargo check` 零警告零错误

---

### Task 1: 添加依赖 + 重写 ui.rs

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/ui.rs`（完整重写）

**Interfaces:**
- Produces:
  - `ui::print_banner()` — 显示带框标题，无返回值
  - `ui::print_section(title: &str)` — 加粗段落标题，无返回值
  - `ui::print_success(msg: &str)` — 绿色 ✓ 行，无返回值
  - `ui::print_error(msg: &str)` — 红色 ✗ 行，输出到 stderr，无返回值
  - `ui::print_info(msg: &str)` — 暗色 · 行，无返回值
  - `ui::prompt_yn(question: &str) -> bool` — 签名不变
  - `ui::prompt_select(prompt: &str, options: &[&str]) -> usize` — 签名不变

- [ ] **Step 1: 在 Cargo.toml 添加依赖**

  在 `[dependencies]` 末尾追加：
  ```toml
  dialoguer = "0.11"
  console   = "0.15"
  ```

- [ ] **Step 2: 重写 src/ui.rs**

  完整替换文件内容：
  ```rust
  use console::style;
  use dialoguer::{theme::ColorfulTheme, Confirm, Select};

  pub fn print_banner() {
      println!();
      println!("  {}", style("╭─────────────────────────────────╮").cyan());
      println!(
          "  {}  {}  {}",
          style("│").cyan(),
          style("MSST-Net 客户端安装程序").cyan().bold(),
          style("│").cyan()
      );
      println!("  {}", style("╰─────────────────────────────────╯").cyan());
      println!();
  }

  pub fn print_section(title: &str) {
      println!();
      println!("  {} {}", style("──").dim(), style(title).bold());
      println!();
  }

  pub fn print_success(msg: &str) {
      println!("  {} {}", style("✓").green().bold(), style(msg).bold());
  }

  pub fn print_error(msg: &str) {
      eprintln!("  {} {}", style("✗").red().bold(), style(msg).red());
  }

  pub fn print_info(msg: &str) {
      println!("  {} {}", style("·").dim(), style(msg).dim());
  }

  pub fn prompt_yn(question: &str) -> bool {
      Confirm::with_theme(&ColorfulTheme::default())
          .with_prompt(question)
          .default(true)
          .interact()
          .unwrap_or(false)
  }

  pub fn prompt_select(prompt: &str, options: &[&str]) -> usize {
      Select::with_theme(&ColorfulTheme::default())
          .with_prompt(prompt)
          .items(options)
          .default(0)
          .interact()
          .unwrap_or(0)
  }
  ```

- [ ] **Step 3: 验证编译**

  ```bash
  cargo check
  ```

  期望：`Finished` 无错误无警告（dialoguer/console 初次下载依赖，需联网）。

- [ ] **Step 4: 提交**

  ```bash
  git add Cargo.toml Cargo.lock src/ui.rs
  git commit -m "feat: replace text UI with dialoguer + console"
  ```

---

### Task 2: 更新 main.rs 调用点

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: Task 1 产出的所有 `ui::print_*` 函数

- [ ] **Step 1: main() 顶部添加 banner，替换 elevated 错误**

  找到：
  ```rust
  println!("=== MSST-Net 客户端安装程序 ===");
  println!();

  if !detect::is_elevated() {
      eprintln!("错误：安装程序必须以 root（Linux/macOS）或管理员（Windows）权限运行。");
      std::process::exit(1);
  }
  ```

  替换为：
  ```rust
  ui::print_banner();

  if !detect::is_elevated() {
      ui::print_error("安装程序必须以 root（Linux/macOS）或管理员（Windows）权限运行。");
      std::process::exit(1);
  }
  ```

- [ ] **Step 2: run_install — 段落标题和完成信息**

  找到：
  ```rust
  println!("正在从 {} 获取最新版本信息...", mirror.display_name());
  ```
  替换为：
  ```rust
  ui::print_section(&format!("从 {} 获取最新版本信息", mirror.display_name()));
  ```

  找到：
  ```rust
  println!("最新版本：{}", release.tag_name);
  println!();
  ```
  替换为：
  ```rust
  ui::print_info(&format!("最新版本：{}", release.tag_name));
  ```

  找到：
  ```rust
  let install_dir = os.install_dir();
  println!("安装目录：{}", install_dir.display());
  ```
  替换为：
  ```rust
  let install_dir = os.install_dir();
  ui::print_info(&format!("安装目录：{}", install_dir.display()));
  ```

  找到：
  ```rust
  println!();
  println!("正在配置系统服务...");
  ```
  替换为：
  ```rust
  ui::print_section("配置系统服务");
  ```

  找到（run_install 结尾）：
  ```rust
  println!();
  println!("=== 安装完成！===");
  println!("安装目录：{}", install_dir.display());
  println!("网络核心：{}", core_dest.display());
  if let Some(p) = &controller_dest {
      println!("控制器  ：{}", p.display());
  } else {
      println!("控制器  ：已通过系统包管理器安装");
  }
  println!("服务    ：msst-net（已启用并运行）");
  ```
  替换为：
  ```rust
  println!();
  ui::print_success("安装完成！");
  ui::print_info(&format!("安装目录：{}", install_dir.display()));
  ui::print_info(&format!("网络核心：{}", core_dest.display()));
  if let Some(p) = &controller_dest {
      ui::print_info(&format!("控制器  ：{}", p.display()));
  } else {
      ui::print_info("控制器  ：已通过系统包管理器安装");
  }
  ui::print_info("服务    ：msst-net（已启用并运行）");
  ```

- [ ] **Step 3: run_update — 段落标题和完成信息**

  找到：
  ```rust
  println!("正在从 {} 获取最新版本信息...", mirror.display_name());
  ```
  替换为：
  ```rust
  ui::print_section(&format!("从 {} 获取最新版本信息", mirror.display_name()));
  ```

  找到：
  ```rust
  println!("最新版本：{}", release.tag_name);
  println!();
  ```
  替换为：
  ```rust
  ui::print_info(&format!("最新版本：{}", release.tag_name));
  ```

  找到：
  ```rust
  let install_dir = os.install_dir();
  println!("安装目录：{}", install_dir.display());
  ```
  替换为：
  ```rust
  let install_dir = os.install_dir();
  ui::print_info(&format!("安装目录：{}", install_dir.display()));
  ```

  找到：
  ```rust
  println!("正在停止服务...");
  ```
  替换为：
  ```rust
  ui::print_section("停止服务");
  ```

  找到：
  ```rust
  println!();
  println!("正在重启服务...");
  ```
  替换为：
  ```rust
  ui::print_section("重启服务");
  ```

  找到（run_update 结尾）：
  ```rust
  println!();
  println!("=== 更新完成！===");
  println!("安装目录：{}", install_dir.display());
  println!("网络核心：{}", core_dest.display());
  if let Some(p) = &controller_dest {
      println!("控制器  ：{}", p.display());
  } else {
      println!("控制器  ：已通过系统包管理器安装");
  }
  ```
  替换为：
  ```rust
  println!();
  ui::print_success("更新完成！");
  ui::print_info(&format!("安装目录：{}", install_dir.display()));
  ui::print_info(&format!("网络核心：{}", core_dest.display()));
  if let Some(p) = &controller_dest {
      ui::print_info(&format!("控制器  ：{}", p.display()));
  } else {
      ui::print_info("控制器  ：已通过系统包管理器安装");
  }
  ```

- [ ] **Step 4: run_uninstall — 警告和完成信息**

  找到：
  ```rust
  println!("警告：此操作将停止并删除 MSST-Net 服务及所有已安装文件。");
  ```
  替换为：
  ```rust
  ui::print_error("此操作将停止并删除 MSST-Net 服务及所有已安装文件。");
  ```

  找到：
  ```rust
  println!("正在停止并移除服务...");
  ```
  替换为：
  ```rust
  ui::print_section("停止并移除服务");
  ```

  找到：
  ```rust
  if install_dir.exists() {
      println!("正在删除安装目录：{}", install_dir.display());
      std::fs::remove_dir_all(&install_dir)?;
      println!("安装目录已删除。");
  }

  println!();
  println!("=== 卸载完成！===");
  ```
  替换为：
  ```rust
  if install_dir.exists() {
      ui::print_info(&format!("正在删除安装目录：{}", install_dir.display()));
      std::fs::remove_dir_all(&install_dir)?;
  }

  println!();
  ui::print_success("卸载完成！");
  ```

- [ ] **Step 5: 其余 println! 清理**

  以下 `println!` 保持原样（它们是安装过程中的进度提示，改动收益低）：
  - `println!("检测到操作系统：...")` — `confirm_os()` 内部
  - `println!("CPU 架构：...")` — arch 检测
  - `println!("检测到包管理器：...")` — Linux pkg mgr
  - `println!("正在下载网络核心...")` / `println!("正在下载控制器...")` — 紧接 indicatif 进度条，保持一致

  以下替换：
  ```rust
  // confirm_os 中
  println!();   // 每个分支末尾的空行 → 保留不动（Select 之后有自动换行）
  ```

  > 注意：`prompt_select` 已改用 `dialoguer::Select`，自带视觉分隔，原先手动 `println!()` 的空行可能产生多余空白——如果运行后视觉上偏多，删除 `confirm_os` / `select_*` 函数内调用 prompt 之后的 `println!();`。

- [ ] **Step 6: 验证编译**

  ```bash
  cargo check
  ```

  期望：`Finished` 无错误无警告。

- [ ] **Step 7: 手动运行验证视觉效果**

  以非特权用户运行（直接触发 elevated 错误路径，验证红色 ✗）：
  ```bash
  cargo run
  ```

  检查：
  1. 顶部显示青色 banner 框
  2. "必须以 root..." 错误以红色 ✗ 显示
  3. 若以 sudo 运行，菜单使用方向键选择，高亮当前项
  4. 完成后显示绿色 ✓

- [ ] **Step 8: 提交**

  ```bash
  git add src/main.rs
  git commit -m "feat: apply colored output and section headers in main"
  ```
