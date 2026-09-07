use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// 地址表示类型，支持 IPv4、IPv6 和主机名解析
///
/// 此枚举用于表示网络层的地址类型。当地址为主机名时，
/// 地址不包含端口号，端口号另存于 [`NetLocation`] 中。
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Address {
    /// IPv4 地址
    Ipv4(Ipv4Addr),
    /// IPv6 地址
    Ipv6(Ipv6Addr),
    /// 域名地址
    Hostname(String),
}

impl Address {
    /// 未指定地址的常量
    pub const UNSPECIFIED: Self = Address::Ipv4(Ipv4Addr::UNSPECIFIED);

    /// 从字符串解析地址
    ///
    /// # 参数
    /// - `s`: 地址字符串，支持 IPv4、IPv6 和主机名格式
    ///
    /// # 返回
    /// 如果解析成功，返回 [`Address`] 枚举实例，否则返回 `std::io::Error`
    ///
    /// # 支持的格式
    /// - 标准 IPv4 地址，如 "192.168.1.1"
    /// - 标准 IPv6 地址，如 "2001:db8::1"
    /// - 主机名，如 "example.com"
    ///
    /// # 示例
    /// ```rust
    /// use gateway_proxy::proto::address::Address;
    ///
    /// let addr = Address::from("192.168.1.1").unwrap();
    /// assert!(matches!(addr, Address::Ipv4(_)));
    /// ```
    pub fn from(s: &str) -> std::io::Result<Self> {
        let mut dots = 0;
        let mut possible_ipv4 = true;
        let mut possible_ipv6 = true;
        let mut possible_hostname = true;
        for b in s.as_bytes().iter() {
            let c = *b;
            if c == b':' {
                possible_ipv4 = false;
                possible_hostname = false;
                break;
            } else if c == b'.' {
                possible_ipv6 = false;
                dots += 1;
                if dots > 3 {
                    // 只能是主机名
                    break;
                }
            } else if (b'A'..=b'F').contains(&c) || (b'a'..=b'f').contains(&c) {
                possible_ipv4 = false;
            } else if !c.is_ascii_digit() {
                possible_ipv4 = false;
                possible_ipv6 = false;
                break;
            }
        }

        if possible_ipv4
            && dots == 3
            && let Ok(addr) = s.parse::<Ipv4Addr>()
        {
            return Ok(Address::Ipv4(addr));
        }

        if possible_ipv6 && let Ok(addr) = s.parse::<Ipv6Addr>() {
            return Ok(Address::Ipv6(addr));
        }

        if possible_hostname {
            return Ok(Address::Hostname(s.to_string()));
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse address: {s}"),
        ))
    }

    /// 检查地址是否为 IPv6 类型
    ///
    /// # 返回
    /// 如果地址为 IPv6 类型则返回 `true`，否则返回 `false`
    pub fn is_ipv6(&self) -> bool {
        matches!(self, Address::Ipv6(_))
    }

    /// 获取地址的主机名表示
    ///
    /// # 返回
    /// 如果地址为主机名则返回其字符串切片，否则返回 `None`
    pub fn hostname(&self) -> Option<&str> {
        match self {
            Address::Hostname(hostname) => Some(hostname),
            _ => None,
        }
    }
}

impl fmt::Display for Address {
    /// 显示地址的字符串表示
    ///
    /// # 格式
    /// - IPv4 地址：如 "192.168.1.1"
    /// - IPv6 地址：如 "2001:db8::1"
    /// - 主机名：如 "example.com"
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Ipv4(i) => write!(f, "{i}"),
            Address::Ipv6(i) => write!(f, "{i}"),
            Address::Hostname(h) => write!(f, "{h}"),
        }
    }
}

/// 网络位置，包含地址和端口号
///
/// 代表一个完整的网络端点，包括 IP 地址/主机名和端口号。
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct NetLocation {
    address: Address,
    port: u16,
}

impl NetLocation {
    /// 未指定网络位置的常量
    pub const UNSPECIFIED: Self = NetLocation::new(Address::UNSPECIFIED, 0);

