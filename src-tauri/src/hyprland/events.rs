use std::{collections::HashSet, sync::Mutex};

use anyhow::{anyhow, bail};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::error;

use super::types::{Address, WorkspaceId};

/// This enum holds workspace data
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum WorkspaceType {
  /// A named workspace
  Regular(
    /// The name
    String,
  ),
  /// The special workspace
  Special(
    /// The name, if exists
    Option<String>,
  ),
}

/// This tuple struct holds window event data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowEventData {
  /// The window class
  pub window_class: String,
  /// The window title
  pub window_title: String,
  /// The window address
  pub window_address: Address,
}

/// This tuple struct holds window event data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowTitleEventData {
  /// The window address
  pub window_address: Address,
  /// The window title
  pub window_title: String,
}

/// This tuple struct holds monitor event data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorEventData {
  /// The monitor name
  pub monitor_name: String,
  /// The workspace
  pub workspace: WorkspaceType,
}

/// This tuple struct holds monitor event data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowFloatEventData {
  /// The window address
  pub window_address: Address,
  /// The float state
  pub is_floating: bool,
}

/// Event data for v2 workspace events
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEventData {
  /// Workspace Id
  pub workspace_id: WorkspaceId,
  /// Workspace name
  pub workspace_name: String,
}

/// Event data for a minimize event
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinimizeEventData {
  /// Window address
  pub window_address: Address,
  /// whether it's minimized or not
  pub is_minimized: bool,
}

/// Event data for screencast event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreencastEventData {
  /// State/Is it turning on?
  pub is_turning_on: bool,
  /// Owner type, is it a monitor?
  pub is_monitor: bool,
}

/// The data for the event executed when moving a window to a new workspace
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowMoveEvent {
  /// Window address
  pub window_address: Address,
  /// The workspace name
  pub workspace_name: String,
}

/// The data for the event executed when opening a new window
#[derive(Clone, Debug)]
pub struct WindowOpenEvent {
  /// Window address
  pub window_address: Address,
  /// The workspace name
  pub workspace_name: String,
  /// Window class
  pub window_class: String,
  /// Window title
  pub window_title: String,
}

/// The data for the event executed when changing keyboard layouts
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutEvent {
  /// Keyboard name
  pub keyboard_name: String,
  /// Layout name
  pub layout_name: String,
}

