# qianshou 🌏

qianshou teŋge qe WebSocket relay service ⚡

qianshou ла a high-performance WebSocket 中继服务 ⚡

qianshou es un servicio de retransmisión WebSocket ⚡

qianshou is a high-performance WebSocket relay service ⚡

qianshou 是用于 local-agent-relay 架构的 Rust 服务器组件

qianshou есть компонент сервера для архитектуры local-agent-relay

qianshou es un componente de servidor escrito en Rust para la arquitectura local-agent-relay

qianshou 支持双向通信 JSON-RPC 协议调用 SSE 日志流

qianshou поддерживает двустороннюю связь вызовы протокола JSON-RPC и потоковую передачу логов SSE

qianshou soporta comunicación bidireccional llamadas de protocolo JSON-RPC y transmisión de logs SSE

实时双向通信是核心功能

Realtime bidirectional communication is the core function

La comunicación bidireccional en tiempo real es la función central

Token 认证确保安全

Токен аутентификация обеспечивает безопасность

La autenticación de token asegura la seguridad

心跳每三十秒发送

Heartbeat sent every thirty seconds

El latido se envía cada treinta segundos

日志流订阅便于监控

Подписка на поток логов облегчает мониторинг

La suscripción al flujo de logs facilita la monitorización

快速开始

Быстрый старт

Inicio rápido

```bash
git clone https://github.com/cokeisnotbest/qianshou.git
cd qianshou
cargo build
cargo run
```

配置

Конфигурация

Configuración

编辑 config.yaml

Редактировать config.yaml

Editar config.yaml

```yaml
host: "0.0.0.0"
port: 8080
token: "your-token"
```

架构

Архитектура

Arquitectura

Client ←WSS→ Relay ←WSS→ Agent

许可证

Лицензия

Licencia

MIT

Авторское право

Derechos de autor

联系

Связаться

Contacto

https://github.com/cokeisnotbest/qianshou/issues
