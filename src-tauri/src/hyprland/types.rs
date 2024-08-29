use derive_more::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Display)]
pub struct Address(String);

impl Address {
  #[inline(always)]
  pub fn fmt_new(address: &str) -> Self {
    // this way is faster than std::fmt
    Self("0x".to_owned() + address)
  }
  /// This creates a new address from a value that implements [std::string::ToString]
  pub fn new<T: ToString>(string: T) -> Self {
    Self(string.to_string())
  }
}

/// This type provides the id used to identify workspaces
/// > its a type because it might change at some point
pub type WorkspaceId = i32;

pub type MonitorId = i128;

/// This enum defines the possible command flags that can be used.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandFlag {
  /// The JSON flag.
  #[default]
  JSON,
  /// An empty flag.
  Empty,
}

/// This struct defines the content of a command, which consists of a flag and a data string.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandContent {
  /// The flag for the command.
  pub flag: CommandFlag,
  /// The data string for the command.
  pub data: String,
}

impl CommandContent {
  /// Converts the command content to a byte vector.
  ///
  /// # Examples
  ///
  /// ```
  /// use hyprland::shared::*;
  ///
  /// let content = CommandContent { flag: CommandFlag::JSON, data: "foo".to_string() };
  /// let bytes = content.as_bytes();
  /// assert_eq!(bytes, b"j/foo");
  /// ```
  pub fn as_bytes(&self) -> Vec<u8> {
    self.to_string().into_bytes()
  }
}

impl std::fmt::Display for CommandContent {
  /// Formats the command content as a string for display.
  ///
  /// # Examples
  ///
  /// ```
  /// use hyprland::shared::*;
  ///
  /// let content = CommandContent { flag: CommandFlag::JSON, data: "foo".to_string() };
  /// let s = format!("{}", content);
  /// assert_eq!(s, "j/foo");
  /// ```
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.flag {
      CommandFlag::JSON => write!(f, "j/{}", &self.data),
      CommandFlag::Empty => write!(f, "/{}", &self.data),
    }
  }
}

/// Creates a `CommandContent` instance with the given flag and formatted data.
///
/// # Arguments
///
/// * `$flag` - A `CommandFlag` variant (`JSON` or `Empty`) that represents the flag for the command.
/// * `$($k:tt)*` - A format string and its arguments to be used as the data in the `CommandContent` instance.
#[macro_export]
macro_rules! command {
  ($flag:ident, $($k:tt)*) => {{
    use $crate::hyprland::types::CommandFlag;

    CommandContent {
      flag: CommandFlag::$flag,
      data: format!($($k)*),
    }
  }};
}
pub use command;
