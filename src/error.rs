#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("failed to get tile at coordinate: {0:?}")]
    GetTile((usize, usize)),

    #[error("failed to draw to terminal")]
    DrawToTerminal(#[source] std::io::Error),

    #[error("failed to get input event")]
    GetEvent(#[source] std::sync::mpsc::RecvError),

    #[error("failed to get ctrlc handler")]
    SetHandler(#[source] ctrlc::Error),

    #[error("failed to get stdout in raw mode")]
    GetStdoutInRawMode(#[source] std::io::Error),

    #[error("failed to get alternate screen for mouse terminal")]
    GetAlternateScreenForMouseTerminal(#[source] std::io::Error),

    #[error("failed to create terminal object")]
    CreateTerminal(#[source] std::io::Error),

    #[error("failed to convert usize to u16")]
    ConvertUsizeToU16(#[source] std::num::TryFromIntError),
}
