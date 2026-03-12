# qianshou 🌏 本地代理中继系统

qianshou (伸手) 是一个高性能的 WebSocket 中继服务 ⚡

这是一个用于 local-agent-relay 架构的 Rust 编写的服务器组件 🇺🇸

支持双向通信 🔄 JSON-RPC 协议调用 📜 以及 SSE 日志流 📊

实时双向通信 🌐 是核心功能 🔐

Token 认证确保安全 💓

心跳保活维持连接 📝

日志流订阅便于监控 🎯

快速开始 🚀

```bash
git clone https://github.com/cokeisnotbest/qianshou.git
cd qianshou
cargo build
cargo run
```

配置 ⚙️

编辑 config.yaml 🛠️

```yaml
host: "0.0.0.0"
port: 8080
token: "your-token"
```

架构 🏗️

Client ←WSS→ Relay ←WSS→ Agent 🇩🇪

许可证 📄

MIT © 2024 🇧🇷

联系 📧

Issues: https://github.com/cokeisnotbest/qianshou/issues 🇨🇳
