//! Wi-Fi firmware flash protocol.
//!
//! The bike exposes a Wi-Fi AP during firmware update mode.  Flashing
//! happens over raw TCP sockets:
//!
//! | Port | Purpose                                 |
//! |------|-----------------------------------------|
//! |    7 | Console — text commands (`a`, `x`, etc.) |
//! |  777 | Blob transfer (`.fwb` firmware file)     |
//! |  877 | ESP-TOP binary (`vcu-lfs-top.bin`)       |
//!
//! See `PROTOCOL.md` for the full flash sequence.

use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

const BIKE_IP: &str = "192.168.167.1";
const PORT_CONSOLE: u16 = 7;
const PORT_BLOB: u16 = 777;
const PORT_ESP_TOP: u16 = 877;

const CONSOLE_TIMEOUT: Duration = Duration::from_secs(10);
const BLOB_TIMEOUT: Duration = Duration::from_secs(30);
const ESP_TOP_TIMEOUT: Duration = Duration::from_secs(20);
const ESP_REBOOT_WAIT: Duration = Duration::from_secs(20);

const FLASH_BLOCK_SIZE: usize = 4096;

/// Progress updates emitted during a flash operation.
#[derive(Debug, Clone)]
pub enum FlashProgress {
    EnteringFlashMenu,
    SendingEspTop { bytes_sent: usize, total: usize },
    WaitingForEspReboot,
    TransferringBlob { bytes_sent: usize, total: usize },
    ReloadingBlob,
    ExitingMenu,
    Done,
    Failed(String),
}

impl std::fmt::Display for FlashProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnteringFlashMenu => write!(f, "Entering flash menu..."),
            Self::SendingEspTop { bytes_sent, total } => {
                let pct = if *total > 0 {
                    *bytes_sent * 100 / *total
                } else {
                    0
                };
                write!(f, "Sending ESP-TOP binary... {pct}% ({bytes_sent}/{total})")
            }
            Self::WaitingForEspReboot => write!(f, "Waiting for ESP reboot (20s)..."),
            Self::TransferringBlob { bytes_sent, total } => {
                let pct = if *total > 0 {
                    *bytes_sent * 100 / *total
                } else {
                    0
                };
                write!(f, "Transferring blob... {pct}% ({bytes_sent}/{total})")
            }
            Self::ReloadingBlob => write!(f, "Reloading blob..."),
            Self::ExitingMenu => write!(f, "Exiting flash menu..."),
            Self::Done => write!(f, "Flash complete."),
            Self::Failed(msg) => write!(f, "Flash failed: {msg}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Low-level helpers
// ---------------------------------------------------------------------------

/// Open a TCP connection to the bike with a timeout.
async fn connect(port: u16, timeout: Duration) -> anyhow::Result<TcpStream> {
    let addr = format!("{BIKE_IP}:{port}");
    let stream = tokio::time::timeout(timeout, TcpStream::connect(&addr))
        .await
        .map_err(|_| anyhow::anyhow!("connection to {addr} timed out"))??;
    Ok(stream)
}

/// Send a text command on the console port (7).
///
/// Each command opens a fresh TCP connection, matching the official app's
/// behaviour.
async fn send_console_command(cmd: &str) -> anyhow::Result<()> {
    let mut stream = connect(PORT_CONSOLE, CONSOLE_TIMEOUT).await?;
    stream.write_all(cmd.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;
    // Give the bike a moment to process before we drop the socket.
    tokio::time::sleep(Duration::from_millis(500)).await;
    Ok(())
}

/// Send a binary payload to a port in chunks, reporting progress.
async fn send_binary(
    port: u16,
    data: &[u8],
    timeout: Duration,
    mut progress: impl FnMut(usize, usize),
) -> anyhow::Result<()> {
    let mut stream = connect(port, timeout).await?;

    let total = data.len();
    let mut sent = 0usize;
    // Send in 4 KiB chunks so we can report progress.
    for chunk in data.chunks(FLASH_BLOCK_SIZE) {
        tokio::time::timeout(timeout, stream.write_all(chunk))
            .await
            .map_err(|_| anyhow::anyhow!("write to port {port} timed out"))??;
        sent += chunk.len();
        progress(sent, total);
    }

    stream.flush().await?;
    Ok(())
}

/// Pad `data` to a 4096-byte boundary by filling with `0xFF`.
fn pad_to_block(data: &[u8]) -> Vec<u8> {
    let remainder = data.len() % FLASH_BLOCK_SIZE;
    if remainder == 0 {
        return data.to_vec();
    }
    let padded_len = data.len() + (FLASH_BLOCK_SIZE - remainder);
    let mut buf = vec![0xFFu8; padded_len];
    buf[..data.len()].copy_from_slice(data);
    buf
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Execute the full firmware flash sequence.
///
/// Either or both of `esp_top` and `blob` may be provided.  The sequence
/// adapts accordingly:
///
/// 1. Send `"a"` on console to enter flash menu.
/// 2. If `esp_top` is `Some`, send the ESP-TOP binary over port 877, then
///    wait 20 s for the ESP to reboot.
/// 3. If `blob` is `Some`, pad it to a 4096-byte boundary, send it over
///    port 777, then send `"reload-blob"` on the console.
/// 4. Send `"x"` on console to exit flash menu.
///
/// `on_progress` is called with each state change so the UI can display
/// meaningful feedback.
pub async fn flash(
    esp_top: Option<&[u8]>,
    blob: Option<&[u8]>,
    on_progress: impl Fn(FlashProgress) + Send,
) -> anyhow::Result<()> {
    if esp_top.is_none() && blob.is_none() {
        anyhow::bail!("nothing to flash — provide at least one file");
    }

    // 1. Enter flash menu.
    on_progress(FlashProgress::EnteringFlashMenu);
    send_console_command("a").await?;

    // 2. ESP-TOP binary.
    if let Some(esp_data) = esp_top {
        on_progress(FlashProgress::SendingEspTop {
            bytes_sent: 0,
            total: esp_data.len(),
        });
        send_binary(PORT_ESP_TOP, esp_data, ESP_TOP_TIMEOUT, |sent, total| {
            on_progress(FlashProgress::SendingEspTop {
                bytes_sent: sent,
                total,
            });
        })
        .await?;

        on_progress(FlashProgress::WaitingForEspReboot);
        tokio::time::sleep(ESP_REBOOT_WAIT).await;
    }

    // 3. Firmware blob.
    if let Some(blob_data) = blob {
        let padded = pad_to_block(blob_data);
        on_progress(FlashProgress::TransferringBlob {
            bytes_sent: 0,
            total: padded.len(),
        });
        send_binary(PORT_BLOB, &padded, BLOB_TIMEOUT, |sent, total| {
            on_progress(FlashProgress::TransferringBlob {
                bytes_sent: sent,
                total,
            });
        })
        .await?;

        on_progress(FlashProgress::ReloadingBlob);
        send_console_command("reload-blob").await?;
    }

    // 4. Exit flash menu.
    on_progress(FlashProgress::ExitingMenu);
    send_console_command("x").await?;

    on_progress(FlashProgress::Done);
    Ok(())
}
