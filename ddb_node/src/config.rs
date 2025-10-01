use std::{
    fs,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::Path,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    bind_addr: SocketAddr,
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let data = fs::read_to_string(path).expect("config path should be openable");
		toml::from_str(&data).expect("config file should be formatted correctly")
    }

	pub fn bind_addr(&self) -> &SocketAddr {
		&self.bind_addr
	}
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr:  SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 2000)),
        }
    }
}
