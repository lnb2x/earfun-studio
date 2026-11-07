"""GTK equivalents of the supplied Windows theme primitives."""
import ctypes
from pathlib import Path
import gi
gi.require_version('Gtk', '4.0')
from gi.repository import Gtk, PangoCairo, GdkPixbuf, Gdk
from .protocol import GAINS

ASSETS = Path(__file__).resolve().parent.parent / 'assets'


def load_fonts():
    # Register bundled OFL fonts in this process, without installing system fonts.
    fc = ctypes.CDLL('libfontconfig.so.1')
    fc.FcConfigGetCurrent.restype = ctypes.c_void_p
    fc.FcConfigAppFontAddFile.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    config = fc.FcConfigGetCurrent()
    for path in (ASSETS/'fonts').glob('*.ttf'):
        fc.FcConfigAppFontAddFile(config, str(path).encode())
    PangoCairo.FontMap.get_default().changed()


def icon(name, size=20):
    image = Gtk.Image.new_from_file(str(ASSETS/'icons'/f'{name}.svg'))
    image.set_pixel_size(size)
    return image


class Columns(Gtk.FlowBox):
    """Wrap full cards without imposing a desktop-width minimum on the window."""
    def __init__(self, breakpoint=780, gap=14, columns=2):
        super().__init__(selection_mode=Gtk.SelectionMode.NONE, homogeneous=True,
                         min_children_per_line=1, max_children_per_line=columns,
                         column_spacing=gap, row_spacing=gap, hexpand=True,
                         valign=Gtk.Align.START)
        self.child_minimum = int((breakpoint-gap*(columns-1))/columns)
        self.add_css_class('columns')

    def append(self, child):
        child.set_hexpand(True)
        child.set_size_request(self.child_minimum, -1)
        Gtk.FlowBox.append(self, child)


class EqCurve(Gtk.Picture):
    def __init__(self, levels):
        super().__init__(hexpand=True, can_shrink=True)
        self.set_size_request(-1, 104)
        self.set_content_fit(Gtk.ContentFit.FILL)
        self.levels = levels
        self.last_frame = None
        self.update_property([Gtk.AccessibleProperty.LABEL], ['Đường cong equalizer 10 dải, bản chỉnh trên máy'])
        self.add_tick_callback(self.render_curve)

    def render_curve(self, *_):
        width, height = self.get_width(), self.get_height()
        gains = tuple(GAINS[v+10] for v in self.levels())
        frame = width, height, gains
        if width < 50 or height < 30 or frame == self.last_frame:
            return True
        self.last_frame = frame
        left, right, top, bottom = 38, width-18, 13, height-13
        y = lambda gain: top + (6-gain)/12*(bottom-top)
        parts = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}"><rect width="100%" height="100%" rx="13" fill="#f8fafe"/>']
        for db in (6, 3, 0, -3, -6):
            text = f'{db:+}' if db > 0 else str(db)
            parts.append(f'<text x="10" y="{y(db)+3}" font-family="Be Vietnam Pro" font-size="10" fill="#7c8296">{text}</text>')
            parts.append(f'<path d="M{left},{y(db)} H{right}" stroke="#e2e9f6" stroke-dasharray="2 2"/>')
        points = [(left+(right-left)*(i+.5)/10, y(v)) for i, v in enumerate(gains)]
        for x, _ in points:
            parts.append(f'<path d="M{x},{top} V{bottom}" stroke="#edf1f9"/>')
        path = f'M{points[0][0]},{points[0][1]}'
        for i in range(9):
            a, b, c, d = points[max(i-1, 0)], points[i], points[i+1], points[min(i+2, 9)]
            path += f' C{b[0]+(c[0]-a[0])/6},{b[1]+(c[1]-a[1])/6} {c[0]-(d[0]-b[0])/6},{c[1]-(d[1]-b[1])/6} {c[0]},{c[1]}'
        parts.append(f'<path d="{path}" fill="none" stroke="#6685d1" stroke-width="2.5"/>')
        for x, yy in points:
            parts.append(f'<circle cx="{x}" cy="{yy}" r="3" fill="white" stroke="#6685d1" stroke-width="1.8"/>')
        parts.append('</svg>')
        loader = GdkPixbuf.PixbufLoader.new_with_type('svg')
        loader.write(''.join(parts).encode())
        loader.close()
        self.set_paintable(Gdk.Texture.new_for_pixbuf(loader.get_pixbuf()))
        return True
