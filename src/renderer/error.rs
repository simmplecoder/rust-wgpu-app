use eframe::wgpu;

#[derive(Debug, thiserror::Error)]
pub enum ComputeRendererError {
    #[error("device poll failed: {0}")]
    DevicePoll(#[from] wgpu::PollError),

    #[error("failed waiting for mapped buffer: {0}")]
    MapWait(#[from] std::sync::mpsc::RecvError),

    #[error("failed to map output buffer: {0}")]
    BufferMap(String),
}
