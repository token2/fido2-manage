#!/usr/bin/env python3
"""System-tray daemon for fido2-manage.

Shows a tray icon (via AppIndicator3) with Open / Quit menu items, and
auto-opens the GUI when a supported FIDO2 security key is inserted. Device
insertion is detected with pyudev so everything stays inside the user's
graphical session (udev itself has no display and cannot launch GUI apps).

The GTK/GLib main loop and the udev observer are only started under
``__main__`` so the module can be imported and unit-tested.
"""
import os
import subprocess
import sys

# Vendor ID of the target FIDO2 keys (Thetis = 0x1ea8). Additional vendor IDs
# can be added here to broaden auto-open support.
FIDO2_VENDOR_IDS = {"1ea8"}

# Resolve the GUI launcher next to this script.
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
GUI_PATH = os.path.join(SCRIPT_DIR, "gui.py")


def is_fido2_insertion(action, subsystem, vendor_id):
    """Return True if a udev event represents insertion of a supported key.

    action:    udev action string ("add", "remove", ...)
    subsystem: device subsystem ("hidraw", "usb", ...)
    vendor_id: lowercased 4-hex-digit vendor id, or None
    """
    if action != "add":
        return False
    if subsystem not in ("hidraw", "usb"):
        return False
    if not vendor_id:
        return False
    return vendor_id.lower() in FIDO2_VENDOR_IDS


def gui_launch_command():
    """Return the argv list used to launch the GUI."""
    return [sys.executable, GUI_PATH]


# Debounce: udev emits several events per physical insertion (one per hidraw
# node plus the usb device). Collapse them into a single launch, and never run
# more than one GUI at a time.
_DEBOUNCE_SECONDS = 3.0
_last_launch_ts = 0.0
_gui_process = None


def _display_available():
    """True if a usable X/Wayland display is present. Without this the launched
    GUI would crash immediately; relaunching in a loop can exhaust the X server.
    """
    return bool(os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY"))


def launch_gui(popen=subprocess.Popen, clock=None):
    """Launch the GUI, guarded against relaunch storms.

    - Skips launching if no display is available.
    - Debounces rapid repeat events within _DEBOUNCE_SECONDS.
    - Does not start a second GUI while one is still running.
    Returns the process handle if launched, else None. ``popen``/``clock`` are
    injectable for tests.
    """
    global _last_launch_ts, _gui_process
    import time as _time
    now = (clock or _time.monotonic)()

    if not _display_available():
        return None
    if now - _last_launch_ts < _DEBOUNCE_SECONDS:
        return None
    # If a previous GUI is still alive, don't spawn another.
    if _gui_process is not None and _gui_process.poll() is None:
        return None

    _last_launch_ts = now
    _gui_process = popen(gui_launch_command(), cwd=SCRIPT_DIR)
    return _gui_process


def _extract_vendor_id(device):
    """Best-effort extraction of a vendor id from a pyudev Device."""
    for key in ("ID_VENDOR_ID", "ID_USB_VENDOR_ID"):
        val = device.get(key)
        if val:
            return val.lower()
    return None


def handle_udev_event(device, launcher=launch_gui):
    """Handle a single pyudev event; launch the GUI on a matching insertion.
    Returns True if a launch was attempted (subject to launcher guards)."""
    action = getattr(device, "action", None)
    subsystem = getattr(device, "subsystem", None)
    vendor_id = _extract_vendor_id(device)
    if is_fido2_insertion(action, subsystem, vendor_id):
        launcher()
        return True
    return False


def build_menu(on_open, on_quit):
    """Build the tray menu with Open and Quit items. Imported lazily so the
    module can be imported without GTK present (e.g. in CI/tests)."""
    import gi
    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk

    menu = Gtk.Menu()

    open_item = Gtk.MenuItem(label="Open")
    open_item.connect("activate", on_open)
    menu.append(open_item)

    quit_item = Gtk.MenuItem(label="Quit")
    quit_item.connect("activate", on_quit)
    menu.append(quit_item)

    menu.show_all()
    return menu


def main():  # pragma: no cover - requires a live GTK session + tray
    import gi
    gi.require_version("Gtk", "3.0")
    gi.require_version("AppIndicator3", "0.1")
    from gi.repository import Gtk, GLib, AppIndicator3
    import pyudev

    indicator = AppIndicator3.Indicator.new(
        "fido2-manage-tray",
        "security-high",  # themed icon name
        AppIndicator3.IndicatorCategory.APPLICATION_STATUS,
    )
    indicator.set_status(AppIndicator3.IndicatorStatus.ACTIVE)

    def on_open(_widget):
        launch_gui()

    def on_quit(_widget):
        Gtk.main_quit()

    indicator.set_menu(build_menu(on_open, on_quit))

    # udev monitoring on a background thread; GUI launch is marshalled back to
    # the GLib main loop to stay thread-safe.
    context = pyudev.Context()
    monitor = pyudev.Monitor.from_netlink(context)

    def _observe(device):
        GLib.idle_add(handle_udev_event, device)

    observer = pyudev.MonitorObserver(monitor, callback=_observe)
    observer.start()

    Gtk.main()


if __name__ == "__main__":
    main()
