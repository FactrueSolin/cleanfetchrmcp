# 项目介绍

- 基于 Rust + rmcp 的 MCP 服务器（适配 rmcp 0.14.0）
- 通过 Selenium 获取网页内容
- 支持 httpstream 协议

# 项目部署

仅支持 Docker Compose 部署：

1. 复制 `.env.example` 为 `.env`
2. 配置环境变量：见下方「环境变量配置（.env）」
3. 运行：

```bash
docker compose up -d
```

服务端点：`http://localhost:13006/mcp`

## 环境变量配置（.env）

本项目通过环境变量控制 **Selenium 连接地址**、**服务监听端口**、**可选代理**与 **MCP 接口鉴权**。

> 说明：代码会对部分值进行 `trim()`（去除首尾空白），并将“未设置 / 空字符串”视为未配置。

### 配置示例

```env
# Selenium Remote WebDriver 地址（Docker Compose 场景通常指向 selenium 容器）
SELENIUM_URL=http://selenium:4444

# 本服务监听端口
PORT=13006

# 可选：为 Selenium 浏览器设置 HTTP/HTTPS 代理（不需要可留空）
PROXY_URL=127.0.0.1:7891

# 可选：为 /mcp 端点启用 Token 鉴权（留空则不启用鉴权）
MCP_AUTH_TOKEN=your-strong-token
```

### 字段说明

| 变量名           |     必填 | 作用                                                                                                               | 可选值 / 格式                                                                                                        | 示例                   | 默认行为                             |
| ---------------- | -------: | ------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- | ---------------------- | ------------------------------------ |
| `SELENIUM_URL`   | 建议填写 | Selenium Remote WebDriver 地址，用于通过 Selenium 拉取网页 HTML。Docker Compose 部署时通常需要指向 Selenium 容器。 | URL 字符串。常见为 `http://<host>:4444`，部分 Selenium 镜像也会使用 `http://<host>:4444/wd/hub`。                    | `http://selenium:4444` | 未设置时使用 `http://127.0.0.1:4444` |
| `PORT`           |     可选 | 本服务 HTTP 监听端口（对外暴露 `/mcp`）。                                                                          | `1`~`65535` 的整数（Rust `u16`）。                                                                                   | `13006`                | 未设置或解析失败时使用 `3000`        |
| `PROXY_URL`      |     可选 | 为 Selenium 浏览器设置代理（同时用于 `httpProxy` 与 `sslProxy`）。适合在需要走代理访问目标站点时启用。             | 代理地址字符串。通常为 `<host>:<port>`；是否需要协议前缀取决于你的 Selenium/浏览器环境，建议优先使用不带协议的写法。 | `127.0.0.1:7891`       | 未设置/空值时不配置代理              |
| `MCP_AUTH_TOKEN` |     可选 | 为 `/mcp` 端点启用 Token 鉴权：设置后，调用方需携带正确 Token 才能访问 MCP 服务。                                  | 任意非空字符串。建议使用随机长串（避免弱口令）。                                                                     | `your-strong-token`    | 未设置/空值时禁用鉴权                |

### 必填/可选项建议

- **Docker Compose 部署**：建议显式设置 `SELENIUM_URL` 与 `PORT`（与 `docker-compose.yml` 的端口映射保持一致）；`PROXY_URL`、`MCP_AUTH_TOKEN` 按需设置。
- **本地直连 Selenium**：若你本机 Selenium 在 `http://127.0.0.1:4444`，则可不设置 `SELENIUM_URL`；`PORT` 不设置时默认 `3000`。

# 项目功能介绍

提供 4 个 MCP 工具：

- `fetch_markdown`：获取网页并转换为 Markdown 格式
- `fetch_txt`：获取网页并转换为纯文本
- `fetch_urls`：提取网页中的所有链接
- `fetch_html`：获取网页原始 HTML

每个工具参数：

- `url` (string)：目标 URL
- `max_length` (number, 可选)：最大字数限制，默认 `128000`