/// The mutable state available to Closures
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct State {
  /// The active workspace
  pub active_workspace: WorkspaceType,
  /// The active monitor
  pub active_monitor: String,
  /// The fullscreen state
  pub fullscreen_state: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
  Workspace(WorkspaceEventData),
  DestroyWorkspace(WorkspaceEventData),
  CreateWorkspace(WorkspaceEventData),
  MoveWorkspace(MonitorEventData),
  RenameWorkspace(WorkspaceEventData),
  ActiveWindow(Option<Address>),
  FocusedMon(MonitorEventData),
  Fullscreen(bool),
  MonitorAdded(String),
  MonitorRemoved(String),
  OpenWindow(WindowOpenEvent),
  CloseWindow(Address),
  MoveWindow(WindowMoveEvent),
  ActiveLayout(LayoutEvent),
  SubMap(String),
  OpenLayer(String),
  CloseLayer(String),
  ChangeFloatingMode(WindowFloatEventData),
  Urgent(Address),
  Minimize(MinimizeEventData),
  WindowTitle(WindowTitleEventData),
  Screencast(ScreencastEventData),
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
enum ParsedEventType {
  Workspace,
  WorkspaceV2,
  DestroyWorkspace,
  DestroyWorkspaceV2,
  CreateWorkspace,
  CreateWorkspaceV2,
  MoveWorkspace,
  RenameWorkspace,
  ActiveWindow,
  ActiveWindowV2,
  FocusedMon,
  Fullscreen,
  MonitorAdded,
  MonitorRemoved,
  OpenWindow,
  CloseWindow,
  MoveWindow,
  ActiveLayout,
  SubMap,
  OpenLayer,
  CloseLayer,
  ChangeFloatingMode,
  Urgent,
  Minimize,
  WindowTitle,
  WindowTitleV2,
  Screencast,
  Unknown,
}

/// All the recognized events
static EVENT_SET: Lazy<Box<[(ParsedEventType, Regex)]>> = Lazy::new(|| {
  [
    (
      ParsedEventType::Workspace,
      r"\bworkspace>>(?P<workspace>.*)",
    ),
    (
      ParsedEventType::WorkspaceV2,
      r"\bworkspacev2>>(?P<id>.*),(?P<name>.*)",
    ),
    (
      ParsedEventType::DestroyWorkspace,
      r"destroyworkspace>>(?P<id>.*)",
    ),
    (
      ParsedEventType::DestroyWorkspaceV2,
      r"destroyworkspacev2>>(?P<id>.*),(?P<name>.*)",
    ),
    (
      ParsedEventType::CreateWorkspace,
      r"createworkspace>>(?P<workspace>.*)",
    ),
    (
      ParsedEventType::CreateWorkspaceV2,
      r"createworkspacev2>>(?P<id>.*),(?P<name>.*)",
    ),
    (
      ParsedEventType::MoveWorkspace,
      r"moveworkspace>>(?P<workspace>.*),(?P<monitor>.*)",
    ),
    (
      ParsedEventType::RenameWorkspace,
      r"renameworkspace>>(?P<id>.*),(?P<name>.*)",
    ),
    (
      ParsedEventType::FocusedMon,
      r"focusedmon>>(?P<monitor>.*),(?P<workspace>.*)",
    ),
    (
      ParsedEventType::ActiveWindow,
      r"activewindow>>(?P<class>.*?),(?P<title>.*)",
    ),
    (
      ParsedEventType::ActiveWindowV2,
      r"activewindowv2>>(?P<address>.*)",
    ),
    (ParsedEventType::Fullscreen, r"fullscreen>>(?P<state>0|1)"),
    (
      ParsedEventType::MonitorRemoved,
      r"monitorremoved>>(?P<monitor>.*)",
    ),
    (
      ParsedEventType::MonitorAdded,
      r"monitoradded>>(?P<monitor>.*)",
    ),
    (
      ParsedEventType::OpenWindow,
      r"openwindow>>(?P<address>.*),(?P<workspace>.*),(?P<class>.*),(?P<title>.*)",
    ),
    (
      ParsedEventType::CloseWindow,
      r"closewindow>>(?P<address>.*)",
    ),
    (
      ParsedEventType::MoveWindow,
      r"movewindow>>(?P<address>.*),(?P<workspace>.*)",
    ),
    (
      ParsedEventType::ActiveLayout,
      r"activelayout>>(?P<keyboard>.*)(?P<layout>.*)",
    ),
    (ParsedEventType::SubMap, r"submap>>(?P<submap>.*)"),
    (ParsedEventType::OpenLayer, r"openlayer>>(?P<namespace>.*)"),
    (
      ParsedEventType::CloseLayer,
      r"closelayer>>(?P<namespace>.*)",
    ),
    (
      ParsedEventType::ChangeFloatingMode,
      r"changefloatingmode>>(?P<address>.*),(?P<floatstate>[0-1])",
    ),
    (
      ParsedEventType::Minimize,
      r"minimize>>(?P<address>.*),(?P<state>[0-1])",
    ),
    (
      ParsedEventType::Screencast,
      r"screencast>>(?P<state>[0-1]),(?P<owner>[0-1])",
    ),
    (ParsedEventType::Urgent, r"urgent>>(?P<address>.*)"),
    (
      ParsedEventType::WindowTitle,
      r"windowtitle>>(?P<address>.*)",
    ),
    (
      ParsedEventType::WindowTitleV2,
      r"windowtitlev2>>(?P<address>.*),(?P<title>.*)",
    ),
    (ParsedEventType::Unknown, r"(?P<Event>^[^>]*)"),
  ]
  .into_iter()
  .map(|(e, r)| {
    (
      e,
      match Regex::new(r) {
        Ok(value) => value,
        Err(e) => match e {
          regex::Error::Syntax(str) => panic!("Regex syntax error: {str}"),
          regex::Error::CompiledTooBig(size) => {
            panic!("The compiled regex size is too big! ({size})")
          }
          _ => panic!("Error compiling regex: {e}"),
        },
      },
    )
  })
  .collect()
});

static CHECK_TABLE: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

fn parse_workspace(str: String) -> WorkspaceType {
  if str == "special" {
    WorkspaceType::Special(None)
  } else if str.starts_with("special:") {
    {
      let mut iter = str.split(':');
      iter.next();
      match iter.next() {
        Some(name) => WorkspaceType::Special(Some(name.to_string())),
        None => WorkspaceType::Special(None),
      }
    }
  } else {
    WorkspaceType::Regular(str)
  }
}

/// This internal function parses event strings
pub fn parse_event(event: String) -> crate::Result<Vec<Event>> {
  let event_iter = event
    .trim()
    .lines()
    .map(|event_line| {
      let type_matches = EVENT_SET
        .iter()
        .filter_map(|(event_type, regex)| Some((event_type, regex.captures(event_line)?)))
        .collect::<Vec<_>>();

      (event_line, type_matches)
    })
    .filter(|(_, b)| !b.is_empty());

  let mut temp_event_holder = Vec::new();

  for (event_str, matches) in event_iter {
    match matches.len() {
      0 => error!("No regex matches found for event: {event_str}"),
      1 => {
        error!("Unknown event: {event_str}");
        continue;
      }
      2 => {
        let (event_type, captures) = match matches
          .into_iter()
          .find(|(e, _)| **e != ParsedEventType::Unknown)
        {
          Some(t) => t,
          None => {
            error!("Only unknown events captured");
            continue;
          }
        };

        temp_event_holder.push((event_str, event_type, captures));
      }
      _ => error!("Event matched more than one regex"),
    }
  }

  let mut events: Vec<Event> = Vec::with_capacity(temp_event_holder.len());
  for (event_str, event_type, captures) in temp_event_holder.iter() {
    match event_type {
      // Ignored events, v2 exists for these
      ParsedEventType::Workspace
      | ParsedEventType::DestroyWorkspace
      | ParsedEventType::CreateWorkspace
      | ParsedEventType::ActiveWindow
      | ParsedEventType::WindowTitle => continue,

      ParsedEventType::WorkspaceV2 => events.push(Event::Workspace(WorkspaceEventData {
        workspace_id: captures["id"]
          .parse::<WorkspaceId>()
          .map_err(|e| anyhow!("workspacev2: invalid integer error: {e}"))?,
        workspace_name: captures["name"].to_string(),
      })),
      ParsedEventType::DestroyWorkspaceV2 => {
        events.push(Event::DestroyWorkspace(WorkspaceEventData {
          workspace_id: captures["id"]
            .parse::<WorkspaceId>()
            .map_err(|e| anyhow!("destroyworkspacev2: invalid integer error: {e}"))?,
          workspace_name: captures["name"].to_string(),
        }))
      }
      ParsedEventType::CreateWorkspaceV2 => {
        events.push(Event::CreateWorkspace(WorkspaceEventData {
          workspace_id: captures["id"]
            .parse::<WorkspaceId>()
            .map_err(|e| anyhow!("createworkspacev2 v2: invalid integer error: {e}"))?,
          workspace_name: captures["name"].to_string(),
        }))
      }
      ParsedEventType::MoveWorkspace => events.push(Event::MoveWorkspace(MonitorEventData {
        monitor_name: captures["monitor"].to_string(),
        workspace: parse_workspace(captures["workspace"].to_string()),
      })),
      ParsedEventType::RenameWorkspace => events.push(Event::RenameWorkspace(WorkspaceEventData {
        workspace_id: captures["id"]
          .parse::<WorkspaceId>()
          .map_err(|e| anyhow!("Workspace rename: invalid integer error: {e}"))?,
        workspace_name: captures["name"].to_string(),
      })),
      ParsedEventType::FocusedMon => events.push(Event::FocusedMon(MonitorEventData {
        monitor_name: captures["monitor"].to_string(),
        workspace: WorkspaceType::Regular(captures["workspace"].to_string()),
      })),
      ParsedEventType::ActiveWindowV2 => {
        let addr = &captures["address"];
        let event = if addr != "," {
          Event::ActiveWindow(Some(Address::fmt_new(addr)))
        } else {
          Event::ActiveWindow(None)
        };
        events.push(event);
      }
      ParsedEventType::Fullscreen => {
        let state = &captures["state"] != "0";
        events.push(Event::Fullscreen(state))
      }
      ParsedEventType::MonitorRemoved => {
        events.push(Event::MonitorRemoved(captures["monitor"].to_string()))
      }
      ParsedEventType::MonitorAdded => {
        events.push(Event::MonitorAdded(captures["monitor"].to_string()))
      }
      ParsedEventType::OpenWindow => events.push(Event::OpenWindow(WindowOpenEvent {
        window_address: Address::fmt_new(&captures["address"]),
        workspace_name: captures["workspace"].to_string(),
        window_class: captures["class"].to_string(),
        window_title: captures["title"].to_string(),
      })),
      ParsedEventType::CloseWindow => {
        events.push(Event::CloseWindow(Address::fmt_new(&captures["address"])))
      }
      ParsedEventType::MoveWindow => events.push(Event::MoveWindow(WindowMoveEvent {
        window_address: Address::fmt_new(&captures["address"]),
        workspace_name: captures["workspace"].to_string(),
      })),
      ParsedEventType::ActiveLayout => events.push(Event::ActiveLayout(LayoutEvent {
        keyboard_name: captures["keyboard"].to_string(),
        layout_name: captures["layout"].to_string(),
      })),
      ParsedEventType::SubMap => events.push(Event::SubMap(captures["submap"].to_string())),
      ParsedEventType::OpenLayer => {
        events.push(Event::OpenLayer(captures["namespace"].to_string()))
      }
      ParsedEventType::CloseLayer => {
        events.push(Event::CloseLayer(captures["namespace"].to_string()))
      }
      ParsedEventType::ChangeFloatingMode => {
        let state = &captures["floatstate"] == "0";
        events.push(Event::ChangeFloatingMode(WindowFloatEventData {
          window_address: Address::fmt_new(&captures["address"]),
          is_floating: state,
        }))
      }
      ParsedEventType::Minimize => {
        let state = &captures["state"] == "1";
        events.push(Event::Minimize(MinimizeEventData {
          window_address: Address::fmt_new(&captures["address"]),
          is_minimized: state,
        }))
      }
      ParsedEventType::Screencast => {
        let state = &captures["state"] == "1";
        let owner = &captures["owner"] == "1";
        events.push(Event::Screencast(ScreencastEventData {
          is_turning_on: state,
          is_monitor: owner,
        }))
      }
      ParsedEventType::Urgent => events.push(Event::Urgent(Address::fmt_new(&captures["address"]))),
      ParsedEventType::WindowTitleV2 => events.push(Event::WindowTitle(WindowTitleEventData {
        window_title: captures["title"].to_string(),
        window_address: Address::fmt_new(&captures["address"]),
      })),
      ParsedEventType::Unknown => {
        let table = CHECK_TABLE.lock();

        if let Ok(mut tbl) = table {
          let (event_string, print_str) = match captures.name("event").map(|s| s.as_str()) {
            Some(s) => (s.to_string(), s),
            None => ("Unknown".to_owned(), *event_str),
          };

          let should_run = tbl.insert(event_string);
          if should_run {
            error!("Unknown event: {print_str}");
          }
        }

        bail!("unknown event: {event_str}")
      }
    }
  }

  Ok(events)
}
