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

    #[error("failed to get stdout in raw mode")]
    GetStdoutInRawMode(#[source] std::io::Error),

    #[error("failed to create terminal object")]
    CreateTerminal(#[source] std::io::Error),

    #[error("failed to parse number from string: {1}")]
    ParseNum(#[source] std::num::ParseIntError, String),

    #[error("value must be greater than zero")]
    GetNonZeroValue,

    #[error("failed to convert u32 to usize")]
    ConvertU32ToUsize(#[source] std::num::TryFromIntError),
}
