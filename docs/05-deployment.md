# 05 · 私有服务器部署与运维方案（124.223.154.233）

> 目标读者：后端工程师 / 运维负责人。基线实测（2026-10-06）：TCP 22 可达。目标：**单 VPS 自治运行**——Cricket-Server、PostgreSQL(知识库真相源)、SearXNG、Crawler、备份调度，无任何第三方 SaaS 依赖。

---

## ⚠️ 0. 第 0 优先级任务（M0 开始前执行）

> root 密码曾以明文出现在沟通链路中，视为**已泄露**。以下三条在任何部署动作之前完成：

```bash
# ① 轮换 root 密码（强随机 ≥ 24 位，入密码管理器）
passwd root

# ② 生成 ed25519 密钥并注入（本地执行）
ssh-keygen -t ed25519 -a 100 -f ~/.ssh/cricket_deploy
ssh-copy-id -i ~/.ssh/cricket_deploy.pub root@124.223.154.233

# ③ 服务器上：禁用密码登录与 root 远程登录后重启 sshd
#    /etc/ssh/sshd_config.d/10-cricket.conf
#      PasswordAuthentication no
#      KbdInteractiveAuthentication no
#      PermitRootLogin prohibit-password
sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config
systemctl reload ssh     # 注意：保持现有会话不退出验证成功后再登出
```

此后一切自动化（CI/CD、备份脚本）均以 `cricket_deploy` 密钥 + 受限用户执行，密钥永不进入仓库。

---

## 1. 系统基线加固清单

| # | 项 | 措施 |
|---|----|------|
| 1 | 账户 | 新建 `cricket` 运行账户（无 sudo 密码、无 shell 登录选项）；root 仅 console |
| 2 | 防火墙（ufw） | `allow 22`（可限源 IP 段）、`allow 80,443/tcp`，默认 deny；Docker 发布端口仅绑定 `127.0.0.1`（Postgres/SearXNG **绝不** 0.0.0.0） |
| 3 | fail2ban | sshd jail：`maxretry 3, bantime 1h`；caddy 401 洪水自定义 jail |
| 4 | 自动更新 | `unattended-upgrades`（仅 security 源）；Postgres 大版本升级走手动窗口 |
| 5 | 时间 | chrony（TLS 证书与 HMAC 时间窗依赖准确时钟） |
| 6 | 日志 | journald `SystemMaxUse=500M`；容器日志 `max-size=10m, max-file=3`；**日志脱敏**：relay 请求体默认只记元数据（模型、token 数、时长），不记 prompt 明文（用户隐私，私有化核心承诺） |
| 7 | 审计 | `/var/log/auth.log` 与 `docker events` 进入每日备份集 |
| 8 | 内核 | sysctl：`net.ipv4.tcp_fastopen=3`（低延迟加分）、`somaxconn=4096`（SSE 并发） |

---

## 2. 生产拓扑（Docker Compose）

```
Internet ──443──▶ Caddy ──┬──▶ cricket-server :8080 (127.0.0.1)   ← Axum, SSE
                          │        │ 5432
                          │        └──▶ postgres (127.0.0.1, pgvector+zhparser)
                          ├──▶ searxng :8888    (127.0.0.1, web_search 上游)
                          └──▶ /static/*（更新 feed / 安装包分发，Caddy file_server）
```

`deploy/docker-compose.yml`（要点）：

```yaml
name: cricket
services:
  postgres:
    image: pgvector/pgvector:pg16            # zhparser 扩展单独安装到自定义镜像
    build: deploy/postgres-zh                # Dockerfile: 编译 zhparser + initdb 脚本
    environment:
      POSTGRES_DB: cricket
      POSTGRES_USER: cricket
      POSTGRES_PASSWORD_FILE: /run/secrets/pg_pass
    secrets: [pg_pass]
    volumes: [pgdata:/var/lib/postgresql/data]
    ports: ["127.0.0.1:5432:5432"]
    healthcheck: { test: ["CMD-SHELL", "pg_isready -U cricket"], interval: 10s }

  server:
    image: ghcr.io/<owner>/cricket-server:latest
    env_file: [/srv/cricket/.env]           # 密钥只在服务器文件系统，见 §4
    environment:
      DATABASE_URL: postgres://cricket@postgres/cricket   # 密码经 secret 注入
      KB_EMBED_DIM: "1024"
      SSE_RING_BUFFER: "2048"
      LOG_LEVEL: info
    ports: ["127.0.0.1:8080:8080"]
    depends_on: { postgres: { condition: service_healthy } }
    restart: unless-stopped
    logging: { options: { max-size: "10m", max-file: "3" } }

  searxng:
    image: searxng/searxng:latest
    volumes: [./searxng:/etc/searxng]        # 开启 search_formats: [json]
    ports: ["127.0.0.1:8888:8080"]
    restart: unless-stopped

  caddy:
    image: caddy:2
    ports: ["80:80", "443:443", "443:443/udp"]   # h2/h3
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - ./dist:/srv/static                   # Windows 更新 feed 与安装包
    restart: unless-stopped

volumes: { pgdata: {}, caddy_data: {} }
secrets: { pg_pass: { file: /srv/cricket/secrets/pg_pass } }
```

`Caddyfile`（SSE 关键：禁缓冲 + 长超时）：

```caddyfile
cricket.example.com {
    encode zstd gzip
    handle /api/v1/* {
        reverse_proxy 127.0.0.1:8080 {
            flush_interval -1            # ★ SSE 透传，禁响应缓冲
            transport http { read_timeout 10m write_timeout 10m }
        }
    }
    handle /static/* {
        root * /srv/static
        file_server
        header /static/* Cache-Control "public, max-age=3600"
    }
    header { Strict-Transport-Security "max-age=31536000" Referrer-Policy no-referrer }
}
```

