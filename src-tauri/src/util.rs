use anyhow::Result;
use gdk::{glib::translate::ToGlibPtr, prelude::*};
use gtk::{prelude::*, Widget};
use rand::{distributions::Alphanumeric, Rng};
use wayland_backend::client::ObjectId;
use wayland_client::{protocol::wl_surface::WlSurface, Connection, Proxy};

pub fn get_current_username() -> Option<String> {
  let uid = unsafe { libc::getuid() };
  let mut passwd = unsafe { std::mem::zeroed::<libc::passwd>() };
  let mut buf = vec![0; 2048];
  let mut result = std::ptr::null_mut::<libc::passwd>();

  loop {
    let r = unsafe { libc::getpwuid_r(uid, &mut passwd, buf.as_mut_ptr(), buf.len(), &mut result) };

    if r != libc::ERANGE {
      break;
    }

    let newsize = buf.len().checked_mul(2)?;
    buf.resize(newsize, 0);
  }

  if result.is_null() {
    return None;
  }

  if result != &mut passwd {
    return None;
  }

  let raw = unsafe { std::ffi::CStr::from_ptr(result.read().pw_name) };
  Some(String::from(raw.to_string_lossy()))
}

pub fn rand_string() -> String {
  rand::thread_rng()
    .sample_iter(&Alphanumeric)
    .take(32)
    .map(char::from)
    .collect()
}

pub fn get_wl_window<T: IsA<Widget>>(gtk_window: &T) -> Result<gdkwayland::WaylandWindow> {
  let result = gtk_window
    .window()
    .ok_or(anyhow::anyhow!("window has no window"))?
    .downcast::<gdkwayland::WaylandWindow>()
    .map_err(|_| anyhow::anyhow!("window is not wayland"))?;

  Ok(result)
}

pub fn get_wl_surface(
  conn: &Connection,
  wl_window: &gdkwayland::WaylandWindow,
) -> Result<WlSurface> {
  let ptr =
    unsafe { gdk_wayland_sys::gdk_wayland_window_get_wl_surface(wl_window.to_glib_none().0) };

  let surface = unsafe { ObjectId::from_ptr(WlSurface::interface(), ptr as *mut _)? };

  Ok(WlSurface::from_id(conn, surface)?)
}
