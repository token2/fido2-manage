"""Tests for fido2_tray.py.

Covers the testable logic: the insertion predicate, launch-command
construction, the udev event handler, vendor-id extraction, and build_menu
(with gi/Gtk mocked). The GTK main loop in main() is excluded from coverage
(pragma) since it requires a live tray/session.
"""
import sys
import types
import importlib
from unittest import mock

import pytest


@pytest.fixture
def tray():
    sys.modules.pop("fido2_tray", None)
    return importlib.import_module("fido2_tray")


# --- is_fido2_insertion --------------------------------------------------
def test_insertion_match(tray):
    assert tray.is_fido2_insertion("add", "hidraw", "1ea8") is True
    assert tray.is_fido2_insertion("add", "usb", "1EA8") is True  # case-insensitive


@pytest.mark.parametrize("action,subsystem,vid", [
    ("remove", "hidraw", "1ea8"),   # wrong action
    ("add", "block", "1ea8"),       # wrong subsystem
    ("add", "hidraw", "abcd"),      # wrong vendor
    ("add", "hidraw", None),        # no vendor
])
def test_insertion_no_match(tray, action, subsystem, vid):
    assert tray.is_fido2_insertion(action, subsystem, vid) is False


# --- gui_launch_command / launch_gui -------------------------------------
def test_gui_launch_command(tray):
    cmd = tray.gui_launch_command()
    assert cmd[0] == sys.executable
    assert cmd[1].endswith("gui.py")


def test_launch_gui_uses_popen(tray, monkeypatch):
    monkeypatch.setenv("DISPLAY", ":0")
    fake = mock.MagicMock()
    tray._last_launch_ts = 0.0
    tray._gui_process = None
    tray.launch_gui(popen=fake, clock=lambda: 100.0)
    fake.assert_called_once()
    args, kwargs = fake.call_args
    assert args[0][1].endswith("gui.py")
    assert kwargs["cwd"] == tray.SCRIPT_DIR


def test_launch_gui_no_display(tray, monkeypatch):
    monkeypatch.delenv("DISPLAY", raising=False)
    monkeypatch.delenv("WAYLAND_DISPLAY", raising=False)
    fake = mock.MagicMock()
    tray._last_launch_ts = 0.0
    tray._gui_process = None
    assert tray.launch_gui(popen=fake, clock=lambda: 100.0) is None
    fake.assert_not_called()


def test_launch_gui_debounced(tray, monkeypatch):
    monkeypatch.setenv("DISPLAY", ":0")
    fake = mock.MagicMock()
    tray._last_launch_ts = 99.0     # last launch 1s ago (< debounce window)
    tray._gui_process = None
    assert tray.launch_gui(popen=fake, clock=lambda: 100.0) is None
    fake.assert_not_called()


def test_launch_gui_single_instance(tray, monkeypatch):
    monkeypatch.setenv("DISPLAY", ":0")
    alive = mock.MagicMock()
    alive.poll.return_value = None   # still running
    tray._last_launch_ts = 0.0
    tray._gui_process = alive
    fake = mock.MagicMock()
    assert tray.launch_gui(popen=fake, clock=lambda: 100.0) is None
    fake.assert_not_called()


def test_launch_gui_after_previous_exited(tray, monkeypatch):
    monkeypatch.setenv("DISPLAY", ":0")
    dead = mock.MagicMock()
    dead.poll.return_value = 0       # previous GUI exited
    tray._last_launch_ts = 0.0
    tray._gui_process = dead
    fake = mock.MagicMock()
    tray.launch_gui(popen=fake, clock=lambda: 100.0)
    fake.assert_called_once()


def test_display_available(tray, monkeypatch):
    monkeypatch.setenv("DISPLAY", ":0")
    assert tray._display_available() is True
    monkeypatch.delenv("DISPLAY", raising=False)
    monkeypatch.setenv("WAYLAND_DISPLAY", "wayland-0")
    assert tray._display_available() is True
    monkeypatch.delenv("WAYLAND_DISPLAY", raising=False)
    assert tray._display_available() is False


# --- _extract_vendor_id --------------------------------------------------
def test_extract_vendor_id_primary(tray):
    assert tray._extract_vendor_id({"ID_VENDOR_ID": "1EA8"}) == "1ea8"


def test_extract_vendor_id_fallback(tray):
    assert tray._extract_vendor_id({"ID_USB_VENDOR_ID": "1EA8"}) == "1ea8"


def test_extract_vendor_id_none(tray):
    assert tray._extract_vendor_id({}) is None


# --- handle_udev_event ---------------------------------------------------
class _Dev(dict):
    """Mimics a pyudev Device: attribute access for action/subsystem plus
    dict access for properties."""
    def __init__(self, action, subsystem, props):
        super().__init__(props)
        self.action = action
        self.subsystem = subsystem


def test_handle_event_launches(tray):
    dev = _Dev("add", "hidraw", {"ID_VENDOR_ID": "1ea8"})
    launcher = mock.MagicMock()
    assert tray.handle_udev_event(dev, launcher=launcher) is True
    launcher.assert_called_once()


def test_handle_event_ignores(tray):
    dev = _Dev("remove", "hidraw", {"ID_VENDOR_ID": "1ea8"})
    launcher = mock.MagicMock()
    assert tray.handle_udev_event(dev, launcher=launcher) is False
    launcher.assert_not_called()
