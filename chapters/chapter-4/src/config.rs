#[derive(Debug)]
pub struct ServerConfig {
    pub max_body_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_body_size: 2 << 20, // 2 MiB
        }
    }
}

#[derive(Debug)]
pub struct ServerConfigBuilder {
    max_body_size: usize,
}

impl ServerConfigBuilder {
    pub fn new() -> Self {
        let default_config = ServerConfig::default();

        Self {
            max_body_size: default_config.max_body_size,
        }
    }
    pub fn max_body_size(mut self, size: usize) -> Self {
        self.max_body_size = size;
        self
    }
    pub fn build(self) -> ServerConfig {
        ServerConfig {
            max_body_size: self.max_body_size,
        }
    }
}