    /// 创建新的网络位置
    ///
    /// # 参数
    /// - `address`: 地址类型
    /// - `port`: 端口号
    ///
    /// # 返回
    /// 新的 [`NetLocation`] 实例
    pub const fn new(address: Address, port: u16) -> Self {
        Self { address, port }
    }

    /// 检查网络位置是否为未指定状态
    ///
    /// # 返回
    /// 如果网络位置为未指定则返回 `true`，否则返回 `false`
    pub fn is_unspecified(&self) -> bool {
        self == &Self::UNSPECIFIED
    }

    /// 从字符串解析网络位置
    ///
    /// # 参数
    /// - `s`: 网络位置字符串，如 "192.168.1.1:8080" 或 "[::1]:80"
    /// - `default_port`: 可选的默认端口，当字符串中没有指定端口时使用
    ///
    /// # 返回
    /// 如果解析成功，返回 [`NetLocation`] 实例，否则返回 `std::io::Error`
    ///
    /// # 支持的格式
    /// - 标准地址:端口组合，如 "192.168.1.1:8080"
    /// - IPv6 地址:端口组合，如 "[::1]:80"
    /// - 主机名:端口组合，如 "example.com:8080"
    ///
    /// # 示例
    /// ```rust
    /// use gateway_proxy::proto::address::NetLocation;
    ///
    /// let loc = NetLocation::from_str("192.168.1.1:8080", None).unwrap();
    /// assert_eq!(loc.port(), 8080);
    /// ```
    pub fn from_str(s: &str, default_port: Option<u16>) -> std::io::Result<Self> {
        let (address_str, port, expect_ipv6) = match s.rfind(':') {
            Some(i) => {
                // 冒号可能是 IPv6 地址的一部分
                match s[i + 1..].parse::<u16>() {
                    Ok(port) => (&s[0..i], Some(port), false),
                    Err(_) => (s, default_port, true),
                }
            }
            None => (s, default_port, false),
        };

        let address = Address::from(address_str)?;
        if expect_ipv6 && !address.is_ipv6() {
            return Err(std::io::Error::other("Invalid location"));
        }

        let port = port.ok_or_else(|| std::io::Error::other("No port"))?;

        Ok(Self { address, port })
    }

    /// 从 IP 地址和端口号创建网络位置
    ///
    /// # 参数
    /// - `ip`: IP 地址
    /// - `port`: 端口号
    ///
    /// # 返回
    /// 新的 [`NetLocation`] 实例
    ///
    /// # 示例
    /// ```rust
    /// use gateway_proxy::proto::address::NetLocation;
    /// use std::net::IpAddr;
    ///
    /// let loc = NetLocation::from_ip_addr(IpAddr::V4([192, 168, 1, 1].into()), 8080);
    /// assert_eq!(loc.port(), 8080);
    /// ```
    /// 从 `IpAddr` 直构（跳过字符串解析；测试与已知 IP 场景使用）
    pub fn from_ip_addr(ip: IpAddr, port: u16) -> Self {
        let address = match ip {
            IpAddr::V4(addr) => Address::Ipv4(addr),
            IpAddr::V6(addr) => Address::Ipv6(addr),
        };
        Self { address, port }
    }

    /// 获取地址部分
    ///
    /// # 返回
    /// 地址的不可变引用
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// 获取端口号
    ///
    /// # 返回
    /// 端口号
    pub fn port(&self) -> u16 {
        self.port
    }

    /// 将网络位置转换为非阻塞的 SocketAddr
    ///
    /// # 返回
    /// 如果地址为 IP 地址（IPv4 或 IPv6），返回相应的 [`SocketAddr`]，否则返回 `None`
    ///
    /// # 注意
    /// 当地址为主机名时，无法转换，因为需要 DNS 解析。这通常在稍后阶段完成。
    pub fn to_socket_addr_nonblocking(&self) -> Option<SocketAddr> {
        match self.address {
            Address::Ipv6(addr) => Some(SocketAddr::new(IpAddr::V6(addr), self.port)),
            Address::Ipv4(addr) => Some(SocketAddr::new(IpAddr::V4(addr), self.port)),
            Address::Hostname(ref _d) => None,
        }
    }
}

