//! gateway：单二进制 CLI。
//! 只做 argv 解析 + 装配 + 信号处理，不含业务逻辑。见 docs/08-mvp.md §3.3
mod cmd;
mod middleware;
mod routes;
mod server;
mod shutdown;
mod state;

fn main() {
    // 子命令：run（默认）/ check / stat / errors / test / models
    eprintln!("gateway: 骨架阶段，尚未实现。见 docs/08-mvp.md");
    std::process::exit(1);
}
