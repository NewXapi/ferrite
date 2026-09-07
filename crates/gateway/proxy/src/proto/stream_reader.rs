use memchr::memchr;
use tokio::io::AsyncReadExt;

/// 流读取器，提供高效的行读取、字节读取和 peek 操作
///
/// 此结构体在读取流时维护内部缓冲区，支持高效的行解析（必须以 CRLF 结尾）、
/// 字节读取、peek 操作以及未解析数据的访问。设计用于协议解析场景。
///
/// # 内部实现
/// - 使用固定大小缓冲区（默认 32KB）
/// - 支持 peek 操作而不消耗缓冲区
/// - 自动处理缓冲区偏移以支持高效读取
/// - 遇到 bare LF（无 CR）时默认抛出错误
///
/// # 使用示例
/// ```rust
/// use gateway_proxy::proto::stream_reader::StreamReader;
/// use tokio::net::TcpStream;
///
/// async fn example() {
///     let mut stream = TcpStream::connect("example.com:80").await.unwrap();
///     let mut reader = StreamReader::new();
///     let line = reader.read_line(&mut stream).await.unwrap();
///     println!("Received line: {}", line);
/// }
/// ```
pub struct StreamReader {
    buf: Box<[u8]>,
    start_offset: usize,
    end_offset: usize,
}

const DEFAULT_BUFFER_SIZE: usize = 32768;
const ERROR_ON_BARE_LF: bool = true;

impl StreamReader {
    /// 创建默认缓冲区大小的 StreamReader
    ///
    /// 默认缓冲区大小为 32KB。
    pub fn new() -> Self {
        Self::new_with_buffer_size(DEFAULT_BUFFER_SIZE)
    }

    /// 使用指定缓冲区大小创建 StreamReader
    ///
    /// # 参数
    /// - `buffer_size`: 缓冲区大小（字节）。同时决定最大可读取的行长度。
    ///
    /// # 注意
    /// 缓冲区大小会影响最大行长度限制。
    pub fn new_with_buffer_size(buffer_size: usize) -> Self {
        Self {
            buf: Self::allocate_vec(buffer_size).into_boxed_slice(),
            start_offset: 0usize,
            end_offset: 0usize,
        }
    }

    /// 分配一个未初始化的 Vec（性能优化）
    ///
    /// 此函数使用 unsafe 操作来避免初始化开销。
    /// 仅供本模块内部使用。
    #[inline]
    #[allow(clippy::uninit_vec)]
    fn allocate_vec<T>(len: usize) -> Vec<T> {
        let mut ret = Vec::with_capacity(len);
        unsafe {
            ret.set_len(len);
        }
        ret
    }

    fn reset_buf_offset(&mut self) {
        if self.start_offset == 0 {
            return;
        }
        self.buf.copy_within(self.start_offset..self.end_offset, 0);
        self.end_offset -= self.start_offset;
        self.start_offset = 0;
    }