---

## 3. `.env` 密钥清单（全部入 `.gitignore`，模板才入库）

```
CRICKET_API_KEYS__OPENAI=sk-…
CRICKET_API_KEYS__ANTHROPIC=sk-ant-…
CRICKET_API_KEYS__GEMINI=…
CRICKET_PAT_SIGNING_KEY=<openssl rand -hex 32>     # 端侧令牌签发
CRICKET_SKILL_HMAC_KEY=<openssl rand -hex 32>      # 远程 Skill 签名
CRICKET_ADMIN_PAT=<…>                                # 备份/管理端点专用
```

轮换策略：厂商 key 每季度轮换；PAT 签发密钥轮换 = 全端重新激活（App 有 UI 引导）。

---

## 4. TLS 与域名

- **推荐**：绑定一个域名（A 记录 → 124.223.154.233），Caddy 自动 ACME（Let's Encrypt）+ HTTP/3。SSE 必须走 TLS（PAT 明文头不可裸奔）。
- **无域名降级**（不推荐长期）：Caddy `tls internal` 自签 + 端侧**证书指纹校验**（Core 内置 pin，`rustls` `ServerCertVerifier` 自实现）——私有化内网场景仍加密可用。
- 端侧配置 `server_url` 支持直 IP:443 + 指纹模式，首次激活时校验并展示指纹（用户确认防中间人）。

---

## 5. 备份与恢复

| 对象 | 方案 | 频率 / 保留 |
|------|------|------------|
| PostgreSQL（**知识库真相源，最高优先级**） | `pg_dump -Fc` + 传输前 `gpg` 对称加密 + 上传对象存储（腾讯云 COS，生命周期规则） | 每日 03:30；保留 30 天 + 每周 1 份留 3 月 |
| compose / Caddyfile / .env（**密钥除外**） | 服务器配置目录 git 化（私有仓库） | 每次变更 |
| `.env` 与 secrets | 离线密码管理器副本（**不入任何仓库/备份桶**） | 每次轮换 |
| 会话数据 | 端为真相源（§00 §6），服务器镜像可由端侧 `sync/push` 重建 | — |

```bash
# /srv/cricket/backup.sh（cron: 30 3 * * *）
docker compose exec -T postgres pg_dump -U cricket -Fc cricket \
  | gpg --batch --yes --pinentry-mode loopback --passphrase-file /srv/cricket/secrets/gpg \
      -c -o /backup/cricket-$(date +%F).dump.gpg
rclone copy /backup/ cos:cricket-backup/pg --transfers 2     # COS 生命周期管保留
find /backup -mtime +7 -delete
```

**恢复演练**（M6 验收项 + 每季度例行）：新机 → `compose up postgres` → 解密 `gpg -d … | pg_restore` → `curl /healthz` + 端侧登录检索冒烟 → 记录 RTO（目标 ≤ 30 分钟）。

---

## 6. 监控与告警（零第三方依赖）

| 层 | 手段 |
|----|------|
| 存活 | Uptime Kuma 容器（同机部署但**仅绑 127.0.0.1**，经 Caddy 子域名带 BasicAuth 暴露）：`/healthz`、`/api/v1/version`、TCP 22 |
| 事件级 | `cricket-server` 内建 `/metrics`（Prometheus 文本格式：sse_active、事件环水位、relay 各厂商延迟直方图、kb_search P95）；`promtail` 可选 |
| 宿主机 | `node_exporter`（仅本机）；磁盘 > 85% 告警 |
| 告警通道 | Uptime Kuma → Telegram Bot / SMTP（用户自有） |
| 审计 | 每周自动生成用量报告（relay 请求数 / token / 成本估算），存 `/srv/cricket/reports/` |

SSE 专项观测：`sse_active` 突降（可能 Caddy/防火墙断流）+ 客户端重连率（服务端计数 `Last-Event-ID` 命中率）。

---

## 7. 发布与回滚

```
CI tag → 镜像 ghcr.io/<owner>/cricket-server:<tag>
       → 服务器: docker compose pull server && docker compose up -d --no-deps server
       → 健康探针: /healthz 5 次×10s 全过 → 放行流量（Caddy 不重启，连接级无损）
       → sqlx migrate 在容器入口自动执行（migrate 上锁，失败即退出不接流量）

回滚: docker tag 上一版 → up -d --no-deps → 观测
DB 兼容纪律: 迁移只增不删两版（端侧 iOS 有审核延迟，旧版本 App 需继续工作 ≥ 2 周）
```

灰次序：先服务端（兼容旧端）→ Windows（自动更新）→ iOS（TestFlight → 审核）。

---

## 8. 验收清单（M6 逐项打勾）

- [ ] §0 三步密钥轮换完成，`PasswordAuthentication no` 生效且本机验证
- [ ] ufw/fail2ban/unattended-upgrades 就位，`nmap` 外部扫描仅见 22/80/443
- [ ] 域名 + ACME 证书生效；SSE 经 Caddy 直通（`curl -N` 流式逐帧到达）
- [ ] compose 栈单命令冷启动 < 60s；`/healthz` 204
- [ ] pg_dump 桶内可取、可解密、可在空机恢复并通过端侧冒烟（RTO ≤ 30min）
- [ ] 监控全链在线，一次告警演练通过
- [ ] `.env` 无明文入仓（`gitleaks` CI 扫描绿）
- [ ] 服务器重装剧本（本 §1–§5 顺序执行清单）已文档化并演练一次
