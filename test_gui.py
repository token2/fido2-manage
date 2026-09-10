"""Test suite for gui.py (fido2-manage GUI wrapper).

tkinter and subprocess are mocked so the tests run headlessly and never touch
real hardware or spawn windows. The goal is full coverage of the module's
logic: the resident-line parser, all command handlers, and main().
"""
import sys
import types
import importlib
from unittest import mock

import pytest


def _install_fake_tk(monkeypatch, terminal=("gnome-terminal", ["--"])):
    tk = types.ModuleType("tkinter")
    for name in ("Tk", "Toplevel", "Label", "Button", "Text", "StringVar", "BooleanVar"):
        setattr(tk, name, mock.MagicMock(name=name))
    tk.END = "end"; tk.BOTH = "both"; tk.TOP = "top"; tk.LEFT = "left"; tk.BOTTOM = "bottom"
    tk.RIGHT = "right"; tk.X = "x"; tk.NORMAL = "normal"
    tk.DISABLED = "disabled"; tk.ACTIVE = "active"

    ttk = types.ModuleType("tkinter.ttk")
    for name in ("Frame", "Combobox", "Treeview", "Scrollbar", "Button",
                 "Label", "Checkbutton", "Style", "Entry", "Notebook"):
        setattr(ttk, name, mock.MagicMock(name=name))
    tk.ttk = ttk

    messagebox = types.ModuleType("tkinter.messagebox")
    for name in ("showerror", "showinfo", "showwarning"):
        setattr(messagebox, name, mock.MagicMock(name=name))
    messagebox.askyesno = mock.MagicMock(return_value=True)
    tk.messagebox = messagebox

    simpledialog = types.ModuleType("tkinter.simpledialog")
    simpledialog.askstring = mock.MagicMock(return_value="x")
    tk.simpledialog = simpledialog

    font = types.ModuleType("tkinter.font")
    _f = mock.MagicMock()
    _f.metrics.return_value = 37
    font.nametofont = mock.MagicMock(return_value=_f)
    tk.font = font

    monkeypatch.setitem(sys.modules, "tkinter", tk)
    monkeypatch.setitem(sys.modules, "tkinter.ttk", ttk)
    monkeypatch.setitem(sys.modules, "tkinter.messagebox", messagebox)
    monkeypatch.setitem(sys.modules, "tkinter.simpledialog", simpledialog)
    monkeypatch.setitem(sys.modules, "tkinter.font", font)

    pexpect = types.ModuleType("pexpect")
    pexpect.spawn = mock.MagicMock(name="spawn")
    pexpect.EOF = "EOF"
    pexpect.TIMEOUT = "TIMEOUT"
    pexpect.exceptions = types.SimpleNamespace(TIMEOUT=type("T", (Exception,), {}))
    monkeypatch.setitem(sys.modules, "pexpect", pexpect)

    monkeypatch.setattr(
        "shutil.which",
        lambda name: name if terminal and name == terminal[0] else None,
    )
    return tk


@pytest.fixture
def gui(monkeypatch):
    _install_fake_tk(monkeypatch)
    sys.modules.pop("gui", None)
    mod = importlib.import_module("gui")
    mod.PIN = None
    return mod


# --- parse_resident_line -------------------------------------------------
def test_parse_full_line(gui):
    line = "[Info] Credential ID: ABC==, User: Jane Doe, Email: jane@example.com, Handle: SGVsbG8="
    assert gui.parse_resident_line(line) == {
        "credential_id": "ABC==", "user": "Jane Doe",
        "email": "jane@example.com", "handle": "SGVsbG8=",
    }


def test_parse_non_credential_line(gui):
    assert gui.parse_resident_line("[Info] Storage stats:") is None


def test_parse_missing_trailing_fields(gui):
    r = gui.parse_resident_line("Credential ID: XYZ, User: Bob")
    assert r["credential_id"] == "XYZ" and r["user"] == "Bob"
    assert r["email"] == "" and r["handle"] == ""


# --- detect_terminal -----------------------------------------------------
def test_detect_terminal_found(gui, monkeypatch):
    monkeypatch.setattr("shutil.which", lambda n: "x" if n == "gnome-terminal" else None)
    assert gui.detect_terminal()[0] == "gnome-terminal"


def test_detect_terminal_none(gui, monkeypatch):
    monkeypatch.setattr("shutil.which", lambda n: None)
    assert gui.detect_terminal() == (None, None)


# --- get_device_list -----------------------------------------------------
def test_get_device_list_ok(gui, monkeypatch):
    monkeypatch.setattr(gui.subprocess, "run",
                        lambda *a, **k: mock.MagicMock(stdout="Device [1] : Key\n"))
    assert gui.get_device_list() == ["Device [1] : Key"]


def test_get_device_list_error(gui, monkeypatch):
    monkeypatch.setattr(gui.subprocess, "run", mock.MagicMock(side_effect=OSError("boom")))
    assert gui.get_device_list() == []