    /// 读取一行字节（必须以 CRLF 结尾）
    ///
    /// # 参数
    /// - `stream`: 异步读取流
    ///
    /// # 返回
    /// 行内容的字节切片引用（不包含 CRLF）
    ///
    /// # 错误
    /// - 如果行以 bare LF 结尾（无 CR），返回 InvalidData 错误
    /// - 其他读取错误
    pub async fn read_line_bytes<T: AsyncReadExt + Unpin>(
        &mut self,
        stream: &mut T,
    ) -> std::io::Result<&mut [u8]> {
        let mut search_start_offset = self.start_offset;
        loop {
            let search_end_offset = self.end_offset;
            match memchr(b'\n', &self.buf[search_start_offset..search_end_offset]) {
                Some(pos) => {
                    let newline_pos = search_start_offset + pos;
                    if newline_pos == self.start_offset || self.buf[newline_pos - 1] != b'\r' {
                        if ERROR_ON_BARE_LF {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Line is not terminated by CRLF",
                            ));
                        } else {
                            search_start_offset = newline_pos + 1;
                            continue;
                        }
                    }
                    // Strips CRLF.
                    let line = &mut self.buf[self.start_offset..newline_pos - 1];
                    let new_start_offset = newline_pos + 1;
                    if new_start_offset == search_end_offset {
                        self.start_offset = 0;
                        self.end_offset = 0;
                    } else {
                        self.start_offset = new_start_offset;
                    }
                    return Ok(line);
                }
                None => {
                    // There are no more newlines.
                    let previous_start_offset = self.start_offset;

                    self.read(stream).await?;

                    // Only searches through new data.
                    if previous_start_offset != self.start_offset {
                        // Can only move to zero when reset_buf_offset is called.
                        assert!(self.start_offset == 0);
                        search_start_offset = search_end_offset - previous_start_offset;
                    } else {
                        search_start_offset = search_end_offset;
                    }
                }
            }
        }
    }

    /// 读取一行字符串（必须以 CRLF 结尾）
    ///
    /// # 参数
    /// - `stream`: 异步读取流
    ///
    /// # 返回
    /// 行内容的 UTF-8 字符串切片
    ///
    /// # 错误
    /// - 如果行以 bare LF 结尾，返回 InvalidData 错误
    /// - UTF-8 解码失败，返回 InvalidData 错误
    pub async fn read_line<T: AsyncReadExt + Unpin>(
        &mut self,
        stream: &mut T,
    ) -> std::io::Result<&str> {
        let line_bytes = self.read_line_bytes(stream).await?;
        std::str::from_utf8(line_bytes).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to decode utf8: {e}"),
            )
        })
    }

    /// 读取一个字节
    ///
    /// # 参数
    /// - `stream`: 异步读取流
    ///
    /// # 返回
    /// 读取到的字节
    pub async fn read_u8<T: AsyncReadExt + Unpin>(
        &mut self,
        stream: &mut T,
    ) -> std::io::Result<u8> {
        while self.end_offset - self.start_offset < 1 {
            self.read(stream).await?;
        }
        let value = self.buf[self.start_offset];
        let new_start_offset = self.start_offset + 1;
        if new_start_offset == self.end_offset {
            self.start_offset = 0;
            self.end_offset = 0;
        } else {
            self.start_offset = new_start_offset;
        }
        Ok(value)
    }

    /// 查看第一个字节但不消耗它
    ///
    /// # 参数
    /// - `stream`: 异步读取流
    ///
    /// # 返回
    /// 第一个字节
    pub async fn peek_u8<T: AsyncReadExt + Unpin>(
        &mut self,
        stream: &mut T,
    ) -> std::io::Result<u8> {
        while self.end_offset - self.start_offset < 1 {
            self.read(stream).await?;
        }
        // Returns the byte without advancing start_offset.
        Ok(self.buf[self.start_offset])
    }

    /// 查看前 N 个字节但不消耗它们
    ///
    /// # 参数
    /// - `stream`: 异步读取流
    /// - `len`: 要查看的字节数
    ///
    /// # 返回
    /// 字节切片
    pub async fn peek_slice<T: AsyncReadExt + Unpin + ?Sized>(
        &mut self,
        stream: &mut T,
        len: usize,
    ) -> std::io::Result<&[u8]> {
        if len > self.buf.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Requested length {} exceeds buffer size {}",
                    len,
                    self.buf.len()
                ),
            ));
        }
        while self.end_offset - self.start_offset < len {
            self.read(stream).await?;
        }
        // Returns the slice without advancing start_offset.
        Ok(&self.buf[self.start_offset..self.start_offset + len])
    }

    /// 消耗之前 peek 的字节
    ///
    /// # 参数
    /// - `len`: 要消耗的字节数
    ///
    /// # 注意
    /// 必须在 peek_slice 或 peek_u8 之后调用，且 len 必须小于等于之前 peek 的长度。
    pub fn consume(&mut self, len: usize) {
        let new_start_offset = self.start_offset + len;
        debug_assert!(new_start_offset <= self.end_offset);
        if new_start_offset == self.end_offset {
            self.start_offset = 0;
            self.end_offset = 0;
        } else {
            self.start_offset = new_start_offset;
        }
    }

    /// 读取一个 u16 大端字节序整数
    ///
    /// # 参数
    /// - `stream`: 异步读取流
    ///
    /// # 返回
    /// 读取到的 u16 值
    pub async fn read_u16_be<T: AsyncReadExt + Unpin>(
        &mut self,
        stream: &mut T,
    ) -> std::io::Result<u16> {
        while self.end_offset - self.start_offset < 2 {
            self.read(stream).await?;
        }
        let value =
            u16::from_be_bytes([self.buf[self.start_offset], self.buf[self.start_offset + 1]]);
        let new_start_offset = self.start_offset + 2;
        if new_start_offset == self.end_offset {
            self.start_offset = 0;
            self.end_offset = 0;
        } else {
            self.start_offset = new_start_offset;
        }
        Ok(value)
    }

    /// 读取指定长度的字节切片
    ///
    /// # 参数
    /// - `stream`: 异步读取流
    /// - `len`: 要读取的字节数
    ///
    /// # 返回
    /// 读取到的字节切片
    pub async fn read_slice<T: AsyncReadExt + Unpin + ?Sized>(
        &mut self,
        stream: &mut T,
        len: usize,
    ) -> std::io::Result<&[u8]> {
        if len > self.buf.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Requested length {} exceeds buffer size {}",
                    len,
                    self.buf.len()
                ),
            ));
        }
        while self.end_offset - self.start_offset < len {
            self.read(stream).await?;
        }
        let slice = &self.buf[self.start_offset..self.start_offset + len];
        let new_start_offset = self.start_offset + len;
        if new_start_offset == self.end_offset {
            self.start_offset = 0;
            self.end_offset = 0;
        } else {
            self.start_offset = new_start_offset;
        }
        Ok(slice)
    }

    /// 将指定长度的字节读取到提供的缓冲区
    ///
    /// # 参数
    /// - `stream`: 异步读取流
    /// - `buf`: 目标缓冲区
    pub async fn read_slice_into<T: AsyncReadExt + Unpin>(
        &mut self,
        stream: &mut T,
        buf: &mut [u8],
    ) -> std::io::Result<()> {
        let slice = self.read_slice(stream, buf.len()).await?;
        buf.copy_from_slice(slice);
        Ok(())
    }

    /// 获取当前未解析的缓冲数据
    ///
    /// # 返回
    /// 当前缓冲区中未被解析的字节切片
    pub fn unparsed_data(&self) -> &[u8] {
        &self.buf[self.start_offset..self.end_offset]
    }

    /// 获取当前未解析数据的拥有权拷贝
    ///
    /// # 返回
    /// 如果有未解析数据则返回 Some(Box<[u8]>)，否则返回 None
    pub fn unparsed_data_owned(&self) -> Option<Box<[u8]>> {
        let unparsed_data = self.unparsed_data();
        if unparsed_data.is_empty() {
            None
        } else {
            Some(unparsed_data.to_vec().into_boxed_slice())
        }
    }

    async fn read<T: AsyncReadExt + Unpin + ?Sized>(
        &mut self,
        stream: &mut T,
    ) -> std::io::Result<()> {
        if self.is_cache_full() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "cache is full",
            ));
        }

        self.reset_buf_offset();

        loop {
            match stream.read(&mut self.buf[self.end_offset..]).await {
                Ok(len) => {
                    if len == 0 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::ConnectionAborted,
                            "EOF while reading",
                        ));
                    }
                    self.end_offset += len;
                    return Ok(());
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    fn is_cache_full(&self) -> bool {
        self.start_offset == 0 && self.end_offset == self.buf.len()
    }
}

impl Default for StreamReader {
    fn default() -> Self {
        Self::new()
    }
}
