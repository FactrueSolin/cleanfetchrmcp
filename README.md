# 项目介绍

- 基于 Rust + rmcp 的 MCP 服务器（适配 rmcp 0.14.0）
- 通过 Selenium 获取网页内容
- 支持 httpstream 协议

# 项目部署

仅支持 Docker Compose 部署：

1. 复制 `.env.example` 为 `.env`
2. 配置环境变量：
   - `SELENIUM_URL`：Selenium Remote WebDriver 地址（如 `http://selenium:4444/wd/hub`）
   - `PORT`：服务监听端口（示例配置使用 `13006`）
   - `PROXY_URL`：可选代理地址（不需要可留空）
   - `MCP_AUTH_TOKEN`：MCP 调用鉴权 Token
3. 运行：

```bash
docker compose up -d
```

服务端点：`http://localhost:13006/mcp`

# 项目功能介绍

提供 4 个 MCP 工具：

- `fetch_markdown`：获取网页并转换为 Markdown 格式
- `fetch_txt`：获取网页并转换为纯文本
- `fetch_urls`：提取网页中的所有链接
- `fetch_html`：获取网页原始 HTML

每个工具参数：

- `url` (string)：目标 URL
- `max_length` (number, 可选)：最大字数限制，默认 `128000`
