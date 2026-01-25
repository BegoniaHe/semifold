<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="./docs/docs/public/logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="./docs/docs/public/logo-light.svg">
    <img alt="Semifold Logo" src="./docs/docs/public/logo-light.svg">
  </picture>

# Semifold

[English](README.md) | 中文

</div>

下一代跨语言 monorepo 版本与发布管理工具。

如今，跨语言 monorepo 越来越常见。例如，一个 monorepo 可能包含一个 Rust 库和一个 Node.js 包，开发者使用 `napi-rs` 为 Node.js 创建 Rust 绑定。

`semifold`（CLI：`smif` | `semifold`）帮助团队在大型跨语言 monorepo 中管理版本、变更日志和包发布，实现**一致性、自动化和零痛点**。无论您是在多种语言中构建库、应用还是服务，Semifold 都能保持您的发布流程清晰可预测。

## ✨ 特性

| 特性                             | 描述                                                    |
| -------------------------------- | ------------------------------------------------------- |
| **跨语言 monorepo 支持**         | 管理 Rust / Node.js / 更多语言的版本（可扩展）          |
| **基于 Changeset 的工作流**      | 清晰可追溯的版本变更记录                                |
| **自动版本升级**                 | `smif version` 读取变更并升级语义化版本                 |
| **自动生成变更日志**             | 从提交元数据 / changeset 生成                           |
| **一键发布**                     | 可靠地发布多个包                                        |
| **CI 友好设计**                  | `smif ci` 为 GitHub Actions 或其他 CI 提供稳定的管道    |

## 📌 状态

支持的语言：

- [x] Rust
- [x] Node.js
- [x] Python
- [x] C++
- [ ] Go
- [ ] Java
- [ ] Kotlin

## 🧠 灵感来源

Semifold 的灵感来自以下优秀项目：

[Changesets](https://github.com/changesets/changesets) — 一个简单优雅的基于 changeset 的版本管理工作流，主要专注于 JavaScript 和 npm monorepo。

[Covector](https://github.com/jbolda/covector/) — 一个灵活的多目标发布管理器，旨在支持更复杂的生态系统。

## 🤝 贡献

非常欢迎贡献！请阅读[贡献指南](CONTRIBUTING.md)开始参与。

## 📄 许可证

Semifold 采用 AGPL-3.0 许可证。