impl fmt::Display for NetLocation {
    /// 显示网络位置的字符串表示
    ///
    /// # 格式
    /// "地址:端口"，如 "192.168.1.1:8080" 或 "example.com:80"
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.address, self.port)
    }
}

/// 网络位置，附带可选的预解析地址
///
/// 当 `resolved_addr` 为 `Some` 时，消费者应直接使用该地址
/// 而不是执行额外的 DNS 解析，以避免重复解析同一主机名的情况。
/// 这通常发生在连接管道（规则匹配、套接字连接、协议设置）中。
#[derive(Debug, Clone)]
pub struct ResolvedLocation {
    location: NetLocation,
    resolved_addr: Option<SocketAddr>,
}

impl ResolvedLocation {
    /// 创建一个未解析的网络位置
    ///
    /// # 参数
    /// - `location`: 网络位置
    ///
    /// # 返回
    /// 新的 [`ResolvedLocation`] 实例
    pub fn new(location: NetLocation) -> Self {
        Self {
            location,
            resolved_addr: None,
        }
    }

    /// 创建一个带有预解析地址的网络位置
    ///
    /// # 参数
    /// - `location`: 网络位置
    /// - `addr`: 已解析的套接字地址
    ///
    /// # 返回
    /// 新的 [`ResolvedLocation`] 实例
    pub fn with_resolved(location: NetLocation, addr: SocketAddr) -> Self {
        Self {
            location,
            resolved_addr: Some(addr),
        }
    }

    /// 获取基础网络位置
    ///
    /// # 返回
    /// 网络位置的不可变引用
    pub fn location(&self) -> &NetLocation {
        &self.location
    }

    /// 消费自身并返回基础网络位置
    ///
    /// # 返回
    /// 网络位置
    pub fn into_location(self) -> NetLocation {
        self.location
    }

    /// 获取预解析的地址
    ///
    /// # 返回
    /// 如果有预解析地址则返回 `Some(SocketAddr)`，否则返回 `None`
    pub fn resolved_addr(&self) -> Option<SocketAddr> {
        self.resolved_addr
    }

    /// 获取端口号（转发 [`NetLocation::port`]）
    pub fn port(&self) -> u16 {
        self.location.port()
    }

    /// 获取地址
    ///
    /// # 返回
    /// 地址的不可变引用
    pub fn address(&self) -> &Address {
        self.location.address()
    }

    /// 设置解析后的地址
    ///
    /// # 参数
    /// - `addr`: 解析后的套接字地址
    ///
    /// # 注意
    /// 此方法通常由解析器在进行懒解析时使用
    pub fn set_resolved(&mut self, addr: SocketAddr) {
        self.resolved_addr = Some(addr);
    }
}

impl fmt::Display for ResolvedLocation {
    /// 显示解析后的网络位置的字符串表示
    ///
    /// # 格式
    /// - 如果有解析后的地址："地址:端口 (resolved: 解析后的地址)"
    /// - 否则："地址:端口"
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.resolved_addr {
            Some(addr) => write!(f, "{} (resolved: {})", self.location, addr),
            None => write!(f, "{}", self.location),
        }
    }
}

impl From<NetLocation> for ResolvedLocation {
    /// 将 [`NetLocation`] 转换为 [`ResolvedLocation`]
    ///
    /// # 参数
    /// - `location`: 网络位置
    ///
    /// # 返回
    /// 新的 [`ResolvedLocation`] 实例，带有 `None` 的解析地址
    fn from(location: NetLocation) -> Self {
        Self::new(location)
    }
}

impl From<&NetLocation> for ResolvedLocation {
    /// 将 [`NetLocation`] 的引用转换为 [`ResolvedLocation`]
    ///
    /// # 参数
    /// - `location`: 网络位置的引用
    ///
    /// # 返回
    /// 新的 [`ResolvedLocation`] 实例，带有 `None` 的解析地址
    fn from(location: &NetLocation) -> Self {
        Self::new(location.clone())
    }
}
