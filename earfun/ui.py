import concurrent.futures
import json
import subprocess
from pathlib import Path
import gi
gi.require_version('Gtk', '4.0')
from gi.repository import Gtk, Gio, GLib, Gdk
from .backend import Backend
from .settings import Settings
from .widgets import ASSETS, Columns, EqCurve, icon, load_fonts
from . import protocol as p, audio, presets, workflows

PAGE_DESCRIPTIONS = {
    'Tổng quan': 'Âm thanh theo cách của bạn', 'Equalizer': 'Tinh chỉnh từng dải âm',
    'Cảm ứng': 'Thao tác quen thuộc, theo cách của bạn', 'Hai thiết bị': 'Quản lý các thiết bị đang kết nối',
    'Tai nghe': 'Cá nhân hóa tai nghe của bạn', 'Ứng dụng': 'Tùy chọn và thông tin ứng dụng',
}


def box(vertical=True, spacing=12):
    return Gtk.Box(orientation=Gtk.Orientation.VERTICAL if vertical else Gtk.Orientation.HORIZONTAL, spacing=spacing)


def label(text='', css=None):
    widget = Gtk.Label(label=text, xalign=0, wrap=True)
    if css:
        for name in css.split():
            widget.add_css_class(name)
    return widget


def button(text, callback, primary=False, css=None):
    widget = Gtk.Button(label=text)
    if primary:
        widget.add_css_class('primary')
    if css:
        for name in css.split():
            widget.add_css_class(name)
    widget.connect('clicked', lambda _: callback())
    return widget


def spacer():
    return Gtk.Box(hexpand=True)


def accessible(widget, name):
    widget.update_property([Gtk.AccessibleProperty.LABEL], [name])
    return widget


