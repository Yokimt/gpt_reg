use gpt_reg::moemail::{GenerateEmailRequest, MoEmailClient};
use headless_chrome::{Browser, LaunchOptions, protocol::cdp::Page};
use spider_fingerprint::{
    build_stealth_script,
    configs::{AgentOs, Tier},
};
use std::ffi::OsString;
use std::time::Duration;
use std::fs;
use regex::Regex;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("1. 生成 Stealth JS...");
    // 1. 生成最高级别的指纹伪装脚本
    let stealth_js = build_stealth_script(Tier::Full, AgentOs::Windows);

    let start_maximized = OsString::from("--start-maximized");
    let proxy_server = OsString::from("--proxy-server=http://127.0.0.1:7890");
    let host_resolver_rules =
        OsString::from("--host-resolver-rules=MAP * ~NOTFOUND , EXCLUDE 127.0.0.1");
    let webrtc_policy =
        OsString::from("--force-webrtc-ip-handling-policy=default_public_interface_only");
    let disable_automation = OsString::from("--disable-blink-features=AutomationControlled");

    println!("2. 配置并启动 headless_chrome...");
    // 2. 配置 Chrome 启动选项 (CDP 方式)
    let launch_options = LaunchOptions::default_builder()
        .headless(false) 
        .args(vec![
            start_maximized.as_os_str(),
            proxy_server.as_os_str(),
            host_resolver_rules.as_os_str(),
            webrtc_policy.as_os_str(),
            disable_automation.as_os_str(),
        ])
        .build()
        .expect("启动参数配置失败");

    // 启动浏览器并创建新标签页
    let browser = Browser::new(launch_options)?;
    let tab = browser.new_tab()?;

    // 设置默认的元素查找超时时间为 15 秒
    tab.set_default_timeout(Duration::from_secs(15));

    println!("3. 注入 Stealth JS...");
    // 3.使用 CDP 协议在任何网页加载前注入指纹伪装
    tab.call_method(Page::AddScriptToEvaluateOnNewDocument {
        source: stealth_js,
        world_name: None,
        include_command_line_api: None,
        run_immediately: None,
    })?;

    println!("4. 准备访问目标网站...");
    // 4. 访问高防护网站
    tab.navigate_to("https://chatgpt.com")?;
    // 等待页面加载完成和 CF 可能的 5 秒盾
    tab.wait_until_navigated()?;
    tokio::time::sleep(Duration::from_secs(5)).await;

    println!("5. 开始执行页面交互逻辑...");

    // 查找并点击注册按钮
    let btn = tab.wait_for_element("button[data-testid='signup-button']")?;
    btn.click()?;

    // 等待并查找邮箱输入框
    let email_input = tab.wait_for_element("input#email")?;

    // --- 异步调用邮箱 API ---
    let client = MoEmailClient::with_api_key(
        "https://api.url",
        "API_KEY", 
    );
    let config = client.get_config().await?;
    println!("config: {:?}", config);

    let mail = client
        .generate_email(GenerateEmailRequest::new("lanu.eu.org").expiry_time(3_600_000))
        .await?;
    println!("new email: {:?}", mail);
    // 输入邮箱
    email_input.type_into(&mail.email)?;

    // 点击提交按钮
    let submit_btn = tab.wait_for_element("button[type='submit']")?;
    submit_btn.click()?;

    // 等待密码输入框加载
    let pwd_input = tab.wait_for_element("input[name='new-password']")?;

    pwd_input.call_js_fn("function() { this.value = ''; }", vec![], false)?;

    let password = gpt_reg::rand_key::generate_strong_password(16)?;
    println!("生成的随机密码: {}", password);
    pwd_input.type_into(&password)?;
    let btn = tab.wait_for_element("button[data-dd-action-name='Continue'][type='submit']")?;
    btn.click()?;
    tokio::time::sleep(std::time::Duration::from_secs(15)).await;
    let email_messages = client.list_email_messages(&mail.id, None).await?;
    for msg in email_messages.messages {
        if msg.subject.find("ChatGPT验证码").is_some() {
            println!("找到验证码邮件，正在解析...");
            let re = Regex::new(r"(?:^|[^\w#])(\d{6})(?:[^\w]|$)").unwrap();
            let mesg = client.get_message(&mail.id, &msg.id).await?;
            if let Some(caps) = re.captures(&mesg.html) {
                let code = caps.get(1).unwrap().as_str();
                println!("提取到验证码: {}", code);
                let code_input = tab.wait_for_element("input[name='code']")?;
                code_input.click()?;
                code_input.type_into(code)?;
                let continue_btn =
                    tab.wait_for_element("button[data-dd-action-name='Continue']")?;
                continue_btn.click()?;
                let name_input = tab.wait_for_element("input[name='name']")?;
                name_input.click()?;
                let name = gpt_reg::rand_key::generate_random_lower_id(8);
                name_input.type_into(&name)?;
                let age_input = tab.wait_for_element("input[name='age']")?;
                age_input.click()?;
                age_input.type_into("25")?;
                let submit_btn = tab.wait_for_element("button[data-dd-action-name='Continue']")?;
                submit_btn.click()?;
                fs::write("./account_info.txt", format!("Email: {}\nPassword: {}", mail.email, password))?;
                break;
            } else {
                println!("未能从邮件内容中提取到验证码");
            }
        }
    }

    println!("自动化流程执行完毕，等待观察...");
    tokio::time::sleep(std::time::Duration::from_secs(1000)).await;

    // 清理资源不在这手动调用，当 browser 变量被 drop 时，它会自动清理。
    Ok(())
}