# --- _selected_device_digit / _run_wrapper -------------------------------
def test_selected_device_digit_ok(gui):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [2] : Key"
    assert gui._selected_device_digit() == "2"


def test_selected_device_digit_none(gui):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "nothing"
    assert gui._selected_device_digit() is None


def test_run_wrapper_no_pin(gui, monkeypatch):
    cp = mock.MagicMock(stdout="ok")
    run = mock.MagicMock(return_value=cp)
    monkeypatch.setattr(gui.subprocess, "run", run)
    assert gui._run_wrapper(["-list"]) is cp
    assert run.call_args[0][0] == [gui.FIDO_COMMAND, "-list"]


def test_run_wrapper_with_pin_prompt(gui, monkeypatch):
    cp = mock.MagicMock(stdout="ok")
    monkeypatch.setattr(gui.subprocess, "run", mock.MagicMock(return_value=cp))
    monkeypatch.setattr(gui, "get_pin", lambda: setattr(gui, "PIN", "1234"))
    assert gui._run_wrapper(["-stats"], need_pin=True) is cp


def test_run_wrapper_pin_cancelled(gui, monkeypatch):
    monkeypatch.setattr(gui, "get_pin", lambda: None)
    gui.PIN = None
    assert gui._run_wrapper(["-stats"], need_pin=True) is None


def test_run_wrapper_pin_already_set(gui, monkeypatch):
    cp = mock.MagicMock()
    monkeypatch.setattr(gui.subprocess, "run", mock.MagicMock(return_value=cp))
    gui.PIN = "9999"
    assert gui._run_wrapper(["-stats"], need_pin=True) is cp


# --- get_pin -------------------------------------------------------------
def test_get_pin(gui):
    gui.simpledialog.askstring.return_value = "4321"
    assert gui.get_pin() == "4321"
    assert gui.PIN == "4321"


# --- theme: _gsettings ---------------------------------------------------
def test_gsettings_success(gui, monkeypatch):
    cp = mock.MagicMock(returncode=0, stdout="'prefer-dark'\n")
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: cp)
    assert gui._gsettings("schema", "key") == "prefer-dark"


def test_gsettings_nonzero(gui, monkeypatch):
    cp = mock.MagicMock(returncode=1, stdout="")
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: cp)
    assert gui._gsettings("s", "k") is None


def test_gsettings_empty(gui, monkeypatch):
    cp = mock.MagicMock(returncode=0, stdout="''\n")
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: cp)
    assert gui._gsettings("s", "k") is None


def test_gsettings_exception(gui, monkeypatch):
    monkeypatch.setattr(gui.subprocess, "run", mock.MagicMock(side_effect=OSError))
    assert gui._gsettings("s", "k") is None


# --- theme: detect_color_scheme / detect_accent_color --------------------
def test_detect_color_scheme_dark(gui, monkeypatch):
    monkeypatch.setattr(gui, "_gsettings", lambda *a: "prefer-dark")
    assert gui.detect_color_scheme() == "dark"


def test_detect_color_scheme_light(gui, monkeypatch):
    monkeypatch.setattr(gui, "_gsettings", lambda *a: "default")
    assert gui.detect_color_scheme() == "light"


def test_detect_color_scheme_none(gui, monkeypatch):
    monkeypatch.setattr(gui, "_gsettings", lambda *a: None)
    assert gui.detect_color_scheme() == "light"


def test_detect_accent_known(gui, monkeypatch):
    monkeypatch.setattr(gui, "_gsettings", lambda *a: "purple")
    assert gui.detect_accent_color() == gui._ACCENT_HEX["purple"]


def test_detect_accent_unknown(gui, monkeypatch):
    monkeypatch.setattr(gui, "_gsettings", lambda *a: "chartreuse")
    assert gui.detect_accent_color() == "#3584e4"


def test_detect_accent_none(gui, monkeypatch):
    monkeypatch.setattr(gui, "_gsettings", lambda *a: None)
    assert gui.detect_accent_color() == "#3584e4"


# --- theme: build_palette ------------------------------------------------
def test_build_palette_dark(gui):
    p = gui.build_palette("dark", "#9141ac")
    assert p["bg"] == "#1e1e2e" and p["accent"] == "#9141ac"


def test_build_palette_light(gui):
    p = gui.build_palette("light", "#3584e4")
    assert p["bg"] == "#f6f6fb" and p["accent"] == "#3584e4"


# --- theme: apply_theme --------------------------------------------------
def test_apply_theme(gui):
    style = mock.MagicMock()
    palette = gui.build_palette("dark", "#9141ac")
    assert gui.apply_theme(style, palette) is palette
    style.configure.assert_any_call("TFrame", background="#1e1e2e")


def test_apply_theme_theme_use_fails(gui):
    style = mock.MagicMock()
    style.theme_use.side_effect = Exception("no clam")
    palette = gui.build_palette("light", "#3584e4")
    # Should swallow the theme_use error and still configure styles.
    gui.apply_theme(style, palette)
    assert style.configure.called