class App(Gtk.Application):
    def __init__(self, smoke=False, connect_last=False):
        super().__init__(application_id='vn.earfun.Studio', flags=Gio.ApplicationFlags.NON_UNIQUE if smoke else Gio.ApplicationFlags.DEFAULT_FLAGS)
        self.connect_last = connect_last
        self.smoke = smoke
        self.testing = smoke
        self.settings = Settings()
        self.backend = Backend(lambda event, value: GLib.idle_add(self.handle, event, value))
        self.executor = concurrent.futures.ThreadPoolExecutor(max_workers=1)
        self.connected = self.connecting = self.user_disconnected = self.updating = False
        self.busy = 0
        self.state = {}
        self.rendered_state = {}
        self.dirty_choices = set()
        self.dirty_keys = set()
        self.gesture_rows = {}
        self.balance_dirty = False
        self.eq_sent_levels = None
        self.eq_sending_levels = None
        self.connection_generation = 0
        self.controls, self.phone_buttons, self.bindings = [], [], []
        self.switch_bindings, self.choice_groups = {}, []
        self.address = self.settings.data['last_mac']
        self.devices, self.pages, self.navs = [], {}, {}
        self.connect('activate', self.activate)

    def activate(self, _):
        if hasattr(self, 'window'):
            self.window.present()
            return
        load_fonts()
        theme = Gtk.Settings.get_default()
        theme.set_property('gtk-application-prefer-dark-theme', False)
        theme.set_property('gtk-theme-name', 'Adwaita')
        provider = Gtk.CssProvider()
        provider.load_from_path(str(ASSETS/'theme.css'))
        Gtk.StyleContext.add_provider_for_display(Gdk.Display.get_default(), provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
        self.window = Gtk.ApplicationWindow(application=self, title='EarFun Studio · Ubuntu')
        self.window.set_default_size(*getattr(self, 'initial_size', (1440, 900)))
        self.window.connect('close-request', self.close)
        outer = box(False, 22)
        outer.set_margin_start(20)
        outer.set_margin_end(20)
        outer.set_margin_top(20)
        outer.set_margin_bottom(16)
        self.window.set_child(outer)
        side = box(True, 6)
        side.add_css_class('sidebar')
        side.set_size_request(188, -1)  # 218 px including padding and border
        brand = box(False, 11)
        brand.add_css_class('brand-block')
        brand_icon = box()
        brand_icon.add_css_class('brand-icon')
        brand_icon.append(icon('eq', 26))
        brand.append(brand_icon)
        copy = box(True, 3)
        copy.set_valign(Gtk.Align.CENTER)
        copy.append(label('EarFun Studio', 'brand'))
        copy.append(label('Air Pro 4', 'caption'))
        brand.append(copy)
        side.append(brand)
        outer.append(side)
        main = box(True, 18)
        main.set_hexpand(True)
        outer.append(main)
        self.header = box(False, 9)
        title_block = box(True, 3)
        title_block.set_hexpand(True)
        self.heading = label('Tổng quan', 'title')
        self.page_subtitle = label(PAGE_DESCRIPTIONS['Tổng quan'], 'subtitle')
        title_block.append(self.heading)
        title_block.append(self.page_subtitle)
        self.header.append(title_block)
        toolbar = box(False, 9)
        toolbar.set_valign(Gtk.Align.CENTER)
        toolbar.set_halign(Gtk.Align.END)
        self.device_picker = accessible(Gtk.DropDown.new_from_strings(['Chọn tai nghe']), 'Chọn tai nghe')
        self.device_picker.set_size_request(222, -1)
        toolbar.append(self.device_picker)
        self.scan_button = button('Tìm thiết bị', self.scan)
        toolbar.append(self.scan_button)
        self.header.append(toolbar)
        main.append(self.header)
        main.append(self.device_summary())
        self.stack = Gtk.Stack(transition_type=Gtk.StackTransitionType.NONE, vexpand=True)
        main.append(self.stack)
        builders = [('Tổng quan', self.overview), ('Equalizer', self.equalizer), ('Cảm ứng', self.gestures),
                    ('Hai thiết bị', self.multipoint), ('Tai nghe', self.earbuds), ('Ứng dụng', self.application)]
        for (name, build), glyph in zip(builders, ('overview', 'eq', 'touch', 'devices', 'settings', 'app')):
            nav = button('', lambda n=name: self.navigate(n), css='nav')
            content = box(False, 12)
            content.append(icon(glyph))
            content.append(label(name))
            nav.set_child(content)
            nav.set_tooltip_text(PAGE_DESCRIPTIONS[name])
            accessible(nav, name)
            side.append(nav)
            self.navs[name] = nav
            page = box(True, 14)
            page.set_margin_start(2)
            page.set_margin_end(8)
            page.set_margin_top(2)
            page.set_margin_bottom(12)
            scroll = Gtk.ScrolledWindow(hscrollbar_policy=Gtk.PolicyType.NEVER)
            scroll.set_child(page)
            self.stack.add_named(scroll, name)
            self.pages[name] = page
            build(page)
        stretch = Gtk.Box(vexpand=True)
        side.append(stretch)
        side.append(button('Bluetooth Ubuntu', self.open_bluetooth, css='ghost'))
        self.sidebar_status = label('Bluetooth LE • Chưa kết nối', 'caption')
        self.sidebar_status.set_margin_start(12)
        self.sidebar_status.set_margin_top(10)
        self.sidebar_status.set_margin_bottom(8)
        side.append(self.sidebar_status)
        side.append(label('EarFun Studio 1.2', 'tiny'))
        footer = box(False, 8)
        footer.set_size_request(-1, 28)
        footer.append(icon('check', 16))
        self.notice = label('Lấy tai nghe ra khỏi hộp. Bấm Tìm thiết bị để bắt đầu.', 'notice')
        self.notice.set_selectable(True)
        self.notice.set_hexpand(True)
        footer.append(self.notice)
        main.append(footer)
        self.navigate('Tổng quan')
        self.render_state(force=True)
        self.window.present()
        if self.smoke:
            GLib.timeout_add(500, self.smoke_pages, 0)
        else:
            GLib.timeout_add_seconds(20, self.reconnect)
            if not self.testing:
                loaded = self.task(self.backend.load_devices())
                if self.connect_last:
                    loaded.add_done_callback(lambda f: GLib.idle_add(self.connect_loaded) if not f.exception() else None)

    def device_summary(self):
        summary = Gtk.Grid(column_homogeneous=True, column_spacing=12)
        device = self.card(None, padding='summary-card')
        heading = box(False, 10)
        image = box()
        image.add_css_class('device-icon')
        image.append(icon('earbud', 28))
        heading.append(image)
        copy = box(True, 4)
        copy.set_valign(Gtk.Align.CENTER)
        copy.append(label('EarFun Air Pro 4', 'device-name'))
        badges = box(False, 7)
        self.badge = label('Chưa kết nối', 'caption')
        self.codec_badge = label('—', 'eq-gain')
        badges.append(self.badge)
        badges.append(label('•', 'caption'))
        badges.append(self.codec_badge)
        copy.append(badges)
        heading.append(copy)
        device.append(heading)
        self.connect_button = button('Kết nối', self.toggle_connect, True, css='small')
        self.connect_button.set_halign(Gtk.Align.START)
        device.append(self.connect_button)
        summary.attach(device, 0, 0, 2, 1)
        self.batteries, self.battery_bars = {}, {}
        for i, (key, title) in enumerate((('bat_l', 'Tai trái'), ('bat_r', 'Tai phải'), ('bat_box', 'Hộp sạc'))):
            card = self.card(None, padding='summary-card')
            heading = box(False, 7)
            heading.append(icon('case' if i == 2 else 'earbud', 16))
            heading.append(label(title, 'subtitle'))
            card.append(heading)
            value = label('—', 'battery')
            bar = Gtk.ProgressBar()
            card.append(value)
            card.append(bar)
            summary.attach(card, i+2, 0, 1, 1)
            self.batteries[key], self.battery_bars[key] = value, bar
        return summary

    def navigate(self, name):
        self.stack.set_visible_child_name(name)
        self.heading.set_text(name)
        self.page_subtitle.set_text(PAGE_DESCRIPTIONS[name])
        for n, widget in self.navs.items():
            if n == name:
                widget.add_css_class('active')
            else:
                widget.remove_css_class('active')

    def card(self, parent=None, title=None, subtitle=None, padding=None):
        card = box(True, 12)
        card.add_css_class('card')
        if padding:
            card.add_css_class(padding)
        if title:
            copy = box(True, 4)
            copy.append(label(title, 'section'))
            if subtitle:
                copy.append(label(subtitle, 'subtitle'))
            card.append(copy)
        if parent is not None:
            parent.append(card)
        return card

    def gated(self, widget):
        self.controls.append(widget)
        return widget

    def row(self, parent, title, widget, description=None):
        row = box(False, 20)
        row.set_margin_top(5)
        row.set_margin_bottom(5)
        copy = box(True, 3)
        copy.set_hexpand(True)
        copy.set_valign(Gtk.Align.CENTER)
        copy.append(label(title, 'medium'))
        if description:
            copy.append(label(description, 'caption'))
        row.append(copy)
        widget.set_valign(Gtk.Align.CENTER)
        row.append(widget)
        parent.append(row)
        return row

    def choice(self, parent, title, key, mapping, setter, query, transform=None, warning=None, vertical=False):
        values = list(mapping)
        widget = accessible(Gtk.DropDown.new_from_strings(['—'] + [mapping[v] for v in values]), title)
        widget.set_hexpand(True)
        widget.connect('notify::selected', lambda *_: self.mark_choice_dirty(key, widget) if not self.updating else None)
        def apply():
            idx = widget.get_selected()
            if idx < 1 or idx > len(values):
                return
            value = values[idx-1]
            if key == 'le_audio':
                self.request_plan(workflows.restart_setting, key, value)
                return
            def send():
                self.task(self.backend.set_value(setter, query, transform(value) if transform else value, key, value))
            self.confirm(warning, send) if warning else send()
        action = button('Áp dụng', apply, css='small')
        controls = box(False, 8)
        controls.append(widget)
        controls.append(action)
        if vertical:
            group = box(True, 4)
            group.append(label(title, 'medium'))
            group.append(controls)
            parent.append(group)
        else:
            controls.set_size_request(320, -1)
            self.row(parent, title, controls)
        self.bindings.append((key, widget, values))
        self.choice_groups.append((key, controls))
        return widget

    def switch_row(self, parent, title, callback, description=None, confirmed=False):
        widget = accessible(Gtk.Switch(), title)
        def changed(w, active):
            if self.updating:
                return False
            callback(bool(active))
            return not confirmed
        widget.connect('state-set', changed)
        self.row(parent, title, widget, description)
        return widget

    def toggle(self, parent, title, key, setter, query, inverse=False, direct=False, warning=None, description=None):
        def changed(on):
            if key == 'dual':
                self.request_plan(workflows.dual, on)
                return
            if key == 'ear':
                self.request_plan(workflows.restart_setting, key, on)
                return
            def apply():
                value = int(not on) if inverse else int(on)
                self.task(self.backend.direct(setter, value) if direct else self.backend.set_value(setter, query, value, key, on))
            self.confirm(warning, apply) if warning else apply()
        widget = self.switch_row(parent, title, changed, description)
        self.switch_bindings[key] = widget
        self.gated(widget)
        return widget

    def mark_key_dirty(self, field, widget):
        self.dirty_keys.add(field)
        widget.add_css_class('draft-field')
        self.render_state()

    def mark_balance_dirty(self):
        self.balance_dirty = True
        self.balance.add_css_class('draft-field')
        self.render_state()

    def request_plan(self, builder, *args):
        try:
            plan = builder(self.state, *args)
        except ValueError as error:
            self.show_notice(str(error), True)
            self.render_state()
            return
        if not plan.steps:
            self.show_notice('Tai nghe đã ở thiết lập này.')
            self.render_state()
            return
        self.confirm(plan.confirmation, lambda: self.task(self.backend.run_plan(plan)))

    def mark_choice_dirty(self, key, widget):
        self.dirty_choices.add(key)
        widget.add_css_class('draft-field')
        self.render_state()

    def overview(self, page):
        card = self.card(page, 'Kiểm soát tiếng ồn')
        modes = Columns(breakpoint=640, gap=10, columns=3)
        self.noise_buttons = {}
        for key, text, description, glyph in [('normal', 'Bình thường', 'Âm thanh nguyên bản', 'normal'),
                ('anc', 'Chống ồn', 'Tập trung vào âm nhạc', 'anc'), ('ambient', 'Xuyên âm', 'Nghe môi trường xung quanh', 'ambient')]:
            action = button('', lambda k=key: self.task(self.backend.set_value(20, 21, ['normal', 'anc', 'ambient'].index(k), 'noise', k)), css='mode')
            content = box(False, 11)
            content.append(icon(glyph, 25))
            copy = box(True, 3)
            copy.append(label(text, 'medium'))
            copy.append(label(description, 'tiny subtitle'))
            content.append(copy)
            action.set_child(content)
            accessible(action, text)
            self.noise_buttons[key] = self.gated(action)
            modes.append(action)
        card.append(modes)
        selectors = Columns(breakpoint=640, gap=18)
        anc, ambient = box(), box()
        self.choice(anc, 'Mức chống ồn', 'sub', {'DEFAULT': 'Mạnh', 'BALANCE': 'Cân bằng', 'ADAPTIVE': 'Thích ứng môi trường', 'EAR_ADAPTIVE': 'Thích ứng tai', 'WIND': 'Giảm tiếng gió'}, 58, 59, lambda v: ['EAR_ADAPTIVE', 'WIND', 'DEFAULT', 'BALANCE', 'ADAPTIVE'].index(v), vertical=True)
        self.choice(ambient, 'Chế độ xuyên âm', 'amb', {'DEFAULT': 'Mặc định', 'TOTAL': 'Tự nhiên'}, 60, 61, lambda v: int(v == 'TOTAL'), vertical=True)
        selectors.append(anc)
        selectors.append(ambient)
        card.append(selectors)
        quick = Columns(breakpoint=650)
        self.toggle(self.card(quick), 'Chế độ chơi game', 'game', 18, 19, description='Giảm độ trễ khi xem và chơi')
        self.toggle(self.card(quick), 'Nhận biết đeo tai', 'ear', 73, 74, inverse=True, direct=True,
                    warning='Thay đổi cảm biến đeo tai có thể làm tai nghe khởi động lại. Tiếp tục?', description='Phản hồi khi đeo hoặc tháo tai nghe')
        page.append(quick)
        volume_card = self.card(page, padding='summary-card')
        volume_row = box(False, 18)
        copy = box(True, 3)
        copy.append(label('Âm lượng Ubuntu', 'medium'))
        self.volume_text = label('— • Đầu ra EarFun', 'caption')
        copy.append(self.volume_text)
        volume_row.append(copy)
        self.volume_scale = accessible(Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 0, 100, 1), 'Âm lượng Ubuntu')
        self.volume_scale.set_value(50)
        self.volume_scale.set_hexpand(True)
        volume_row.append(self.volume_scale)
        volume_row.append(self.gated(button('Áp dụng', lambda: self.audio_volume(int(self.volume_scale.get_value())), True)))
        volume_row.append(button('Cài đặt âm thanh', self.open_sound, css='ghost small'))
        volume_card.append(volume_row)
        volume_row.insert_child_after(accessible(self.gated(button('↻', lambda: self.audio_volume(), css='ghost small')), 'Đọc âm lượng'), copy)

    def equalizer(self, page):
        self.eq_syncing = True
        self.preset_options = presets.options(self.settings.data['presets'])
        card = self.card(page, 'Bộ EQ EarFun Audio', '30 mẫu EarFun và 2 mẫu Fernanda cho Air Pro 4 • Chọn mẫu để nạp bản nháp')
        toolbar = box(False, 8)
        self.preset_picker = accessible(Gtk.DropDown.new_from_strings(self.preset_names()), 'Bản chỉnh EQ')
        self.preset_picker.set_size_request(250, -1)
        self.preset_picker.set_enable_search(True)
        toolbar.append(self.preset_picker)
        toolbar.append(button('Nạp bản chỉnh', self.load_preset))
        toolbar.append(button('Đặt lại', lambda: self.set_eq([0]*10), css='ghost'))
        card.append(toolbar)
        self.eq_sliders = []
        self.eq_curve = EqCurve(self.eq_values)
        self.eq_curve.add_css_class('eq-chart')
        card.append(self.eq_curve)
        faders = box(False, 0)
        faders.set_homogeneous(True)
        faders.set_margin_start(38)
        faders.set_margin_end(18)
        for i, freq in enumerate(p.FREQUENCIES):
            column = box(True, 4)
            value = label('0 dB', 'eq-gain')
            value.set_xalign(.5)
            slider = accessible(Gtk.Scale.new_with_range(Gtk.Orientation.VERTICAL, -10, 10, 1), f'EQ {freq:g} Hz')
            slider.add_css_class('eq')
            slider.set_round_digits(0)
            slider.set_inverted(True)
            slider.set_size_request(-1, 138)
            slider.set_halign(Gtk.Align.CENTER)
            self.eq_sliders.append(slider)
            def changed(s, text=value):
                gain = p.GAINS[int(s.get_value())+10]
                text.set_text(f'{gain:+g} dB' if gain else '0 dB')
                if not self.eq_syncing:
                    self.sync_eq_picker()
                self.eq_curve.queue_draw()
            slider.connect('value-changed', changed)
            column.append(value)
            column.append(slider)
            frequency = label(f'{freq/1000:g}k' if freq >= 1000 else f'{freq:g}', 'caption')
            frequency.set_xalign(.5)
            column.append(frequency)
            faders.append(column)
        card.append(faders)
        lower = box(False, 9)
        lower.set_margin_top(8)
        self.preset_name = accessible(Gtk.Entry(placeholder_text='Tên bản chỉnh', width_chars=16), 'Tên bản chỉnh EQ')
        lower.append(self.preset_name)
        lower.append(button('Lưu trên máy', self.save_preset))
        lower.append(spacer())
        lower.append(self.gated(button('Gửi tới tai nghe', self.apply_eq, True)))
        card.append(lower)
        manage = box(False, 8)
        manage.append(button('Đổi tên', self.rename_preset, css='ghost small'))
        manage.append(button('Xóa bản riêng', self.delete_preset, css='ghost small'))
        manage.append(button('Nhập từ tệp…', self.import_preset, css='small'))
        manage.append(button('Xuất bản nháp…', self.export_preset, css='small'))
        card.append(manage)
        self.eq_status = label('Bản chỉnh trên máy • Chưa gửi', 'caption')
        card.append(self.eq_status)
        page.append(label('Đường cong hiển thị bản chỉnh trên máy. Chưa hỗ trợ đọc lại EQ từ tai nghe.', 'caption'))
        self.eq_syncing = False
        self.preset_picker.connect('notify::selected', lambda *_: self.load_preset() if not self.eq_syncing else None)
        self.set_eq(self.settings.data['eq'])

    def set_eq(self, values, key=None):
        values = presets.validate_levels(values)
        self.eq_syncing = True
        try:
            for slider, value in zip(self.eq_sliders, values):
                slider.set_value(value)
            if key is not None:
                keys = [option[0] for option in self.preset_options]
                self.preset_picker.set_selected(keys.index(key) if key in keys else 0)
        finally:
            self.eq_syncing = False
        self.sync_eq_picker()
        self.eq_curve.queue_draw()

    def sync_eq_picker(self):
        levels = self.eq_values()
        index = self.preset_picker.get_selected()
        if index >= len(self.preset_options) or self.preset_options[index][2] != levels:
            index = next((i for i, option in enumerate(self.preset_options) if option[2] == levels), 0)
        self.eq_syncing = True
        self.preset_picker.set_selected(index)
        self.eq_syncing = False
        self.update_eq_status()

    def preset_names(self):
        return [option[1] for option in self.preset_options]

    def refresh_presets(self, selected=None):
        self.eq_syncing = True
        self.preset_options = presets.options(self.settings.data['presets'])
        self.preset_picker.set_model(Gtk.StringList.new(self.preset_names()))
        keys = [row[0] for row in self.preset_options]
        if selected in keys:
            self.preset_picker.set_selected(keys.index(selected))
        self.eq_syncing = False
        self.sync_eq_picker()

    def eq_values(self):
        return [int(s.get_value()) for s in self.eq_sliders]

    def load_preset(self):
        index = self.preset_picker.get_selected()
        if index >= len(self.preset_options):
            return
        key, name, levels = self.preset_options[index]
        if levels is None:
            return
        self.set_eq(levels, key=key)
        if key.startswith('custom:'):
            self.preset_name.set_text(key[7:])

    def save_preset(self):
        name = self.preset_name.get_text().strip()
        if not name or len(name) > 100:
            self.show_notice('Nhập tên bản chỉnh từ 1 đến 100 ký tự.', True)
            return
        levels = self.eq_values()
        def save():
            self.settings.data['presets'][name] = levels
            self.settings.data['eq'] = levels
            self.save_settings()
            self.refresh_presets('custom:'+name)
            self.show_notice('Đã lưu bản chỉnh trên máy.')
        if name in self.settings.data['presets']:
            self.confirm(f'Ghi đè bản chỉnh riêng “{name}”?', save)
        else:
            save()

    def selected_custom_name(self):
        index = self.preset_picker.get_selected()
        key = self.preset_options[index][0] if index < len(self.preset_options) else ''
        if not key.startswith('custom:'):
            self.show_notice('Chọn một bản chỉnh trong nhóm Cá nhân trước.', True)
            return None
        return key[7:]

    def rename_preset(self):
        old = self.selected_custom_name()
        new = self.preset_name.get_text().strip()
        if old is None:
            return
        if not new or len(new) > 100:
            self.show_notice('Nhập tên mới vào ô Tên bản chỉnh (1–100 ký tự).', True)
            return
        if new != old and new in self.settings.data['presets']:
            self.show_notice('Tên mới đã tồn tại. Hãy dùng tên khác.', True)
            return
        self.settings.data['presets'][new] = self.settings.data['presets'].pop(old)
        self.save_settings()
        self.refresh_presets('custom:'+new)
        self.show_notice('Đã đổi tên bản chỉnh riêng.')

    def delete_preset(self):
        name = self.selected_custom_name()
        if name is None:
            return
        def remove():
            del self.settings.data['presets'][name]
            self.save_settings()
            self.refresh_presets()
            self.show_notice('Đã xóa bản chỉnh riêng; bản nháp đang chỉnh vẫn được giữ.')
        self.confirm(f'Xóa bản chỉnh riêng “{name}” trên máy?', remove)

    def export_preset(self):
        name = self.preset_name.get_text().strip() or self.preset_options[self.preset_picker.get_selected()][1]
        document = presets.encode(name, self.eq_values())
        chooser = Gtk.FileChooserNative(title='Xuất bản nháp EQ', transient_for=self.window,
            action=Gtk.FileChooserAction.SAVE, accept_label='Lưu', cancel_label='Hủy')
        chooser.set_current_name('earfun-eq.json')
        def response(dialog, code):
            if code == Gtk.ResponseType.ACCEPT:
                try:
                    Path(dialog.get_file().get_path()).write_text(json.dumps(document, ensure_ascii=False, indent=2)+'\n')
                    self.show_notice('Đã xuất bản nháp EQ ra tệp.')
                except OSError as error:
                    self.show_notice(str(error), True)
            dialog.destroy()
        chooser.connect('response', response)
        chooser.show()

    def import_preset(self):
        chooser = Gtk.FileChooserNative(title='Nhập bản chỉnh EQ EarFun Studio', transient_for=self.window,
            action=Gtk.FileChooserAction.OPEN, accept_label='Nhập', cancel_label='Hủy')
        file_filter = Gtk.FileFilter()
        file_filter.set_name('Bản chỉnh EQ (*.json)')
        file_filter.add_pattern('*.json')
        chooser.add_filter(file_filter)
        def response(dialog, code):
            if code == Gtk.ResponseType.ACCEPT:
                try:
                    with open(dialog.get_file().get_path(), 'rb') as source:
                        data = source.read(65537)
                    if len(data) > 65536:
                        raise ValueError('Tệp EQ quá lớn (tối đa 64 KiB).')
                    name, levels = presets.decode(json.loads(data))
                    self.set_eq(levels)
                    self.preset_name.set_text(name)
                    self.show_notice('Đã nhập vào bản nháp. Bấm Lưu trên máy hoặc Gửi tới tai nghe để áp dụng.')
                except (OSError, ValueError, UnicodeError) as error:
                    self.show_notice(str(error), True)
            dialog.destroy()
        chooser.connect('response', response)
        chooser.show()

    def update_eq_status(self):
        levels = tuple(self.eq_values())
        name = self.preset_options[self.preset_picker.get_selected()][1]
        if self.eq_sending_levels is not None:
            status = 'Đang gửi bản này…' if levels == self.eq_sending_levels else 'Đang gửi bản trước • Bản này chưa gửi'
        elif levels == self.eq_sent_levels:
            status = 'Đã gửi trong phiên này • Chưa đọc lại được EQ'
        else:
            status = 'Chưa gửi'
        self.eq_status.set_text(f'Bản nháp: {name} • {status}')

    def apply_eq(self):
        levels = tuple(self.eq_values())
        generation = self.connection_generation
        self.settings.data['eq'] = list(levels)
        self.save_settings()
        self.eq_sending_levels = levels
        self.update_eq_status()
        future = self.task(self.backend.set_eq(list(levels)))
        def done(f):
            error = f.cancelled() or f.exception() is not None
            GLib.idle_add(self.finish_eq, levels, generation, error)
        future.add_done_callback(done)

    def finish_eq(self, levels, generation, error):
        self.eq_sending_levels = None
        if not error and generation == self.connection_generation:
            self.eq_sent_levels = levels
        self.update_eq_status()
        if error:
            self.show_notice('Chưa gửi đủ EQ. Bản nháp được giữ để bạn thử lại.', True)
        return False

    def gestures(self, page):
        self.key_widgets = {}
        sides = Columns()
        for side, title in [('L', 'Tai trái'), ('R', 'Tai phải')]:
            card = self.card(sides, title, 'Chọn chức năng cho từng thao tác.')
            for gesture in p.GESTURES:
                group = box(True, 4)
                group.append(label(gesture))
                row = box(False, 8)
                widget = accessible(Gtk.DropDown.new_from_strings(['—'] + list(p.OPERATIONS)), f'{title}: {gesture}')
                widget.set_hexpand(True)
                field = side+'.'+gesture
                self.key_widgets[field] = widget
                widget.connect('notify::selected', lambda w, _, f=field: self.mark_key_dirty(f, w) if not self.updating else None)
                def save(s=side, g=gesture, w=widget):
                    idx = w.get_selected()
                    if 1 <= idx <= len(p.OPERATIONS):
                        self.task(self.backend.set_key(s, g, list(p.OPERATIONS)[idx-1]))
                row.append(widget)
                row.append(button('Lưu', save, css='small'))
                self.gesture_rows[field] = row
                group.append(row)
                card.append(self.gated(group))
            def lock(on, s=side):
                if all(key in self.state for key in ('dis_l', 'dis_r')):
                    self.task(self.backend.disable_touch(on if s == 'L' else self.state['dis_l'], on if s == 'R' else self.state['dis_r']))
            widget = self.gated(self.switch_row(card, 'Tắt cảm ứng bên này', lock))
            self.switch_bindings['dis_'+side.lower()] = widget
            if side == 'L':
                self.touch_l = widget
            else:
                self.touch_r = widget
        page.append(sides)

    def multipoint(self, page):
        card = self.card(page, 'Kết nối đa điểm', 'Nghe và chuyển đổi giữa tối đa hai thiết bị.')
        self.toggle(card, 'Cho phép kết nối hai thiết bị', 'dual', 39, 38, direct=True, warning='Thay đổi đa điểm có thể ngắt nhạc và khởi động lại tai nghe. Tiếp tục?')
        actions = box(False, 10)
        actions.append(self.gated(button('Làm mới danh sách', lambda: self.task(self.backend.phones_action()))))
        actions.append(self.gated(button('Ghép thiết bị mới', lambda: self.confirm('Cho tai nghe tìm thiết bị Bluetooth mới? Tiếp tục?', lambda: self.task(self.backend.phones_action(43))), True)))
        card.append(actions)
        self.phone_box = self.card(page, 'Thiết bị đã lưu')
        self.phone_rows = box()
        self.phone_box.append(self.phone_rows)
        self.render_phones([])
        page.append(label('Danh sách do tai nghe cung cấp. Ngắt thiết bị có thể làm gián đoạn âm thanh.', 'caption'))

    def earbuds(self, page):
        card = self.card(page, 'Âm thanh & cuộc gọi')
        self.choice(card, 'Âm lượng thông báo', 'prompt', {v: f'{v}%' if v else 'Tắt' for v in (100, 75, 50, 25, 0)}, 56, 57, lambda v: [100, 75, 50, 25, 0].index(v))
        self.voice_language = label('EarFun Audio', 'subtitle')
        self.row(card, 'Gói giọng thông báo', self.voice_language, 'Cài bằng ứng dụng trên điện thoại; bản Ubuntu chưa hỗ trợ.')
        self.choice(card, 'Micro cuộc gọi', 'mic', {'AUTO': 'Tự động', 'L': 'Tai trái', 'R': 'Tai phải'}, 48, 49, lambda v: ['AUTO', 'L', 'R'].index(v))
        self.choice(card, 'Giới hạn âm lượng', 'hearing_limit', dict(enumerate(['Tắt', '90 dB', '80 dB', '70 dB', '60 dB', '50 dB'])), 83, 84)
        balance_row = box(False, 12)
        self.balance = accessible(Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 0, 100, 1), 'Cân bằng trái/phải')
        self.balance.set_hexpand(True)
        self.balance.set_value(50)
        balance_row.set_size_request(320, -1)
        balance_row.append(self.gated(self.balance))
        def apply_balance():
            value = int(self.balance.get_value())
            self.task(self.backend.set_value(85, 86, value, 'balance', value))
        self.balance_action = self.gated(button('Áp dụng', apply_balance, css='small'))
        balance_row.append(self.balance_action)
        self.balance.connect('value-changed', lambda *_: self.mark_balance_dirty() if not self.updating else None)
        self.row(card, 'Cân bằng trái/phải (50 = giữa)', balance_row)
        card = self.card(page, 'Kết nối Bluetooth', 'Codec cần được cả Ubuntu và bộ phát hỗ trợ.')
        self.choice(card, 'Chế độ Bluetooth', 'le_audio', {False: 'Thông thường · Google Fast Pair', True: 'LE Audio'}, 75, 76, int)
        self.codec_checks = {}
        for key, text in [('ldac', 'LDAC'), ('aptx', 'aptX'), ('adaptive', 'aptX Adaptive'), ('lossless', 'aptX Lossless')]:
            def changed(on, k=key):
                if not all(name in self.state for name in ('ldac', 'aptx', 'adaptive', 'lossless')):
                    self.show_notice('Chờ tai nghe trả về đủ cài đặt codec.', True)
                    return
                values = {name: self.state[name] for name in ('ldac', 'aptx', 'adaptive', 'lossless')}
                values[k] = on
                self.request_plan(workflows.codecs, values)
            widget = self.gated(self.switch_row(card, text, changed))
            self.codec_checks[key] = widget
            self.switch_bindings[key] = widget
        card = self.card(page, 'Tìm tai nghe', 'Tháo tai nghe khỏi tai. Cần giữ kết nối để gửi lệnh dừng sau 10 giây.')
        actions = box(False, 9)
        for mask, title in ((1, 'Phát tiếng tai trái'), (2, 'Phát tiếng tai phải')):
            actions.append(self.gated(button(title, lambda m=mask: self.confirm('Bạn đã tháo tai nghe khỏi tai? Giữ kết nối để gửi dừng sau 10 giây.', lambda: self.task(self.backend.find(m))))))
        actions.append(self.gated(button('Dừng tìm', lambda: self.task(self.backend.find(0)))))
        card.append(actions)
        card = self.card(page, 'Cài đặt nâng cao', 'Khôi phục sẽ xóa thiết lập và các thiết bị đã ghép trên tai nghe.')
        reset = self.gated(button('Khôi phục cài đặt gốc…', lambda: self.confirm('Xóa cài đặt và các kết nối đã ghép trên tai nghe? Bạn sẽ cần ghép lại Bluetooth.', lambda: self.task(self.backend.direct(54))), css='danger'))
        reset.set_halign(Gtk.Align.START)
        card.append(reset)

    def application(self, page):
        card = self.card(page, 'Tùy chọn ứng dụng')
        auto = self.switch_row(card, 'Tự kết nối lại khi ứng dụng đang chạy', self.set_auto, confirmed=True)
        self.updating = True
        auto.set_active(self.settings.data['auto_connect'])
        self.updating = False
        card.append(label('Đóng cửa sổ sẽ thoát ứng dụng. Bản Ubuntu hiện không thu nhỏ xuống khay.', 'caption'))
        card = self.card(page, 'Thông tin & nhật ký', 'Chỉ lưu trên máy. Nhật ký có thể chứa tên và địa chỉ thiết bị Bluetooth.')
        self.firmware = label('Firmware —  /  Codec —', 'caption')
        card.append(self.firmware)
        self.log_view = Gtk.TextView(editable=False, cursor_visible=False, monospace=True, wrap_mode=Gtk.WrapMode.WORD_CHAR)
        scroll = Gtk.ScrolledWindow(min_content_height=180, max_content_height=210)
        scroll.add_css_class('log-frame')
        scroll.set_child(self.log_view)
        card.append(scroll)
        actions = box(False, 8)
        actions.append(button('Xuất nhật ký…', self.export_log))
        self.read_button = self.gated(button('Đọc lại trạng thái', lambda: self.task(self.backend.query())))
        actions.append(self.read_button)
        card.append(actions)
        card = self.card(page, 'EarFun Studio 1.2', 'Ứng dụng Ubuntu độc lập. Giao diện đồng bộ với EarFun Studio Windows.')
        card.append(label('EQ cá nhân hóa theo thính lực, firmware/gói giọng nói, tham gia Auracast, AI/dịch thuật và hồ sơ đám mây hiện vẫn cần EarFun Audio chính hãng.', 'subtitle'))
        actions = box(False, 8)
        actions.append(button('Nguồn và giới hạn', lambda: Gio.AppInfo.launch_default_for_uri((ASSETS.parent/'README.md').as_uri(), None), css='ghost'))
        actions.append(button('Thoát ứng dụng', self.close))
        card.append(actions)

    def open_sound(self):
        self.open_settings('sound')

    def open_settings(self, panel):
        try:
            subprocess.Popen(['gnome-control-center', panel])
        except OSError as ex:
            self.show_notice(str(ex), True)

    def open_bluetooth(self):
        try:
            subprocess.Popen(['gnome-control-center', 'bluetooth'])
        except OSError as ex:
            self.show_notice(str(ex), True)

    def set_auto(self, on):
        self.settings.data['auto_connect'] = on
        self.save_settings()

    def save_settings(self):
        try:
            self.settings.save()
        except OSError as ex:
            self.show_notice('Không lưu được cấu hình: ' + str(ex), True)

    def audio_volume(self, value=None):
        def work():
            try:
                result = audio.volume(self.address, value)
                GLib.idle_add(self.volume_done, result)
            except Exception as ex:
                GLib.idle_add(self.show_notice, str(ex), True)
        self.executor.submit(work)

    def volume_done(self, result):
        try:
            self.volume_scale.set_value(float(result.split()[1])*100)
        except (ValueError, IndexError):
            pass
        self.volume_text.set_text(f'{int(self.volume_scale.get_value())}% • Đầu ra EarFun')
        self.show_notice(result)

    def export_log(self):
        chooser = Gtk.FileChooserNative(title='Lưu nhật ký Bluetooth', transient_for=self.window, action=Gtk.FileChooserAction.SAVE, accept_label='Lưu', cancel_label='Hủy')
        chooser.set_current_name('earfun-bluetooth.log')
        def response(dialog, code):
            if code == Gtk.ResponseType.ACCEPT:
                try:
                    Path(dialog.get_file().get_path()).write_text('\n'.join(self.backend.log), encoding='utf-8')
                    self.show_notice('Đã xuất nhật ký.')
                except OSError as ex:
                    self.show_notice(str(ex), True)
            dialog.destroy()
        chooser.connect('response', response)
        chooser.show()

    def confirm(self, text, callback):
        dialog = Gtk.MessageDialog(transient_for=self.window, modal=True, text=text, message_type=Gtk.MessageType.WARNING, buttons=Gtk.ButtonsType.OK_CANCEL)
        def response(d, code):
            d.destroy()
            if code == Gtk.ResponseType.OK:
                callback()
            else:
                self.render_state()
        dialog.connect('response', response)
        dialog.present()

    def show_notice(self, text, error=False):
        self.notice.set_text(text)
        if error:
            self.notice.add_css_class('error')
        else:
            self.notice.remove_css_class('error')
        return False

    def task(self, coro):
        self.busy += 1
        self.refresh_controls()
        future = self.backend.submit(coro)
        future.add_done_callback(lambda _: GLib.idle_add(self.task_done))
        return future

    def task_done(self):
        self.busy = max(0, self.busy-1)
        self.refresh_controls()
        self.render_state()
        return False

    def scan(self):
        self.task(self.backend.scan())

    def connect_loaded(self):
        if not self.connected and self.devices:
            self.toggle_connect()
        return False

    def toggle_connect(self):
        if self.connected:
            self.user_disconnected = True
            self.task(self.backend.disconnect())
            return
        idx = self.device_picker.get_selected()
        if idx >= len(self.devices):
            self.show_notice('Bấm Tìm thiết bị và chọn tai nghe trước.', True)
            return
        self.address = self.devices[idx][0]
        self.user_disconnected = False
        self.task(self.backend.connect(self.address))

    def reconnect(self):
        if self.settings.data['auto_connect'] and not self.user_disconnected and not self.connected and not self.busy and self.address:
            self.task(self.backend.connect(self.address))
        return True

    def refresh_controls(self):
        enabled = self.connected and not self.busy
        for widget in self.controls + self.phone_buttons:
            widget.set_sensitive(enabled)
        self.scan_button.set_sensitive(not self.busy and not self.connected)
        self.device_picker.set_sensitive(not self.busy and not self.connected)
        self.connect_button.set_sensitive(not self.busy)
        self.connect_button.set_label('Ngắt kết nối' if self.connected else 'Đang kết nối…' if self.connecting else 'Kết nối')
        for key, group in self.choice_groups:
            available = enabled and key in self.state
            group.set_sensitive(available)
            widget = group.get_first_child()
            values = next(values for binding, _, values in self.bindings if binding == key)
            index = widget.get_selected()
            wanted = values[index-1] if 1 <= index <= len(values) else None
            group.get_last_child().set_sensitive(available and wanted is not None and wanted != self.state.get(key))
        actions = list(p.OPERATIONS)
        for key, row in self.gesture_rows.items():
            index = row.get_first_child().get_selected()
            wanted = actions[index-1] if 1 <= index <= len(actions) else None
            row.get_last_child().set_sensitive(enabled and wanted is not None and wanted != self.state.get('keys', {}).get(key))
        for key, widget in self.switch_bindings.items():
            known = key in self.state
            if key.startswith('dis_'):
                known = all(k in self.state for k in ('dis_l', 'dis_r'))
            if key in self.codec_checks:
                known = all(k in self.state for k in ('aptx', 'adaptive', 'lossless', 'ldac', 'dual'))
            widget.set_sensitive(enabled and known)
            widget.set_tooltip_text(None if known else 'Chưa đọc được trạng thái từ tai nghe')
        self.balance.set_sensitive(enabled and 'balance' in self.state)
        self.balance_action.set_sensitive(enabled and 'balance' in self.state and int(self.balance.get_value()) != self.state['balance'])

    def handle(self, event, value):
        if event == 'devices':
            self.devices = value
            self.device_picker.set_model(Gtk.StringList.new([name for mac, name in value] or ['Chưa tìm thấy tai nghe']))
            for i, (mac, _) in enumerate(value):
                if mac == self.address:
                    self.device_picker.set_selected(i)
        elif event == 'connection':
            if value == 'disconnected':
                self.connection_generation += 1
                self.eq_sent_levels = None
                self.update_eq_status()
            self.connected = value == 'connected'
            self.connecting = value == 'connecting'
            self.badge.set_text({'connected': 'Đã kết nối', 'connecting': 'Đang kết nối…', 'disconnected': 'Chưa kết nối'}[value])
            self.sidebar_status.set_text('Bluetooth LE • ' + self.badge.get_text())
            self.refresh_controls()
        elif event == 'connected_address':
            self.address = value
            self.settings.data['last_mac'] = value
            self.save_settings()
        elif event == 'state':
            self.state = value
            self.render_state()
        elif event == 'phones':
            self.render_phones(value)
        elif event in ('notice', 'error'):
            if event == 'notice' and value == 'Đã đọc trạng thái trực tiếp từ tai nghe.' and not self.busy:
                return False
            self.show_notice(value, event == 'error')
        elif event == 'log':
            self.log_view.get_buffer().set_text('\n'.join(self.backend.log))
        return False

    def render_state(self, force=False):
        self.updating = True
        changed = lambda key: force or self.state.get(key) != self.rendered_state.get(key)
        try:
            for key, widget, values in self.bindings:
                val = self.state.get(key)
                idx = widget.get_selected()
                draft = values[idx-1] if 1 <= idx <= len(values) else None
                if key in self.dirty_choices and draft != val:
                    widget.add_css_class('draft-field')
                    widget.set_tooltip_text('Chưa áp dụng • Trạng thái tai nghe: ' + str(val if val is not None else 'chưa đọc được'))
                    continue
                self.dirty_choices.discard(key)
                widget.remove_css_class('draft-field')
                widget.set_tooltip_text(None)
                if changed(key) or draft != val:
                    widget.set_selected(values.index(val)+1 if val in values else 0)
            for key, widget in self.batteries.items():
                value = self.state.get(key)
                widget.set_text(f'{value}%' if value is not None else '—')
                self.battery_bars[key].set_fraction(value/100 if value is not None else 0)
                self.battery_bars[key].set_opacity(1 if value is not None else 0)
            self.codec_badge.set_text(self.state.get('codec', '—'))
            self.voice_language.set_text({'EN': 'English', 'ZH': '中文', 'DE': 'Deutsch', 'JA': '日本語'}.get(self.state.get('voice_language'), 'EarFun Audio'))
            self.firmware.set_text(f'Firmware {self.state.get("version", "—")}  /  Codec {self.state.get("codec", "—")}')
            keys = self.state.get('keys', {})
            actions = list(p.OPERATIONS)
            for key, widget in self.key_widgets.items():
                value = keys.get(key)
                idx = widget.get_selected()
                draft = actions[idx-1] if 1 <= idx <= len(actions) else None
                if key in self.dirty_keys and draft != value:
                    widget.add_css_class('draft-field')
                    widget.set_tooltip_text('Chưa lưu • Tai nghe: ' + (value or 'chưa đọc được'))
                    continue
                self.dirty_keys.discard(key)
                widget.remove_css_class('draft-field')
                widget.set_tooltip_text(None)
                if changed('keys') or draft != value:
                    widget.set_selected(actions.index(value)+1 if value in actions else 0)
            for key, widget in self.switch_bindings.items():
                widget.set_active(self.state.get(key, False))
                widget.set_state(self.state.get(key, False))
            actual_balance = self.state.get('balance')
            if not self.balance_dirty or int(self.balance.get_value()) == actual_balance:
                self.balance_dirty = False
                self.balance.remove_css_class('draft-field')
                if changed('balance'):
                    self.balance.set_value(actual_balance if actual_balance is not None else 50)
            for key, widget in self.noise_buttons.items():
                if self.state.get('noise') == key:
                    widget.add_css_class('active')
                else:
                    widget.remove_css_class('active')
        finally:
            self.updating = False
        self.rendered_state = dict(self.state)
        self.refresh_controls()

    def render_phones(self, phones):
        self.phone_buttons = []
        child = self.phone_rows.get_first_child()
        while child:
            following = child.get_next_sibling()
            self.phone_rows.remove(child)
            child = following
        if not phones:
            self.phone_rows.append(label('Chưa có dữ liệu thiết bị.', 'subtitle'))
        for phone in phones:
            row = box(False)
            text = label(f'{phone["name"] or "Thiết bị"} · {phone["mac"]}\n' + ('Đang kết nối' if phone['connected'] else 'Đã lưu'))
            text.set_hexpand(True)
            row.append(text)
            command = 42 if phone['connected'] else 41
            widget = button('Ngắt' if command == 42 else 'Kết nối', lambda c=command, n=phone['num']: self.confirm('Thay đổi kết nối thiết bị này có thể ngắt âm thanh. Tiếp tục?', lambda: self.task(self.backend.phones_action(c, n))))
            widget.set_sensitive(self.connected and not self.busy)
            self.phone_buttons.append(widget)
            row.append(widget)
            self.phone_rows.append(row)

    def close(self, *_):
        if self.backend.closing:
            return False
        self.backend.closing = True
        self.settings.data['eq'] = self.eq_values()
        self.save_settings()
        self.window.set_sensitive(False)
        future = self.backend.submit(self.backend.disconnect())
        future.add_done_callback(lambda _: GLib.idle_add(self.finish_close))
        return True

    def finish_close(self):
        self.backend.loop.call_soon_threadsafe(self.backend.loop.stop)
        self.executor.shutdown(wait=False, cancel_futures=True)
        self.window.destroy()
        self.quit()
        return False

    def smoke_pages(self, index):
        names = list(self.pages)
        if index < len(names):
            self.navigate(names[index])
            print('UI OK:', names[index], flush=True)
            GLib.timeout_add(200, self.smoke_pages, index+1)
        else:
            self.close()
        return False
