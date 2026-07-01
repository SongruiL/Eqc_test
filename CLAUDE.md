# EQC — Equation Compiler

## 项目背景
- EQC（`equation-compiler`）是一个 Rust 库 + CLI 工具（二进制名 `eqc`），把 YAML / S 表达式定义的数学方程编译成 Python、Rust 算子、Workflow JSON、Markdown、LaTeX。
- 用途：用户（首席科学家）做**农业复杂生态系统的数学建模**，方程网络过于复杂，因此开发 EQC 作为数学工具。
- 当前为半成品。首个可用版本见标签 `v0.1`。远程仓库：github.com/SongruiL/Eqc_test（SSH 推送）。

## 协作方式（重要）
- 用户是做研发的**首席科学家**，不是程序架构师。默认节奏是 **先讨论技术路线，后执行**。
- 收到任务时：**默认先进入讨论态**——陪用户推敲思路、质疑假设、比较建模方案；**在用户明确说「执行」之前，不要修改代码。**
- 用户拍板后，再交由执行团队落地。设想的团队角色：
  - **实现者**：按定好的方案写 Rust/Python 代码
  - **科学审核员**（最关键）：检查数学/模型的科学正确性——量纲、边界条件、是否符合领域常识，而不只是代码能否运行
  - **代码审核员**：查代码正确性与 bug
  - **测试员**：写/跑测试，验证行为
- 始终优先**科学正确性**，其次才是工程实现。

## 构建与运行（本机：Windows，无管理员权限）
- PATH 顺序：`C:\Users\lzyay\winlibs\mingw64\bin;C:\Users\lzyay\Rust196\Rust\bin;C:\Program Files\Git\cmd`
- 网络：git/cargo 操作前先清代理变量 `$env:HTTP_PROXY=''; $env:HTTPS_PROXY=''`（cargo 走本项目 `.cargo/config.toml` 里的 rsproxy.cn 镜像）。
- 构建：`cargo build --features cli`（清代理走 rsproxy.cn 镜像联网；产物 `target\debug\eqc.exe`）。**跳过 `gsl_math`**（需系统 GSL C 库）。
  **勿加 `--offline`**：`cli` feature 依赖 `ureq`（serve 的 /api/llm 代理），本机离线稀疏索引缺 ureq 条目→解析期 `no matching package named ureq` 报错；联网走镜像即可（依赖已缓存、约 20–60s）。
- 测试：`cargo test --features cli`（同样勿加 --offline；当前 413 个全过）。

## 代码约定
- 注释与文档用中文，与现有代码保持一致。
- YAML 表达式使用 `{op, args}` / `{ref}` / `{const}` map 格式（由 `Expr` 的手写 `Deserialize` 解析，见 `src/ast/expr.rs`）。
- 提交前先跑 `cargo test`；提交信息说明这次改了什么。

## 记忆与"蒸馏"位置
- 项目偏好/协作协议：**本文件**（随仓库提交，GitHub 上可见）。
- 事实/决策记忆：`C:\Users\lzyay\.claude\projects\c--Users-lzyay-Desktop-projects-2026\memory\`（`MEMORY.md` 为索引）。
- 想固化某个反复出现的决策时，对我说「记住这个」或「把它蒸馏成 skill」。
