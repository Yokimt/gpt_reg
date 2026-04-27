# gpt_reg

一个用 Rust 编写的自动化示例项目，结合浏览器自动化、指纹伪装、临时邮箱 API 和随机密码生成，演示如何完成一套“创建邮箱 - 接收验证码 - 继续填写表单”的流程。

## 功能

- 启动 Chrome/Chromium 并执行页面自动化操作
- 在页面加载前注入 Stealth JS，降低自动化特征
- 通过 MoEmail API 创建临时邮箱、读取邮件列表和邮件内容
- 从验证码邮件中提取 6 位验证码并回填页面
- 生成随机强密码和随机小写用户名
- 将账号信息写入本地文件 `account_info.txt`

## 项目结构

- `src/main.rs`：主流程入口，负责浏览器自动化和邮件验证码处理
- `src/moemail.rs`：MoEmail API 客户端封装
- `src/rand_key.rs`：随机密码和随机字符串生成工具
- `src/lib.rs`：模块导出

## 环境要求

- Rust 工具链
- 可用的 Chrome 或 Chromium
- 可访问的 MoEmail 服务地址和 API Key
- 如有需要，先配置本地代理，再运行程序

## 运行前配置

请先根据自己的环境修改 `src/main.rs` 里的这些配置：

- `MoEmailClient::with_api_key(...)` 中的 API 地址和 API Key
- `GenerateEmailRequest::new(...)` 中的邮箱域名
- 浏览器启动参数里的代理地址，如 `--proxy-server=http://127.0.0.1:7890`
- 目标网站地址和页面选择器

如果你打算把这个项目长期使用，建议把 API Key 改成环境变量或配置文件读取，不要直接写死在代码里。

## 构建与运行

```bash
cargo check
cargo run
```

## 主流程说明

当前示例程序大致会按下面的顺序执行：

1. 启动浏览器并注入 Stealth JS
2. 打开目标页面并进入注册/填写流程
3. 通过 MoEmail API 创建临时邮箱
4. 输入邮箱地址并继续提交
5. 等待验证码邮件到达
6. 读取邮件列表，筛选主题包含 `ChatGPT验证码` 的邮件
7. 打开邮件详情并提取 6 位验证码
8. 将验证码回填到页面中继续后续步骤
9. 生成并填写随机密码
10. 将邮箱和密码保存到 `account_info.txt`

## MoEmail 客户端示例

如果你只想单独使用邮件 API，可以这样调用：

```rust
use gpt_reg::moemail::{GenerateEmailRequest, MoEmailClient};

let client = MoEmailClient::with_api_key("https://your-domain.com", "YOUR_API_KEY");
let config = client.get_config().await?;
let mail = client
    .generate_email(GenerateEmailRequest::new("moemail.app").expiry_time(3_600_000))
    .await?;
let messages = client.list_email_messages(&mail.id, None).await?;
```

## 注意事项

- 代码中包含浏览器自动化逻辑，运行前请确认目标站点和页面选择器仍然有效
- 邮件验证码的提取规则目前按 6 位数字处理，如果邮件格式变化，需要同步调整正则
- `account_info.txt` 会保存生成的账号信息，请注意本地文件安全
