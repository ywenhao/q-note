"""Exercise real native GTK windows with an isolated SQLite database and X display."""
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
if not os.environ.get("Q_NOTE_SMOKE_SESSION") and shutil.which("dbus-run-session"):
    env = dict(os.environ, Q_NOTE_SMOKE_SESSION="1")
    sys.exit(subprocess.call(["dbus-run-session", "--", sys.executable, __file__], env=env))

display = os.environ.get("Q_NOTE_TEST_DISPLAY", ":98")
server_display = ":" + display.rsplit(":", 1)[1]
env = dict(os.environ, DISPLAY=display, GDK_BACKEND="x11", GSK_RENDERER="cairo", Q_NOTE_TEST_OFFLINE="1", LANG="C.UTF-8")
output = Path(os.environ.get("Q_NOTE_TEST_ARTIFACTS", ROOT / "dist" / "smoke"))
output.mkdir(parents=True, exist_ok=True)
binary = os.environ.get("Q_NOTE_TEST_BINARY", str(ROOT / "target/debug/q-note"))
xdotool = os.environ.get("Q_NOTE_XDOTOOL", "xdotool")
processes = []
logs = (output / "native.log").open("w+")


def run(*args, capture=False):
    result = subprocess.run(list(map(str, args)), env=env, text=True, check=True,
                            stdout=subprocess.PIPE if capture else subprocess.DEVNULL,
                            stderr=subprocess.PIPE, timeout=10)
    return result.stdout.strip() if capture else None


def xdo(*args, capture=False):
    return run(xdotool, *args, capture=capture)


def wait_for(check, timeout=5):
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        result = check()
        if result:
            return result
        time.sleep(0.05)
    raise AssertionError("Timed out waiting for native state")


def windows():
    result = subprocess.run([xdotool, "search", "--onlyvisible", "--name", "Q Note"],
                            env=env, text=True, stdout=subprocess.PIPE)
    return result.stdout.split()


def geometry(window):
    text = xdo("getwindowgeometry", "--shell", window, capture=True)
    return {key: int(value) for key, value in (line.split("=", 1) for line in text.splitlines())}


def tap(window, x, y, button=1):
    xdo("mousemove", "--window", window, x, y)
    time.sleep(0.08)
    xdo("mousedown", button)
    time.sleep(0.06)
    xdo("mouseup", button)
    time.sleep(0.20)


def shot(window, name):
    run("import", "-window", window, "-depth", "8", "-define", "png:color-type=2", output / name)


def pixel(name, x, y):
    return tuple(subprocess.check_output(["convert", str(output / name), "-crop", f"1x1+{x}+{y}", "-depth", "8", "rgb:-"]))


