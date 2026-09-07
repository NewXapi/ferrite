use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// 异步 ping 套接字支持接口
///
/// 定义了基本的 ping 支持查询与写入方法。用于检测远端响应能力。
pub trait AsyncPing {
    /// 查询当前连接是否支持 ping 操作
    fn supports_ping(&self) -> bool;

    /// 异步写入 ping 消息
    ///
    /// # 参数
    /// - `self`: 可变 pin 引用
    /// - `cx`: 任务上下文
    ///
    /// # 返回
    /// 返回 Poll<io::Result<bool>>，表示 ping 是否已写入（true 表示已写入）。
    fn poll_write_ping(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<bool>>;
}

/// 异步流协议核心 trait，继承自 AsyncRead + AsyncWrite + AsyncPing
///
/// 定义了基本的 TCP 异步流抽象，提供 ping 支持查询与写入能力。
pub trait AsyncStream: AsyncRead + AsyncWrite + AsyncPing + Unpin + Send + Sync {}

/// TCP 流实现 AsyncStream 功能
impl AsyncPing for TcpStream {
    fn supports_ping(&self) -> bool {
        false
    }

    fn poll_write_ping(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<bool>> {
        Poll::Ready(Ok(false))
    }
}

impl AsyncStream for TcpStream {}

/// Unix socket 流实现 AsyncStream 功能（当目标系统支持时）。
/// 用 tokio 的 UnixStream——它实现了 AsyncRead/AsyncWrite；std 版没有。
#[cfg(target_family = "unix")]
impl AsyncPing for tokio::net::UnixStream {
    fn supports_ping(&self) -> bool {
        false
    }

    fn poll_write_ping(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<bool>> {
        Poll::Ready(Ok(false))
    }
}

#[cfg(target_family = "unix")]
impl AsyncStream for tokio::net::UnixStream {}

// pattern copied from deref_async_read macro: https://docs.rs/tokio/latest/src/tokio/io/async_read.rs.html#60
/// 为实现了 AsyncPing 和 Unpin 的 Box 类型提供 AsyncPing 实现
impl<T: ?Sized + AsyncPing + Unpin> AsyncPing for std::boxed::Box<T> {
    fn supports_ping(&self) -> bool {
        self.as_ref().supports_ping()
    }

    fn poll_write_ping(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<bool>> {
        let self_mut = self.get_mut();
        Pin::new(&mut **self_mut).poll_write_ping(cx)
    }
}

/// 为实现了 AsyncPing 和 Unpin 的 &mut 类型提供 AsyncPing 实现
impl<T: ?Sized + AsyncPing + Unpin> AsyncPing for &mut T {
    fn supports_ping(&self) -> bool {
        (**self).supports_ping()
    }

    fn poll_write_ping(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<bool>> {
        let self_mut = self.get_mut();
        Pin::new(&mut **self_mut).poll_write_ping(cx)
    }
}

/// 为实现了 AsyncStream 和 Unpin 的 Box 类型提供 AsyncStream 实现
impl<T: ?Sized + AsyncStream + Unpin> AsyncStream for std::boxed::Box<T> {}

/// 为实现了 AsyncStream 和 Unpin 的 &mut 类型提供 AsyncStream 实现
impl<T: ?Sized + AsyncStream + Unpin> AsyncStream for &mut T {}
