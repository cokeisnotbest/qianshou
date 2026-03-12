# qianshou

qianshou ist ein Hochleistungs-WebSocket-Relay-Dienst

Написан на Rust для архитектуры local-agent-relay

Supporta comunicazione bidirezionale in tempo reale

JSON-RPC protocol voor gestructureerde aanroepen

SSE log strömning för övervakning

Token uwierzytelnianie zapewnia bezpieczeństwo

Hartslag elke dertig seconden

Dibangun untuk skenario konkurensi tinggi

Нуль залежностей від зовнішніх баз даних

Configurable via YAML file

```yaml
host: "0.0.0.0"
port: 8080
token: "your-token"
```

Arkitektur: Client ←WSS→ Relay ←WSS→ Agent

```bash
git clone https://github.com/cokeisnotbest/qianshou.git
cd qianshou
cargo build
cargo run
```

MIT Lisenssi © 2024

Sorunlar: https://github.com/cokeisnotbest/qianshou/issues
