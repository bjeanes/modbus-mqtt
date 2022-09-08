use tokio::sync::mpsc::Sender;

use crate::{modbus::register::Register, mqtt};

/// Describes the register to Home Assistant
fn configure(_register: Register, _tx: Sender<mqtt::Message>) -> crate::Result<()> {
    Ok(())
}
