const BUFFER_SIZE: usize = 65536;
const BUFFER_POOL_SIZE: usize = 1024;

// Пул буферов для избежания постоянных аллокаций
pub struct BufferPool {
    buffers: tokio::sync::Mutex<Vec<Vec<u8>>>,
}

impl BufferPool {
    pub fn new() -> Self {
        let mut buffers = Vec::with_capacity(BUFFER_POOL_SIZE);
        for _ in 0..BUFFER_POOL_SIZE {
            buffers.push(vec![0u8; BUFFER_SIZE]);
        }
        Self {
            buffers: tokio::sync::Mutex::new(buffers),
        }
    }

    pub async fn get_buffer(&self) -> Vec<u8> {
        let mut buffers = self.buffers.lock().await;
        buffers.pop().unwrap_or_else(|| vec![0u8; BUFFER_SIZE])
    }

    pub async fn return_buffer(&self, mut buf: Vec<u8>) {
        if buf.len() == BUFFER_SIZE {
            buf.fill(0); // Очищаем буфер
            let mut buffers = self.buffers.lock().await;
            if buffers.len() < BUFFER_POOL_SIZE {
                buffers.push(buf);
            }
        }
    }
}
