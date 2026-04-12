# Bug: Spinner 与 git 输出重叠

## 发现版本
commit d0432c2

## 修复版本
commit 1f6624a

## 问题描述

`gx remote -v` 执行时，spinner 的残留文本和 git 命令的第一行输出混在同一行：

```
[1/1] 📁 . => main
  ⠋ git remote -v in ....origin	https://github.com/rock117/datagrid-rs.git (fetch)
origin	https://github.com/rock117/datagrid-rs.git (push)
viper	https://github.com/rock117/Viper.git (fetch)
viper	https://github.com/rock117/Viper.git (push)
```

预期输出应该是 spinner 旋转完成后被清除，git 输出独立显示：

```
[1/1] 📁 . => main
origin	https://github.com/rock117/datagrid-rs.git (fetch)
origin	https://github.com/rock117/datagrid-rs.git (push)
viper	https://github.com/rock117/Viper.git (fetch)
viper	https://github.com/rock117/Viper.git (push)
```

## 根因分析

`execute_git_command` 函数同时负责两件事：
1. 执行 git 命令并捕获输出（`.output()` 阻塞等待）
2. 打印捕获的 stdout/stderr

调用方的代码流程：

```rust
let sp = spinner::Spinner::new("git remote -v in ...");
let result = execute_git_command(repo, git_cmd);  // 步骤1+2都在这里
sp.stop();  // spinner 在 git 输出打印之后才清除，太晚了
```

问题在于 `execute_git_command` 内部先执行 git 命令（等待完成），然后**立即打印输出**，然后才返回。此时 `sp.stop()` 还没被调用，spinner 的最后一帧仍留在终端行上，导致 git 输出的第一行和 spinner 残留混在一起。

## 解决方案

将 `execute_git_command` 拆分为两个函数：

- `run_git_capture()` — 仅执行 git 命令并返回 `Output`，不打印
- `display_git_output()` — 仅负责打印捕获的输出

调用方改为：

```rust
let sp = spinner::Spinner::new("git remote -v in ...");
let result = run_git_capture(repo, git_cmd);  // 只执行，不打印
sp.stop();                                     // 先清除 spinner
match &result {
    Ok(output) => display_git_output(output),  // 再打印 git 输出
    Err(_) => {}
}
```

这样 spinner 在 git 输出打印前就被清除，两者不会重叠。

## 影响范围

所有通过 `run_git_command` 执行的 git 命令都受此修复影响：
- `gx pull` / `gx push` / `gx fetch` 等
- `gx remote -v`
- 任何自定义 git 命令

以下命令不受影响（它们的 spinner 和输出逻辑不同）：
- `gx info`（spinner 只在 fetch 阶段显示）
- `gx last --remote` / `gx log --remote`（spinner 只在 fetch 阶段显示）
