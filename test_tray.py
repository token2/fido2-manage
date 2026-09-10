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


def test_launch_gui_uses_popen(tray):
    fake = mock.MagicMock()
    tray.launch_gui(popen=fake)
    fake.assert_called_once()
    args, kwargs = fake.call_args
    assert args[0][1].endswith("gui.py")
    assert kwargs["cwd"] == tray.SCRIPT_DIR


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


# --- build_menu (mock gi/Gtk) --------------------------------------------
def test_build_menu(tray, monkeypatch):
    gi = types.ModuleType("gi")
    gi.require_version = mock.MagicMock()
    repository = types.ModuleType("gi.repository")

    class _Item:
        def __init__(self, label=None):
            self.label = label
            self._cb = None
        def connect(self, sig, cb):
            self._cb = cb
        def get_callback(self):
            return self._cb

    class _Menu:
        def __init__(self):
            self.items = []
            self.shown = False
        def append(self, item):
            self.items.append(item)
        def show_all(self):
            self.shown = True

    Gtk = types.SimpleNamespace(Menu=_Menu, MenuItem=_Item)
    repository.Gtk = Gtk
    gi.repository = repository
    monkeypatch.setitem(sys.modules, "gi", gi)
    monkeypatch.setitem(sys.modules, "gi.repository", repository)

    on_open = mock.MagicMock()
    on_quit = mock.MagicMock()
    menu = tray.build_menu(on_open, on_quit)
    assert [i.label for i in menu.items] == ["Open", "Quit"]
    menu.items[0].get_callback()(None)
    menu.items[1].get_callback()(None)
    on_open.assert_called_once()
    on_quit.assert_called_once()
    assert menu.shown is True
