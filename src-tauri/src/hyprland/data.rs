use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::types::{MonitorId, WorkspaceId};

/// This struct holds a basic identifier for a workspace often used in other structs
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceBasic {
  /// The workspace Id
  pub id: WorkspaceId,
  /// The workspace's name
  pub name: String,
}

/// This enum provides the different monitor transforms
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, PartialEq, Eq, Copy)]
#[repr(u8)]
pub enum Transforms {
  /// No transform
  Normal = 0,
  /// Rotated 90 degrees
  Normal90 = 1,
  /// Rotated 180 degrees
  Normal180 = 2,
  /// Rotated 270 degrees
  Normal270 = 3,
  /// Flipped
  Flipped = 4,
  /// Flipped and rotated 90 degrees
  Flipped90 = 5,
  /// Flipped and rotated 180 degrees
  Flipped180 = 6,
  /// Flipped and rotated 270 degrees
  Flipped270 = 7,
}

/// This struct holds information for a monitor
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Monitor {
  /// The monitor id
  pub id: MonitorId,
  /// The monitor's name
  pub name: String,
  /// The monitor's description
  pub description: String,
  /// The monitor width (in pixels)
  pub width: u16,
  /// The monitor height (in pixels)
  pub height: u16,
  /// The monitor's refresh rate (in hertz)
  #[serde(rename = "refreshRate")]
  pub refresh_rate: f32,
  /// The monitor's position on the x axis (not irl ofc)
  pub x: i32,
  /// The monitor's position on the x axis (not irl ofc)
  pub y: i32,
  /// A basic identifier for the active workspace
  #[serde(rename = "activeWorkspace")]
  pub active_workspace: WorkspaceBasic,
  /// Reserved is the amount of space (in pre-scale pixels) that a layer surface has claimed
  pub reserved: (u16, u16, u16, u16),
  /// The display's scale
  pub scale: f32,
  /// I think like the rotation?
  pub transform: Transforms,
  /// a string that identifies if the display is active
  pub focused: bool,
  /// The dpms status of a monitor
  #[serde(rename = "dpmsStatus")]
  pub dpms_status: bool,
  /// VRR state
  pub vrr: bool,
}
