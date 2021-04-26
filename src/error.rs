#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("failed to get tile at coordinate: ({0}, {1})")]
    GetTile(u16, u16),

    #[error("failed to draw to terminal")]
    DrawToTerminal(#[source] std::io::Error),

    #[error("failed to get input event")]
    GetEvent(#[source] std::sync::mpsc::RecvError),

    #[error("failed to get ctrlc handler")]
    SetHandler(#[source] ctrlc::Error),
}
