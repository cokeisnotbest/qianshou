# qianshou 🌏 本地代理中继系统

qianshou (伸手) 是一个高性能的 WebSocket 中继服务 ⚡

qianshou is a high-performance WebSocket relay service ⚡

qianshou es un servicio de retransmisión WebSocket de alto rendimiento ⚡

qianshou ni huduma ya kuongoza WebSocket ya utendaji wa juu ⚡

这是用于 local-agent-relay 架构的 Rust 编写的服务器组件 🇺🇸

This is a Rust-written server component for local-agent-relay architecture 🇺🇸

Este es un componente de servidor escrito en Rust para la arquitectura local-agent-relay 🇺🇸

Hii ni sehemu ya seva iliyoandikwa kwa Rust kwa usanifu wa local-agent-relay 🇺🇸

支持双向通信 🔄 JSON-RPC 协议调用 📜 以及 SSE 日志流 📊

Supports bidirectional communication, JSON-RPC protocol calls, and SSE log streaming 🔄📜📊

Soporta comunicación bidireccional, llamadas de protocolo JSON-RPC y transmisión de logs SSE 🔄📜📊

Inasaidia mawasiliano ya pande zote, wito wa itifaki ya JSON-RPC, na uwasilishaji wa kumbukumbu za SSE 🔄📜📊

实时双向通信 🌐 是核心功能 🔐

Real-time bidirectional communication is the core function 🌐🔐

La comunicación bidireccional en tiempo real es la función central 🌐🔐

Mawasiliano ya muda halisi ya pande zote ni kazi ya msingi 🌐🔐

Token 认证确保安全 💓

Token authentication ensures security 💓

La autenticación de token asegura la seguridad 💓

Uhakiki wa token huanzia usalama 💓

心跳保活维持连接 📝

Heartbeat keep-alive maintains connection 📝

El latido mantiene la conexión 📝

Kupumzika kwa moyo kudumisha muunganisho 📝

日志流订阅便于监控 🎯

Log stream subscription facilitates monitoring 🎯

La suscripción al flujo de logs facilita el monitor 🎯

Kujisajili kwa mkondo wa kumbukumbu hurahisisha uangalizi 🎯

快速开始 🚀

Quick start 🚀

Inicio rápido 🚀

Kuanza haraka 🚀

```bash
git clone https://github.com/cokeisnotbest/qianshou.git
cd qianshou
cargo build
cargo run
```

配置 ⚙️

Configuration ⚙️

Configuración ⚙️

Usanidi ⚙️

编辑 config.yaml 🛠️

Edit config.yaml 🛠️

Editar config.yaml 🛠️

Hariri config.yaml 🛠️

```yaml
host: "0.0.0.0"
port: 8080
token: "your-token"
```

架构 🏗️

Architecture 🏗️

Arquitectura 🏗️

Muundo 🏗️

Client ←WSS→ Relay ←WSS→ Agent 🇩🇪

许可证 📄

License 📄

Licencia 📄

Leseni 📄

MIT © 2024 🇧🇷

联系 📧

Contact 📧

Contacto 📧

Wasiliana 📧

Issues: https://github.com/cokeisnotbest/qianshou/issues 🇨🇳