def card_heights(name):
    pixels = subprocess.check_output(["convert", str(output / name), "-crop", "1x450+15+0", "-depth", "8", "rgb:-"])
    heights, start = [], None
    for y in range(len(pixels) // 3):
        inside = all(abs(pixels[y * 3 + i] - c) < 3 for i, c in enumerate((255, 249, 219)))
        if inside and start is None:
            start = y
        elif not inside and start is not None:
            if y - start > 15:
                heights.append(y - start + 4)
            start = None
    return heights


def rows():
    with sqlite3.connect(data_dir / "q-note.db") as db:
        return db.execute("SELECT id,content,pinned,text_height FROM notes ORDER BY pinned DESC,sort_order,updated_at DESC").fetchall()


def new_note(content, media=None):
    previous_ids = {row[0] for row in rows()}
    previous = len(previous_ids)
    tap(main, 154, 54)
    editor = wait_for(lambda: next((w for w in windows() if geometry(w)["WIDTH"] == 520), None))
    xdo("windowactivate", "--sync", editor)
    time.sleep(0.4)
    tap(editor, 130, 140)
    xdo("type", "--clearmodifiers", "--delay", "1", content)
    if media:
        tap(editor, 220, 548)
        xdo("type", "--clearmodifiers", "--delay", "1", media)
        xdo("key", "Return")
        time.sleep(0.3)
    shot(editor, "editor.png")
    xdo("key", "--clearmodifiers", "ctrl+s")
    wait_for(lambda: len(rows()) == previous + 1)
    wait_for(lambda: editor not in windows())
    return next(row for row in rows() if row[0] not in previous_ids)


try:
    server_args = [os.environ.get("Q_NOTE_XVFB", "Xvfb"), server_display, "-screen", "0", "1600x1000x24", "-ac"]
    server_args += ["-listen", "tcp", "-nolisten", "unix"] if display.startswith("localhost:") else ["-nolisten", "tcp"]
    processes.append(subprocess.Popen(server_args, env=env, stdout=logs, stderr=logs))
    wait_for(lambda: subprocess.run([xdotool, "getdisplaygeometry"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0)
    processes.append(subprocess.Popen([os.environ.get("Q_NOTE_WM", "openbox")], env=env, stdout=logs, stderr=logs))
    time.sleep(0.4)
    temp = tempfile.TemporaryDirectory(prefix="q-note-native-smoke-")
    data_dir = Path(temp.name)
    env["Q_NOTE_DATA_DIR"] = str(data_dir)
    processes.append(subprocess.Popen([binary], env=env, stdout=logs, stderr=logs))
    main = wait_for(lambda: next((w for w in windows() if geometry(w)["WIDTH"] == 300), None))
    time.sleep(1)
    shot(main, "empty.png")

    short = new_note("One short line")
    long = new_note("This longer note wraps at the minimum window width and fits one line after widening.")
    assert short[1] == "One short line"
    tap(main, 100, 101)
    clipboard = run(os.environ.get("Q_NOTE_XCLIP", "xclip"), "-selection", "clipboard", "-o", capture=True)
    assert clipboard == long[1], (clipboard, long[1])
    shot(main, "cards.png")
    assert card_heights("cards.png") == [63, 41], card_heights("cards.png")

    tap(main, 120, 101, 3)
    shot(main, "context.png")
    tap(main, 150, 176)
    wait_for(lambda: rows()[0][2] == 1)
    tap(main, 120, 101, 3)
    tap(main, 150, 176)
    wait_for(lambda: rows()[0][2] == 0)
    tap(main, 120, 101, 3)
    xdo("key", "Escape")
    time.sleep(0.2)

    xdo("windowsize", main, 700, 500)
    time.sleep(0.6)
    shot(main, "wide.png")
    assert card_heights("wide.png") == [41, 41], card_heights("wide.png")
    xdo("windowsize", main, 300, 450)
    time.sleep(0.6)

    # The text-height handle retains the drag after the pointer leaves the card.
    xdo("mousemove", "--window", main, 270, 137)
    time.sleep(0.2)
    xdo("mousedown", 1)
    xdo("mousemove", "--window", main, 270, 203)
    time.sleep(0.2)
    xdo("mouseup", 1)
    wait_for(lambda: rows()[0][3] and rows()[0][3] > 44)
    shot(main, "expanded.png")
    expanded_height = 19 + rows()[0][3]
    assert card_heights("expanded.png")[0] == expanded_height

    # Reordering commits the full order and suppresses the card's copy action.
    second_y = 81 + expanded_height + 10 + 20
    before_order = [row[0] for row in rows()]
    xdo("mousemove", "--window", main, 100, second_y)
    xdo("mousedown", 1)
    frame = geometry(main)
    xdo("mousemove", frame["X"] - 20, frame["Y"] + 90)
    time.sleep(0.2)
    xdo("mouseup", 1)
    time.sleep(0.3)
    assert [row[0] for row in rows()] == before_order, "Dropping outside every card must preserve order"
    xdo("mousemove", "--window", main, 100, second_y)
    xdo("mousedown", 1)
    xdo("mousemove", "--window", main, 100, 90)
    time.sleep(0.2)
    xdo("mouseup", 1)
    wait_for(lambda: rows()[0][0] == short[0])

    tap(main, 278, 101)
    shot(main, "actions.png")
    tap(main, 192, 101)
    shot(main, "palette.png")
    tap(main, 236, 136)
    def short_color():
        with sqlite3.connect(data_dir / "q-note.db") as db:
            return db.execute("SELECT color FROM notes WHERE id=?", [short[0]]).fetchone()[0]
    wait_for(lambda: short_color() == "#ffd8a8")
    tap(main, 278, 101)
    shot(main, "actions-again.png")
    tap(main, 128, 101)
    shot(main, "after-pin.png")
    wait_for(lambda: rows()[0][0] == short[0] and rows()[0][2] == 1)
    tap(main, 120, 101, 3)
    tap(main, 150, 176)
    wait_for(lambda: rows()[0][2] == 0)

    mixed = new_note("Image and file reference", str(ROOT / "app-icon.png"))
    with sqlite3.connect(data_dir / "q-note.db") as db:
        attachment = db.execute("SELECT kind,source,value FROM attachments WHERE note_id=?", [mixed[0]]).fetchone()
    assert attachment == ("image", "path", str(ROOT / "app-icon.png"))
    time.sleep(1)
    shot(main, "before-preview.png")
    tap(main, 48, 145)
    time.sleep(0.4)
    shot(main, "preview.png")
    assert max(pixel("preview.png", 5, 200)) < 150, "Preview overlay did not open"
    xdo("key", "plus")
    xdo("click", "4")
    time.sleep(0.3)
    shot(main, "preview-zoom.png")
    assert pixel("preview-zoom.png", 30, 231)[0] > 180, "Preview did not zoom"
    xdo("key", "0")
    xdo("key", "Escape")
    xdo("key", "--clearmodifiers", "ctrl+n")
    time.sleep(0.3)
    assert windows() == [main], "Closing preview must still block the new-note shortcut"

    tap(main, 235, 55)
    shot(main, "settings.png")
    tap(main, 8, 120)

    tap(main, 272, 414)
    dock = wait_for(lambda: next((w for w in windows() if geometry(w)["WIDTH"] == 30), None))
    assert main not in windows()
    time.sleep(0.3)
    assert geometry(dock)["X"] == geometry(main)["X"] + 270
    xdo("mousemove", "--window", dock, 15, 15)
    xdo("mousedown", 1)
    time.sleep(0.15)
    xdo("mousemove", 1585, 500)
    time.sleep(0.2)
    xdo("mouseup", 1)
    xdo("mousemove", 800, 800)
    time.sleep(0.8)
    assert geometry(dock)["X"] == 1585, geometry(dock)
    xdo("mousemove", 1590, 500)
    time.sleep(0.3)
    assert geometry(dock)["X"] == 1570, geometry(dock)
    tap(dock, 15, 15)
    wait_for(lambda: main in windows())
    assert len(rows()) == 3

    # Closing the editor cancels its draft; it does not change the persisted note.
    tap(main, 154, 54)
    editor = wait_for(lambda: next((w for w in windows() if geometry(w)["WIDTH"] == 520), None))
    tap(editor, 120, 140)
    xdo("type", "--clearmodifiers", "discard this draft")
    tap(editor, 497, 20)
    assert len(rows()) == 3
    tap(main, 279, 20)
    assert main not in windows()
    activation = subprocess.Popen([binary], env=env, stdout=logs, stderr=logs)
    activation.wait(timeout=5)
    wait_for(lambda: main in windows())
    shot(main, "main.png")

    tap(main, 120, 101, 3)
    tap(main, 150, 146)
    editor = wait_for(lambda: next((w for w in windows() if geometry(w)["WIDTH"] == 520), None))
    time.sleep(0.4)
    shot(editor, "editor.png")
    tap(main, 270, 55)
    wait_for(lambda: "编辑便签" in xdo("getwindowname", editor, capture=True))
    shot(main, "main-zh.png")
    shot(editor, "editor-zh.png")
    tap(main, 270, 55)
    wait_for(lambda: "Edit note" in xdo("getwindowname", editor, capture=True))
    xdo("windowactivate", "--sync", editor)
    tap(editor, 60, 520)
    shot(editor, "editor-preview.png")
    assert max(pixel("editor-preview.png", 5, 200)) < 150
    xdo("type", "--clearmodifiers", "SHOULD_NOT_LEAK")
    xdo("key", "--clearmodifiers", "ctrl+s")
    time.sleep(0.2)
    assert editor in windows(), "Saving while previewing must not close the editor"
    xdo("key", "Escape")
    time.sleep(0.25)
    xdo("key", "--clearmodifiers", "ctrl+s")
    wait_for(lambda: editor not in windows())
    assert next(row for row in rows() if row[0] == mixed[0])[1] == mixed[1]
    logs.flush()
    assert "ERROR" not in (output / "native.log").read_text(), (output / "native.log").read_text()
    print(json.dumps({"passed": True, "notes": len(rows()), "artifacts": str(output)}, indent=2))
finally:
    for process in reversed(processes):
        process.terminate()
    for process in reversed(processes):
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
    logs.close()
