"""Manual GTK integration check; no Bluetooth access. Produces offline previews."""
import sys
import tempfile
from pathlib import Path
from earfun.ui import App, Gtk, GLib
from earfun.settings import Settings

app = App(smoke=True)
if '--compact' in sys.argv:
    app.initial_size = (1060, 760)
app.smoke = False  # Keep an isolated application, drive checks below.
temporary = tempfile.TemporaryDirectory()
app.settings = Settings(Path(temporary.name)/'settings.json')
output = Path('artifacts/ui-preview-compact' if '--compact' in sys.argv else 'artifacts/ui-preview')
output.mkdir(parents=True, exist_ok=True)
failures = []
paintable = None


def check(condition, text):
    print(('PASS ' if condition else 'FAIL ') + text, flush=True)
    if not condition:
        failures.append(text)


def start():
    global paintable
    paintable = Gtk.WidgetPaintable.new(app.window)
    check(len(app.pages) == 6, 'All six Windows pages exist')
    check(len(app.batteries) == 3, 'Persistent battery summary')
    check(len(app.switch_bindings) == 9, 'Switches replace boolean drop-downs')
    check(len(app.noise_buttons) == 3, 'Three listening mode tiles')
    check(not app.read_button.get_sensitive(), 'Device controls disabled offline')
    # Local EQ actions must never submit a BLE operation.
    original_submit = app.backend.submit
    submissions = []
    def reject(coro):
        submissions.append(coro)
        coro.close()
        raise AssertionError('Unexpected device write during local editing')
    app.backend.submit = reject
    check(len(app.preset_options) == 33, 'All 32 APK presets and local draft are listed')
    keys = [row[0] for row in app.preset_options]
    app.preset_picker.set_selected(keys.index('earfun:24'))
    check(app.eq_values() == [4, 9, 0, 0, 0, 0, 0, 0, 0, 0], 'APK Bass Boost 3 loads exactly, locally')
    app.set_eq([1]*10)
    check(app.preset_options[app.preset_picker.get_selected()][0] == 'draft', 'Edited curve cannot be mislabeled Flat')
    app.set_eq([0]*10)
    check(app.preset_options[app.preset_picker.get_selected()][0] == 'earfun:0', 'Reset synchronizes Default selection')
    app.preset_name.set_text('Riêng')
    app.save_preset()
    app.preset_name.set_text('Đổi tên')
    app.rename_preset()
    check('Riêng' not in app.settings.data['presets'] and 'Đổi tên' in app.settings.data['presets'], 'Rename keeps the local preset data')
    original_confirm = app.confirm
    app.confirm = lambda text, callback: callback()
    app.delete_preset()
    app.confirm = original_confirm
    check(not app.settings.data['presets'], 'Delete removes only personal presets')
    app.preset_picker.set_selected(0)
    app.set_eq([0]*10)
    check(not submissions, 'Preset and reset never send BLE')
    # State synchronization must never turn into a write, and selecting a value
    # remains a local draft until the explicit Windows-style Apply button.
    app.connected = True
    app.state = {'version': 'TW600-UI-fixture', 'sub': 'DEFAULT', 'noise': 'normal',
                 'game': True, 'ear': True, 'dual': False, 'ldac': False,
                 'aptx': True, 'adaptive': True, 'lossless': True,
                 'dis_l': False, 'dis_r': False, 'le_audio': False}
    app.render_state(force=True)
    check(not submissions, 'Reading device state never writes switches')
    chooser = app.bindings[0][1]
    chooser.set_selected(2)  # BALANCE
    check(not submissions, 'Choosing ANC submode waits for Apply')
    app.backend.submit = original_submit
    calls = []
    async def fake_set(*args):
        calls.append(args)
    app.backend.set_value = fake_set
    app.choice_groups[0][1].get_last_child().emit('clicked')
    GLib.timeout_add(200, check_apply, calls)
    return False


def check_apply(calls):
    check(calls == [(58, 59, 3, 'sub', 'BALANCE')], 'Apply sends exactly the chosen ANC submode')
    app.connected = False
    app.state = {}
    app.render_state(force=True)
    app.navigate('Tổng quan')
    GLib.timeout_add(500, capture, 0)
    return False


def capture(index):
    try:
        snapshot = Gtk.Snapshot.new()
        child = app.window
        paintable.snapshot(snapshot, child.get_width(), child.get_height())
        node = snapshot.to_node()
        check(node is not None, f'Snapshot {index+1}')
        if node:
            texture = app.window.get_renderer().render_texture(node, None)
            texture.save_to_png(str(output/f'page-{index+1}.png'))
        name = list(app.pages)[index]
        check(app.heading.get_text() == name, f'Title and navigation: {name}')
        check(app.page_subtitle.get_text() != '', f'Windows subtitle: {name}')
        if index < 5:
            app.navigate(list(app.pages)[index+1])
            app.window.queue_draw()
            GLib.timeout_add(500, capture, index+1)
        else:
            app.navigate('Tổng quan')
            GLib.timeout_add(800, minimum_size)
    except BaseException as error:
        failures.append(str(error))
        print('ERROR', repr(error), flush=True)
        app.close()
    return False


def minimum_size():
    check(app.window.measure(Gtk.Orientation.HORIZONTAL, -1)[0] <= 1060, 'Minimum window width stays compact')
    check(app.stack.get_width() >= 600, 'Workspace remains usable at minimum width')
    print(f'RESULT: {len(failures)} failures', flush=True)
    app.close()
    return False

GLib.timeout_add(700, start)
GLib.timeout_add_seconds(20, lambda: (failures.append('UI check timeout'), app.close(), False)[-1])
app.run([])
temporary.cleanup()
sys.exit(bool(failures))
