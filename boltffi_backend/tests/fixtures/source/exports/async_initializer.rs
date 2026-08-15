pub struct Client {
    port: u32,
}

#[export(single_threaded)]
impl Client {
    pub fn new() -> Self {
        Self { port: 0 }
    }

    pub async fn connect(port: u32) -> Self {
        Self { port }
    }

    pub fn port(&self) -> u32 {
        self.port
    }
}
