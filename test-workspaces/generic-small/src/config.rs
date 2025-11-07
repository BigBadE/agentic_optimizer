pub struct Config {
    pub host: String,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".to_owned(),
            port: 8080,
        }
    }
}
