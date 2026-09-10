"""Coverage of gui.py handler functions and main().

Reuses the fake-tkinter installation from test_gui via a local copy of the
fixture so the two files stay independent.
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
    _f = mock.MagicMock(); _f.metrics.return_value = 37
    font.nametofont = mock.MagicMock(return_value=_f)
    tk.font = font

    for n, m in (("tkinter", tk), ("tkinter.ttk", ttk),
                 ("tkinter.messagebox", messagebox),
                 ("tkinter.simpledialog", simpledialog), ("tkinter.font", font)):
        monkeypatch.setitem(sys.modules, n, m)

    pexpect = types.ModuleType("pexpect")
    pexpect.spawn = mock.MagicMock(name="spawn")
    pexpect.EOF = "EOF"; pexpect.TIMEOUT = "TIMEOUT"
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
    mod.sys.platform = "linux"
    # Handlers reference these module globals (normally created in main()).
    mod.root = mock.MagicMock()
    mod.device_var = mock.MagicMock()
    mod.device_combobox = mock.MagicMock()
    mod.tree = mock.MagicMock()
    mod.pin_button = mock.MagicMock()
    mod.passkeys_button = mock.MagicMock()
    return mod


def _cp(rc=0, out="", err=""):
    return mock.MagicMock(returncode=rc, stdout=out, stderr=err)


# --- set_dpi_awareness ---------------------------------------------------
def test_set_dpi_awareness_linux(gui, monkeypatch):
    monkeypatch.setattr(gui.sys, "platform", "linux")
    gui.set_dpi_awareness()  # exercises the non-Windows branch


def test_set_dpi_awareness_exception(gui, monkeypatch):
    # Force the Tk() call in the DPI probe to raise, hitting the fallback.
    monkeypatch.setattr(gui.tk, "Tk", mock.MagicMock(side_effect=RuntimeError))
    gui.set_dpi_awareness()


# --- execute_info_command ------------------------------------------------
def _setup_widgets(gui):
    gui.tree = mock.MagicMock()
    gui.pin_button = mock.MagicMock()
    gui.passkeys_button = mock.MagicMock()
    gui.device_var = mock.MagicMock()


def test_execute_info_command_pin_set(gui, monkeypatch):
    _setup_widgets(gui)
    monkeypatch.setattr(gui.subprocess, "run",
                        lambda *a, **k: _cp(0, "key: value\nnocolon\n"))
    child = mock.MagicMock()
    child.expect.return_value = 0
    child.before = ""
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.execute_info_command("1")
    gui.pin_button.config.assert_called()


def test_execute_info_command_no_pin(gui, monkeypatch):
    _setup_widgets(gui)
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: _cp(0, "k: v\n"))
    child = mock.MagicMock(); child.expect.return_value = 1; child.before = ""
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.execute_info_command("1")
    gui.messagebox.showwarning.assert_called()


def test_execute_info_command_errors(gui, monkeypatch):
    _setup_widgets(gui)
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: _cp(0, "k: v\n"))
    child = mock.MagicMock(); child.expect.return_value = 2
    child.before = "FIDO_ERR_PIN_INVALID FIDO_ERR_PIN_AUTH_BLOCKED FIDO_ERR_INVALID_CBOR FIDO_ERR_PIN_REQUIRED"
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    assert gui.execute_info_command("1") is False


def test_execute_info_command_run_fails(gui, monkeypatch):
    _setup_widgets(gui)
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: _cp(1, "", "err"))
    child = mock.MagicMock(); child.expect.return_value = 0; child.before = ""
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.execute_info_command("1")
    gui.messagebox.showerror.assert_called()


def test_execute_info_command_spawn_raises(gui, monkeypatch):
    _setup_widgets(gui)
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: _cp(0, "k: v\n"))
    monkeypatch.setattr(gui.pexpect, "spawn", mock.MagicMock(side_effect=Exception("x")))
    gui.execute_info_command("1")


# --- on_device_selected --------------------------------------------------
def test_on_device_selected_ok(gui, monkeypatch):
    _setup_widgets(gui)
    gui.device_var.get.return_value = "Device [1] : K"
    monkeypatch.setattr(gui, "execute_info_command", mock.MagicMock())
    gui.on_device_selected(None)
    gui.passkeys_button.config.assert_called()


def test_on_device_selected_no_digit(gui):
    _setup_widgets(gui)
    gui.device_var.get.return_value = "no digit"
    gui.on_device_selected(None)
    gui.messagebox.showinfo.assert_called()


# --- on_passkeys_button_click --------------------------------------------
def test_passkeys_no_digit(gui):
    _setup_widgets(gui)
    gui.device_var.get.return_value = "nope"
    gui.on_passkeys_button_click()
    gui.messagebox.showinfo.assert_called()


def test_passkeys_pin_cancelled(gui, monkeypatch):
    _setup_widgets(gui)
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = None
    monkeypatch.setattr(gui, "get_pin", lambda: None)
    gui.on_passkeys_button_click()


def test_passkeys_success(gui, monkeypatch):
    _setup_widgets(gui)
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "1234"
    calls = [
        _cp(0, "relying party = mercury.com\n"),          # -residentKeys list
        _cp(0, "Credential ID: A, User: U, Email: e@x.y, Handle: H\n"),  # per-domain
    ]
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: calls.pop(0))
    monkeypatch.setattr(gui, "show_output_in_new_window", mock.MagicMock())
    gui.on_passkeys_button_click()
    gui.show_output_in_new_window.assert_called()


def test_passkeys_domain_fails(gui, monkeypatch):
    _setup_widgets(gui)
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "1234"
    calls = [_cp(0, "relying party = mercury.com\n"), _cp(1, "", "boom")]
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: calls.pop(0))
    gui.on_passkeys_button_click()
    gui.messagebox.showerror.assert_called()


def test_passkeys_top_command_fails(gui, monkeypatch):
    _setup_widgets(gui)
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "1234"
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: _cp(1, "", "e"))
    gui.on_passkeys_button_click()
    gui.messagebox.showerror.assert_called()


# --- refresh_combobox ----------------------------------------------------
def test_refresh_combobox(gui, monkeypatch):
    _setup_widgets(gui)
    gui.device_combobox = mock.MagicMock()
    monkeypatch.setattr(gui, "get_device_list", lambda: ["Device [1] : K"])
    gui.refresh_combobox()


def test_refresh_combobox_empty(gui, monkeypatch):
    _setup_widgets(gui)
    gui.device_combobox = mock.MagicMock()
    monkeypatch.setattr(gui, "get_device_list", lambda: [])
    gui.refresh_combobox()


# --- show_output_in_new_window (+ inner buttons) -------------------------
def test_show_output_window(gui, monkeypatch):
    tree = mock.MagicMock()
    tree.selection.return_value = ["item"]
    tree.item.return_value = ("mercury.com", "CRED", "U <e@x.y>", "HANDLE")
    monkeypatch.setattr(gui.ttk, "Treeview", mock.MagicMock(return_value=tree))
    popen = mock.MagicMock()
    monkeypatch.setattr(gui.subprocess, "Popen", popen)
    gui.PIN = "1234"
    out = "Domain: mercury.com\nCredential ID: CRED, User: U, Email: e@x.y, Handle: HANDLE\n"
    gui.show_output_in_new_window(out, "1")
    # Retrieve the inner callbacks passed to the two Buttons and run them.
    btn_calls = [c for c in (gui.tk.Button.call_args_list + gui.ttk.Button.call_args_list)]
    for c in btn_calls:
        cmd = c.kwargs.get("command")
        if cmd:
            cmd()
    popen.assert_called()


def test_show_output_window_edit_no_selection(gui, monkeypatch):
    tree = mock.MagicMock()
    tree.selection.return_value = []
    monkeypatch.setattr(gui.ttk, "Treeview", mock.MagicMock(return_value=tree))
    gui.show_output_in_new_window("Domain: x\n", "1")
    for c in (gui.tk.Button.call_args_list + gui.ttk.Button.call_args_list):
        cmd = c.kwargs.get("command")
        if cmd:
            cmd()


def test_show_output_window_edit_cancel_userid(gui, monkeypatch):
    tree = mock.MagicMock()
    tree.selection.return_value = ["i"]
    tree.item.return_value = ("d", "CRED", "u", "H")
    monkeypatch.setattr(gui.ttk, "Treeview", mock.MagicMock(return_value=tree))
    monkeypatch.setattr(gui.subprocess, "Popen", mock.MagicMock())
    gui.simpledialog.askstring.return_value = None  # user cancels
    gui.show_output_in_new_window("Domain: x\n", "1")
    for c in (gui.tk.Button.call_args_list + gui.ttk.Button.call_args_list):
        cmd = c.kwargs.get("command")
        if cmd:
            cmd()


# --- show_stats ----------------------------------------------------------
def test_show_stats(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "1"
    # _run_async invokes on_done synchronously with a fake result.
    monkeypatch.setattr(gui, "_run_async",
                        lambda args, on_done, need_pin=False: on_done(_cp(0, "stats", "e")))
    gui.show_stats()


def test_show_stats_no_device(gui):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "nope"
    gui.show_stats()


def test_show_stats_cancelled(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    # _run_async returns None (PIN cancelled) and never calls on_done.
    monkeypatch.setattr(gui, "_run_async", lambda *a, **k: None)
    gui.show_stats()


def test_show_stats_font_exception(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    monkeypatch.setattr(gui, "_run_async",
                        lambda args, on_done, need_pin=False: on_done(_cp(0, "s")))
    sys.modules["tkinter.font"].nametofont = mock.MagicMock(side_effect=RuntimeError)
    gui.show_stats()


# --- generate_ssh_key ----------------------------------------------------
def _run_dialog_buttons(gui):
    for c in gui.ttk.Button.call_args_list:
        cmd = c.kwargs.get("command")
        if cmd:
            cmd()


def test_generate_ssh_key(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    popen = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "Popen", popen)
    # Make the vars return concrete values.
    gui.tk.StringVar.return_value.get.return_value = "ed25519-sk"
    gui.tk.BooleanVar.return_value.get.return_value = True
    gui.generate_ssh_key()
    _run_dialog_buttons(gui)
    popen.assert_called()


def test_generate_ssh_key_no_device(gui):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "nope"
    gui.generate_ssh_key()


def test_generate_ssh_key_windows(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    monkeypatch.setattr(gui.sys, "platform", "win32")
    run = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "run", run)
    gui.tk.StringVar.return_value.get.return_value = ""   # no application, empty output
    gui.tk.BooleanVar.return_value.get.return_value = False
    gui.generate_ssh_key()
    _run_dialog_buttons(gui)


# --- manage_large_blob ---------------------------------------------------
def test_manage_large_blob_get_set(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.tk.StringVar.return_value.get.return_value = "mercury.com"
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "ok"))
    monkeypatch.setattr(gui.subprocess, "Popen", mock.MagicMock())
    gui.manage_large_blob()
    _run_dialog_buttons(gui)


def test_manage_large_blob_no_device(gui):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "nope"
    gui.manage_large_blob()


def test_manage_large_blob_missing_fields(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.tk.StringVar.return_value.get.return_value = ""   # empty rp -> _require fails
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "ok"))
    gui.manage_large_blob()
    _run_dialog_buttons(gui)
    gui.messagebox.showerror.assert_called()


def test_manage_large_blob_delete(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.tk.StringVar.return_value.get.return_value = "mercury.com"
    gui.PIN = "1"
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "ok"))
    popen = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "Popen", popen)
    gui.messagebox.askyesno.return_value = True
    gui.manage_large_blob()
    _run_dialog_buttons(gui)


def test_manage_large_blob_delete_declined(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.tk.StringVar.return_value.get.return_value = "mercury.com"
    gui.messagebox.askyesno.return_value = False
    gui.manage_large_blob()
    _run_dialog_buttons(gui)


# --- show_about_message --------------------------------------------------
def test_show_about(gui):
    gui.show_about_message()
    gui.messagebox.showinfo.assert_called()


def test_show_version(gui, monkeypatch):
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "1.15.0"))
    gui.show_version()
    gui.messagebox.showinfo.assert_called()


def test_show_version_cancelled(gui, monkeypatch):
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: None)
    gui.show_version()


# --- set_pin / change_pin ------------------------------------------------
def test_set_pin_success(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.simpledialog.askstring.side_effect = ["1234", "1234"]
    child = mock.MagicMock(); child.before = "ok"
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.set_pin()
    gui.messagebox.showinfo.assert_called()


def test_set_pin_no_digit(gui):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "nope"
    gui.set_pin()


def test_set_pin_cancel_first(gui):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.simpledialog.askstring.side_effect = [None]
    gui.set_pin()


def test_set_pin_cancel_confirm(gui):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.simpledialog.askstring.side_effect = ["1234", None]
    gui.set_pin()


def test_set_pin_mismatch_then_match(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.simpledialog.askstring.side_effect = ["1", "2", "3", "3"]
    child = mock.MagicMock(); child.before = "ok"
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.set_pin()


def test_set_pin_policy_violation(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.simpledialog.askstring.side_effect = ["123456", "123456"]
    child = mock.MagicMock()
    child.before = "FIDO_ERR_PIN_POLICY_VIOLATION minpinlen: 6"
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.set_pin()
    gui.messagebox.showerror.assert_called()


def test_set_pin_generic_error(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.simpledialog.askstring.side_effect = ["1234", "1234"]
    child = mock.MagicMock(); child.before = "some error"
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.set_pin()


def test_set_pin_timeout(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.simpledialog.askstring.side_effect = ["1234", "1234"]
    def _raise(*a, **k):
        raise gui.pexpect.exceptions.TIMEOUT("t")
    monkeypatch.setattr(gui.pexpect, "spawn", _raise)
    gui.set_pin()


def test_set_pin_exception(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.simpledialog.askstring.side_effect = ["1234", "1234"]
    monkeypatch.setattr(gui.pexpect, "spawn", mock.MagicMock(side_effect=ValueError("x")))
    gui.set_pin()


def test_change_pin_success(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "old"
    gui.simpledialog.askstring.side_effect = ["new", "new"]
    child = mock.MagicMock()
    child.expect.side_effect = [3, 0, 0, 1]
    child.before = "ok"
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.change_pin()


def test_change_pin_touch(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "old"
    gui.simpledialog.askstring.side_effect = ["new", "new"]
    child = mock.MagicMock()
    child.expect.side_effect = [0, 0, 0, 1]
    child.before = "ok"
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.change_pin()


def test_change_pin_no_digit(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "nope"
    gui.PIN = "old"
    gui.change_pin()


def test_change_pin_mismatch(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "old"
    gui.simpledialog.askstring.side_effect = ["a", "b", "c", "c"]
    child = mock.MagicMock()
    child.expect.side_effect = [3, 0, 0, 1, 3, 0, 0, 1]
    child.before = "ok"
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.change_pin()


def test_change_pin_prompts_when_pin_none(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = None
    monkeypatch.setattr(gui, "get_pin", lambda: setattr(gui, "PIN", "old"))
    gui.simpledialog.askstring.side_effect = ["new", "new"]
    child = mock.MagicMock(); child.expect.side_effect = [3, 0, 0, 1]; child.before = "ok"
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.change_pin()


def test_change_pin_policy_violation(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "old"
    gui.simpledialog.askstring.side_effect = ["new", "new"]
    child = mock.MagicMock()
    # expect() is called: initial 6-option (3=current PIN), then string expects
    # for "Enter new PIN"/"Enter the same PIN again", then policy expect -> 0.
    child.expect.side_effect = [3, 0, 0, 0]
    child.before = "x"
    info = mock.MagicMock(); info.before = "minpinlen: 6"
    spawns = [child, info]
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: spawns.pop(0))
    gui.change_pin()
    gui.messagebox.showerror.assert_called()


def test_change_pin_timeout(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "old"
    gui.simpledialog.askstring.side_effect = ["new", "new"]
    def _raise(*a, **k):
        raise gui.pexpect.exceptions.TIMEOUT("t")
    monkeypatch.setattr(gui.pexpect, "spawn", _raise)
    gui.change_pin()


def test_change_pin_exception(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "old"
    gui.simpledialog.askstring.side_effect = ["new", "new"]
    monkeypatch.setattr(gui.pexpect, "spawn", mock.MagicMock(side_effect=ValueError))
    gui.change_pin()


# --- main ----------------------------------------------------------------
def test_main_runs(gui, monkeypatch):
    monkeypatch.setattr(gui, "get_device_list", lambda: ["Device [1] : K"])
    monkeypatch.setattr(gui.sys, "argv", ["gui.py"])
    gui.main()


def test_main_no_devices(gui, monkeypatch):
    monkeypatch.setattr(gui, "get_device_list", lambda: [])
    monkeypatch.setattr(gui.sys, "argv", ["gui.py"])
    gui.main()


def test_main_dpi(gui, monkeypatch):
    monkeypatch.setattr(gui, "get_device_list", lambda: ["Device [1] : K"])
    monkeypatch.setattr(gui, "set_dpi_awareness", mock.MagicMock())
    monkeypatch.setattr(gui.sys, "argv", ["gui.py", "-dpi"])
    gui.main()
    gui.set_dpi_awareness.assert_called()


def test_main_no_terminal(gui, monkeypatch):
    monkeypatch.setattr(gui, "TERM", None)
    monkeypatch.setattr(gui.sys, "argv", ["gui.py"])
    with pytest.raises(SystemExit):
        gui.main()


def test_main_font_exception(gui, monkeypatch):
    monkeypatch.setattr(gui, "get_device_list", lambda: ["Device [1] : K"])
    monkeypatch.setattr(gui.sys, "argv", ["gui.py"])
    sys.modules["tkinter.font"].nametofont = mock.MagicMock(side_effect=RuntimeError)
    gui.main()


def test_main_theme_exception(gui, monkeypatch):
    monkeypatch.setattr(gui, "get_device_list", lambda: ["Device [1] : K"])
    monkeypatch.setattr(gui.sys, "argv", ["gui.py"])
    # Force the theme block to raise -> exercises the except: pass fallback.
    monkeypatch.setattr(gui, "build_palette", mock.MagicMock(side_effect=RuntimeError))
    gui.main()


# --- remaining branch coverage ------------------------------------------
def test_parse_part_without_colon(gui):
    # "prefix-no-colon" splits off before "Credential ID:" and has no ": ",
    # exercising the skip/continue branch; the credential is still parsed.
    line = "prefix-no-colon, Credential ID: A, Handle: H"
    r = gui.parse_resident_line(line)
    assert r["credential_id"] == "A"
    assert r["handle"] == "H"


def test_set_dpi_awareness_windows(gui, monkeypatch):
    monkeypatch.setattr(gui.sys, "platform", "win32")
    fake_ctypes = types.SimpleNamespace(
        c_int=lambda: mock.MagicMock(),
        windll=types.SimpleNamespace(
            shcore=types.SimpleNamespace(SetProcessDpiAwareness=lambda *a: None),
            user32=types.SimpleNamespace(GetDC=lambda *a: 0),
            gdi32=types.SimpleNamespace(GetDeviceCaps=lambda *a: 96),
        ),
    )
    monkeypatch.setitem(sys.modules, "ctypes", fake_ctypes)
    gui.ctypes = fake_ctypes
    gui.set_dpi_awareness()


def test_change_pin_policy_no_minpinlen(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "old"
    gui.simpledialog.askstring.side_effect = ["new", "new"]
    child = mock.MagicMock()
    child.expect.side_effect = [3, 0, 0, 0]  # current-PIN path, then policy violation
    child.before = "x"
    info = mock.MagicMock(); info.before = "no length here"
    spawns = [child, info]
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: spawns.pop(0))
    gui.change_pin()
    gui.messagebox.showerror.assert_called()


def test_change_pin_error_output(gui, monkeypatch):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = "Device [1] : K"
    gui.PIN = "old"
    gui.simpledialog.askstring.side_effect = ["new", "new"]
    child = mock.MagicMock()
    child.expect.side_effect = [3, 0, 0, 1, 1]
    child.before = "FIDO_ERR boom"
    monkeypatch.setattr(gui.pexpect, "spawn", lambda *a, **k: child)
    gui.change_pin()


def test_show_output_font_exception(gui, monkeypatch):
    tree = mock.MagicMock()
    monkeypatch.setattr(gui.ttk, "Treeview", mock.MagicMock(return_value=tree))
    monkeypatch.setattr(gui.subprocess, "Popen", mock.MagicMock())
    sys.modules["tkinter.font"].nametofont = mock.MagicMock(side_effect=RuntimeError)
    gui.show_output_in_new_window("Domain: x\n", "1")


def test_show_output_none_parse(gui, monkeypatch):
    tree = mock.MagicMock()
    monkeypatch.setattr(gui.ttk, "Treeview", mock.MagicMock(return_value=tree))
    monkeypatch.setattr(gui.subprocess, "Popen", mock.MagicMock())
    # A line that is neither a Domain nor a credential -> parsed is None -> continue.
    gui.show_output_in_new_window("random noise line\n", "1")


def test_show_output_delete_windows(gui, monkeypatch):
    tree = mock.MagicMock()
    tree.selection.return_value = ["i"]
    tree.item.return_value = ("d", "CRED", "u", "H")
    monkeypatch.setattr(gui.ttk, "Treeview", mock.MagicMock(return_value=tree))
    popen = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "Popen", popen)
    monkeypatch.setattr(gui.sys, "platform", "win32")
    gui.simpledialog.askstring.return_value = None  # edit cancels; delete uses Popen
    gui.show_output_in_new_window("Domain: x\n", "1")
    for c in (gui.tk.Button.call_args_list + gui.ttk.Button.call_args_list):
        cmd = c.kwargs.get("command")
        if cmd:
            cmd()
    popen.assert_called()


def test_edit_metadata_windows(gui, monkeypatch):
    tree = mock.MagicMock()
    tree.selection.return_value = ["i"]
    tree.item.return_value = ("d", "CRED", "u", "H")
    monkeypatch.setattr(gui.ttk, "Treeview", mock.MagicMock(return_value=tree))
    monkeypatch.setattr(gui.subprocess, "Popen", mock.MagicMock())
    run = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "run", run)
    monkeypatch.setattr(gui.sys, "platform", "win32")
    gui.simpledialog.askstring.side_effect = ["USERID", "name", "display"]
    gui.PIN = "1"
    gui.show_output_in_new_window("Domain: x\n", "1")
    # Run only the edit button (second tk.Button) to hit the win32 run() branch.
    for c in (gui.tk.Button.call_args_list + gui.ttk.Button.call_args_list):
        cmd = c.kwargs.get("command")
        if cmd:
            try:
                cmd()
            except StopIteration:
                pass
    run.assert_called()


def test_main_guard_executes(monkeypatch):
    """Run gui.py as __main__ to cover the `if __name__ == '__main__'` line."""
    import runpy
    _install_fake_tk(monkeypatch)
    sys.modules.pop("gui", None)
    monkeypatch.setattr(sys, "argv", ["gui.py"])
    runpy.run_module("gui", run_name="__main__")


# --- _run_async ----------------------------------------------------------
def test_run_async_no_pin(gui, monkeypatch):
    got = {}
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: _cp(0, "ok"))
    # root.after runs the callback immediately.
    gui.root.after = lambda ms, fn: fn()
    # Run the thread target synchronously by replacing Thread.
    class _T:
        def __init__(self, target, daemon=None):
            self.target = target
        def start(self):
            self.target()
    monkeypatch.setattr(gui.threading, "Thread", _T)
    gui._run_async(["-list"], lambda r: got.setdefault("r", r))
    assert got["r"].stdout == "ok"


def test_run_async_pin_prompt(gui, monkeypatch):
    monkeypatch.setattr(gui.subprocess, "run", lambda *a, **k: _cp(0, "ok"))
    monkeypatch.setattr(gui, "get_pin", lambda: setattr(gui, "PIN", "1"))
    gui.root.after = lambda ms, fn: fn()
    class _T:
        def __init__(self, target, daemon=None):
            self.target = target
        def start(self):
            self.target()
    monkeypatch.setattr(gui.threading, "Thread", _T)
    got = {}
    gui._run_async(["-stats"], lambda r: got.setdefault("r", r), need_pin=True)
    assert got["r"].stdout == "ok"


def test_run_async_pin_cancelled(gui, monkeypatch):
    monkeypatch.setattr(gui, "get_pin", lambda: None)
    gui.PIN = None
    assert gui._run_async(["-stats"], lambda r: None, need_pin=True) is None


# --- Track 1: bio + min-pin + blob key -----------------------------------
def _dev(gui, sel="Device [1] : K"):
    gui.device_var = mock.MagicMock()
    gui.device_var.get.return_value = sel


def test_show_bio_list(gui, monkeypatch):
    _dev(gui)
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "bio"))
    gui.show_bio_list()
    gui.messagebox.showinfo.assert_called()


def test_show_bio_list_no_device(gui):
    _dev(gui, "nope")
    gui.show_bio_list()


def test_show_bio_list_cancelled(gui, monkeypatch):
    _dev(gui)
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: None)
    gui.show_bio_list()


def test_bio_delete(gui, monkeypatch):
    _dev(gui)
    gui.PIN = "1"
    gui.simpledialog.askstring.return_value = "TID"
    popen = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "Popen", popen)
    gui.bio_delete()
    popen.assert_called()


def test_bio_delete_no_device(gui):
    _dev(gui, "nope")
    gui.bio_delete()


def test_bio_delete_cancel(gui):
    _dev(gui)
    gui.simpledialog.askstring.return_value = None
    gui.bio_delete()


def test_bio_delete_windows(gui, monkeypatch):
    _dev(gui)
    monkeypatch.setattr(gui.sys, "platform", "win32")
    gui.simpledialog.askstring.return_value = "TID"
    run = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "run", run)
    gui.bio_delete()
    run.assert_called()


def test_bio_rename(gui, monkeypatch):
    _dev(gui)
    gui.simpledialog.askstring.side_effect = ["TID", "Left index"]
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "ok"))
    gui.bio_rename()
    gui.messagebox.showinfo.assert_called()


def test_bio_rename_no_device(gui):
    _dev(gui, "nope")
    gui.bio_rename()


def test_bio_rename_cancel_id(gui):
    _dev(gui)
    gui.simpledialog.askstring.return_value = None
    gui.bio_rename()


def test_bio_rename_cancel_name(gui):
    _dev(gui)
    gui.simpledialog.askstring.side_effect = ["TID", None]
    gui.bio_rename()


def test_bio_rename_run_cancelled(gui, monkeypatch):
    _dev(gui)
    gui.simpledialog.askstring.side_effect = ["TID", "N"]
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: None)
    gui.bio_rename()


def test_set_pin_min_rps(gui, monkeypatch):
    _dev(gui)
    gui.simpledialog.askstring.return_value = "a.com,b.com"
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "ok"))
    gui.set_pin_min_rps()
    gui.messagebox.showinfo.assert_called()


def test_set_pin_min_rps_no_device(gui):
    _dev(gui, "nope")
    gui.set_pin_min_rps()


def test_set_pin_min_rps_cancel(gui):
    _dev(gui)
    gui.simpledialog.askstring.return_value = None
    gui.set_pin_min_rps()


def test_set_pin_min_rps_cancelled_run(gui, monkeypatch):
    _dev(gui)
    gui.simpledialog.askstring.return_value = "a.com"
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: None)
    gui.set_pin_min_rps()


def test_gen_blob_key(gui, monkeypatch):
    gui.simpledialog.askstring.return_value = "/tmp/k"
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "ok"))
    gui.gen_blob_key()
    gui.messagebox.showinfo.assert_called()


def test_gen_blob_key_cancel(gui):
    gui.simpledialog.askstring.return_value = None
    gui.gen_blob_key()


def test_gen_blob_key_none_result(gui, monkeypatch):
    gui.simpledialog.askstring.return_value = "/tmp/k"
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: None)
    gui.gen_blob_key()


# --- Track 2: SSH lifecycle ----------------------------------------------
def test_show_ssh_list(gui, monkeypatch):
    _dev(gui)
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "ssh"))
    gui.show_ssh_list()
    gui.messagebox.showinfo.assert_called()


def test_show_ssh_list_no_device(gui):
    _dev(gui, "nope")
    gui.show_ssh_list()


def test_show_ssh_list_cancelled(gui, monkeypatch):
    _dev(gui)
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: None)
    gui.show_ssh_list()


def test_ssh_download(gui, monkeypatch):
    _dev(gui)
    gui.simpledialog.askstring.return_value = "/tmp/ssh"
    popen = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "Popen", popen)
    gui.ssh_download()
    popen.assert_called()


def test_ssh_download_no_device(gui):
    _dev(gui, "nope")
    gui.ssh_download()


def test_ssh_download_cancel(gui):
    _dev(gui)
    gui.simpledialog.askstring.return_value = None
    gui.ssh_download()


def test_ssh_download_windows(gui, monkeypatch):
    _dev(gui)
    monkeypatch.setattr(gui.sys, "platform", "win32")
    gui.simpledialog.askstring.return_value = "/tmp/ssh"
    run = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "run", run)
    gui.ssh_download()
    run.assert_called()


# --- ssh_upload / ssh_add_key --------------------------------------------
def test_ssh_upload_with_port(gui, monkeypatch):
    gui.simpledialog.askstring.side_effect = ["/k.pub", "user@host", "2222"]
    popen = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "Popen", popen)
    gui.ssh_upload()
    popen.assert_called_once()
    argv = popen.call_args[0][0]
    assert "-sshPort" in argv and "2222" in argv


def test_ssh_upload_no_port(gui, monkeypatch):
    gui.simpledialog.askstring.side_effect = ["/k.pub", "user@host", ""]
    popen = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "Popen", popen)
    gui.ssh_upload()
    argv = popen.call_args[0][0]
    assert "-sshPort" not in argv


def test_ssh_upload_cancel_key(gui):
    gui.simpledialog.askstring.return_value = None
    gui.ssh_upload()


def test_ssh_upload_cancel_host(gui):
    gui.simpledialog.askstring.side_effect = ["/k.pub", None]
    gui.ssh_upload()


def test_ssh_upload_windows(gui, monkeypatch):
    monkeypatch.setattr(gui.sys, "platform", "win32")
    gui.simpledialog.askstring.side_effect = ["/k.pub", "user@host", ""]
    run = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "run", run)
    gui.ssh_upload()
    run.assert_called()


def test_ssh_add_key(gui, monkeypatch):
    gui.simpledialog.askstring.return_value = "/k"
    popen = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "Popen", popen)
    gui.ssh_add_key()
    popen.assert_called_once()


def test_ssh_add_key_cancel(gui):
    gui.simpledialog.askstring.return_value = None
    gui.ssh_add_key()


def test_ssh_add_key_windows(gui, monkeypatch):
    monkeypatch.setattr(gui.sys, "platform", "win32")
    gui.simpledialog.askstring.return_value = "/k"
    run = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "run", run)
    gui.ssh_add_key()
    run.assert_called()


# --- Track 3: audit ------------------------------------------------------
def test_export_audit(gui, monkeypatch):
    _dev(gui)
    gui.simpledialog.askstring.side_effect = ["json", "/tmp/a.json"]
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "ok"))
    gui.export_audit()
    gui.messagebox.showinfo.assert_called()


def test_export_audit_no_device(gui):
    _dev(gui, "nope")
    gui.export_audit()


def test_export_audit_bad_format(gui):
    _dev(gui)
    gui.simpledialog.askstring.side_effect = ["xml"]
    gui.export_audit()
    gui.messagebox.showerror.assert_called()


def test_export_audit_cancel_output(gui):
    _dev(gui)
    gui.simpledialog.askstring.side_effect = ["json", None]
    gui.export_audit()


def test_export_audit_cancelled_run(gui, monkeypatch):
    _dev(gui)
    gui.simpledialog.askstring.side_effect = ["csv", "/tmp/a.csv"]
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: None)
    gui.export_audit()


# --- Track 4: age + luks -------------------------------------------------
def test_age_setup(gui, monkeypatch):
    _dev(gui)
    gui.simpledialog.askstring.return_value = "/tmp/age.txt"
    popen = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "Popen", popen)
    gui.age_setup()
    popen.assert_called()


def test_age_setup_no_device(gui):
    _dev(gui, "nope")
    gui.age_setup()


def test_age_setup_cancel(gui):
    _dev(gui)
    gui.simpledialog.askstring.return_value = None
    gui.age_setup()


def test_age_setup_windows(gui, monkeypatch):
    _dev(gui)
    monkeypatch.setattr(gui.sys, "platform", "win32")
    gui.simpledialog.askstring.return_value = "/tmp/age.txt"
    run = mock.MagicMock(); monkeypatch.setattr(gui.subprocess, "run", run)
    gui.age_setup()
    run.assert_called()


def test_luks_enroll(gui, monkeypatch):
    _dev(gui)
    gui.simpledialog.askstring.return_value = "/dev/sda2"
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: _cp(0, "sudo systemd-cryptenroll ..."))
    gui.luks_enroll()
    gui.messagebox.showwarning.assert_called()


def test_luks_enroll_no_device(gui):
    _dev(gui, "nope")
    gui.luks_enroll()


def test_luks_enroll_cancel(gui):
    _dev(gui)
    gui.simpledialog.askstring.return_value = None
    gui.luks_enroll()


def test_luks_enroll_cancelled_run(gui, monkeypatch):
    _dev(gui)
    gui.simpledialog.askstring.return_value = "/dev/sda2"
    monkeypatch.setattr(gui, "_run_wrapper", lambda *a, **k: None)
    gui.luks_enroll()


# --- minimize-to-tray helpers --------------------------------------------
def test_hide_to_tray(gui):
    gui.root = mock.MagicMock()
    gui.hide_to_tray()
    gui.root.withdraw.assert_called_once()


def test_hide_to_tray_no_root(gui):
    gui.root = None
    gui.hide_to_tray()  # no error


def test_show_window(gui):
    gui.root = mock.MagicMock()
    gui.show_window()
    gui.root.deiconify.assert_called_once()
    gui.root.lift.assert_called_once()


def test_show_window_no_root(gui):
    gui.root = None
    gui.show_window()


def test_quit_app(gui):
    gui.root = mock.MagicMock()
    gui.quit_app()
    gui.root.destroy.assert_called_once()


def test_quit_app_no_root(gui):
    gui.root = None
    gui.quit_app()


def test_start_tray_unavailable(gui, monkeypatch):
    # Force the gi import inside start_tray to fail -> returns None.
    import builtins
    real_import = builtins.__import__

    def fake_import(name, *a, **k):
        if name == "gi":
            raise ImportError("no gi")
        return real_import(name, *a, **k)

    monkeypatch.setattr(builtins, "__import__", fake_import)
    assert gui.start_tray() is None


def test_start_tray_success(gui, monkeypatch):
    gui.root = mock.MagicMock()
    created = []
    _gi = types.ModuleType("gi")
    _gi.require_version = mock.MagicMock()
    repo = types.ModuleType("gi.repository")

    class _Item:
        def __init__(self, label=None):
            self.label = label
            self._cb = None
            created.append(self)
        def connect(self, sig, cb):
            self._cb = cb

    class _Menu:
        def __init__(self):
            self.items = []
        def append(self, i):
            self.items.append(i)
        def show_all(self):
            pass

    repo.Gtk = types.SimpleNamespace(Menu=_Menu, MenuItem=_Item, main=mock.MagicMock())
    _ind = mock.MagicMock()
    repo.AppIndicator3 = types.SimpleNamespace(
        Indicator=types.SimpleNamespace(new=mock.MagicMock(return_value=_ind)),
        IndicatorCategory=types.SimpleNamespace(APPLICATION_STATUS=1),
        IndicatorStatus=types.SimpleNamespace(ACTIVE=1),
    )
    _gi.repository = repo
    monkeypatch.setitem(sys.modules, "gi", _gi)
    monkeypatch.setitem(sys.modules, "gi.repository", repo)

    ind = gui.start_tray()
    assert ind is _ind
    # Invoke the Open and Quit menu callbacks -> they schedule via root.after.
    for it in created:
        it._cb(None)
    assert gui.root.after.call_count == 2

