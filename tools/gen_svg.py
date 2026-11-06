"""TRAPDOOR main windowの SVG デザインを生成する（サンプルデータはすべて架空）。"""
import sys
from xml.sax.saxutils import escape as esc

W, H = 1920, 1080
C = dict(bg="#0E0F10", panel="#17181A", panel2="#1F2022", line="#2A2C2F", line2="#3A3D41",
         text="#D9DCDF", sub="#8A9096", dim="#5C6268",
         red="#E5383B", green="#2FB344", yellow="#F2A900", blue="#D0D5DA")
JP = "IBM Plex Sans Condensed, Noto Sans CJK JP, sans-serif"
MONO = "IBM Plex Mono, monospace"

out = []
def a(s): out.append(s)
def rect(x, y, w, h, fill="none", stroke=None, sw=1, rx=0, extra=""):
    rx = min(rx, 2)
    st = f' stroke="{stroke}" stroke-width="{sw}"' if stroke else ""
    a(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}"{st} {extra}/>')
def text(x, y, s, size=14, fill=C["text"], font=JP, weight=400, anchor="start", extra=""):
    a(f'<text x="{x}" y="{y}" font-family="{font}" font-size="{size}" font-weight="{weight}" '
      f'fill="{fill}" text-anchor="{anchor}" {extra}>{esc(s)}</text>')
def line(x1, y1, x2, y2, stroke=C["line"], sw=1, extra=""):
    a(f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="{sw}" {extra}/>')

# 架空の素材サムネ用の配色
THUMBS = {
    "count": ("#1b2a4a", "#0d1424"), "open": ("#5b2a86", "#e5484d"), "bgm": ("#0f3d3e", "#30a46c"),
    "guest": ("#3a2f12", "#f5a524"), "award": ("#4a1d2c", "#ffb3c1"), "break": ("#12304a", "#3e9bff"),
    "end": ("#2b2b2b", "#d0d0d0"), "logo": ("#0b0d10", "#3e9bff"), "bars": None, "black": ("#000", "#000"),
    "photo": ("#203a2a", "#a3e3b5"), "sponsor": ("#33240f", "#ffd27a"),
}

def defs():
    a("<defs>")
    for k, v in THUMBS.items():
        if v:
            a(f'<linearGradient id="g_{k}" x1="0" y1="0" x2="1" y2="1">'
              f'<stop offset="0" stop-color="{v[0]}"/><stop offset="1" stop-color="{v[1]}"/></linearGradient>')
    a('<linearGradient id="meter" x1="0" y1="1" x2="0" y2="0">'
      f'<stop offset="0" stop-color="{C["green"]}"/><stop offset="0.72" stop-color="{C["green"]}"/>'
      f'<stop offset="0.86" stop-color="{C["yellow"]}"/><stop offset="1" stop-color="{C["red"]}"/></linearGradient>')
    a('<pattern id="hatch" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">'
      f'<line x1="0" y1="0" x2="0" y2="8" stroke="{C["line2"]}" stroke-width="2"/></pattern>')
    a("</defs>")

def thumb(x, y, w, h, kind, label=None):
    if kind == "bars":
        cols = ["#c0c0c0", "#c0c000", "#00c0c0", "#00c000", "#c000c0", "#c00000", "#0000c0"]
        bw = w / 7
        for i, c in enumerate(cols):
            rect(round(x + i * bw, 2), y, round(bw + .5, 2), h * .7, c)
        rect(x, y + h * .7, w, h * .3, "#101010")
    else:
        rect(x, y, w, h, f"url(#g_{kind})")
        # 抽象的なモチーフ（実在の映像ではない）
        if kind == "count":
            text(x + w / 2, y + h * .68, "10", size=h * .5, fill="#ffffff", font=MONO, weight=700, anchor="middle", extra='opacity="0.85"')
        elif kind in ("open", "award", "photo"):
            a(f'<circle cx="{x + w*.7}" cy="{y + h*.45}" r="{h*.28}" fill="#ffffff" opacity="0.18"/>')
            rect(x + w * .1, y + h * .62, w * .45, h * .08, "#ffffff", extra='opacity="0.6"')
            rect(x + w * .1, y + h * .76, w * .3, h * .06, "#ffffff", extra='opacity="0.35"')
        elif kind in ("bgm",):
            for i in range(9):
                bh = h * (.15 + .5 * abs(((i * 37) % 10) - 5) / 5)
                rect(round(x + w * .12 + i * w * .085, 2), round(y + h * .8 - bh, 2), round(w * .05, 2), round(bh, 2), "#ffffff", extra='opacity="0.5"')
        elif kind in ("guest", "sponsor"):
            a(f'<path d="M{x+w*.1},{y+h*.85} L{x+w*.4},{y+h*.35} L{x+w*.6},{y+h*.65} L{x+w*.75},{y+h*.45} L{x+w*.95},{y+h*.85} Z" fill="#ffffff" opacity="0.25"/>')
        elif kind in ("break", "logo"):
            cx, cy, r = x + w / 2, y + h / 2, h * .26
            a(f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="#ffffff" stroke-width="{max(1.5, h*.03)}" opacity="0.7"/>')
            a(f'<path d="M{cx-r*.35},{cy-r*.5} L{cx+r*.55},{cy} L{cx-r*.35},{cy+r*.5} Z" fill="#ffffff" opacity="0.7"/>')
        elif kind == "end":
            rect(x + w * .25, y + h * .3, w * .5, h * .06, "#ffffff", extra='opacity="0.7"')
            for i in range(3):
                rect(x + w * .3, y + h * (.48 + i * .12), w * .4, h * .04, "#ffffff", extra='opacity="0.35"')
    if label:
        rect(x + 4, y + h - 18, len(label) * 7 + 10, 14, "#000000", rx=2, extra='opacity="0.7"')
        text(x + 9, y + h - 7, label, size=10, fill="#ffffff", font=MONO)

def wave(x, y, w, h, seed=1, step=3, color=C["sub"], played=0.0, pcolor=C["red"], opacity=1.0):
    """架空の音声波形（中心から上下対称の棒）。played までは pcolor。"""
    import math
    n = int(w // step)
    mid = y + h / 2
    d1, d2 = [], []
    for i in range(n):
        t = i / max(n - 1, 1)
        env = 0.35 + 0.45 * abs(math.sin(t * 3.1 * seed + seed)) + 0.2 * abs(math.sin(t * 23 + seed * 2))
        v = env * (0.75 + 0.25 * abs(math.sin(i * 1.7 + seed * 5) * math.cos(i * 0.43 + seed)))
        a_ = max(0.6, v * h / 2)
        seg = f"M{x+i*step+0.5:.1f},{mid-a_:.1f}V{mid+a_:.1f}"
        (d1 if t < played else d2).append(seg)
    sw = max(1.0, step - 1)
    if d1:
        a(f'<path d="{"".join(d1)}" stroke="{pcolor}" stroke-width="{sw}" opacity="{opacity}"/>')
    a(f'<path d="{"".join(d2)}" stroke="{color}" stroke-width="{sw}" opacity="{opacity}"/>')

# ---------- 共通：ヘッダー ----------
def header(locked=False):
    rect(0, 0, W, 56, C["panel"])
    line(0, 56, W, 56)
    # ロゴ（自作ツール用の仮マーク）
    # TRAPDOOR の印：床（白い枠）と、手前に開いた仕掛け扉（赤）
    rect(20, 15, 28, 26, "none", C["text"], 2)
    rect(25, 30, 18, 8, "#000")
    a(f'<path d="M25,30 L43,30 L39,18 L29,18 Z" fill="{C["red"]}"/>')
    text(60, 36, "TRAPDOOR", size=19, font=MONO, weight=700, extra='letter-spacing="3"')
    line(232, 16, 232, 40, C["line2"])
    text(248, 26, "SHOW", size=10, fill=C["dim"], font=MONO, extra='letter-spacing="1"')
    text(248, 44, "Sample Ceremony 2026 (fictional).trapdoor", size=14, fill=C["text"])
    text(548, 44, "● Saved 14:31", size=11, fill=C["sub"])
    # 時計
    text(W / 2, 39, "14:32:08", size=26, font=MONO, weight=700, anchor="middle")
    text(W / 2 + 76, 39, "JST", size=10, fill=C["dim"], font=MONO)
    # 状態チップ
    def chip(x, w, dot, label, val, active=True):
        rect(x, 13, w, 30, C["panel2"], C["line2"], 1, rx=6)
        a(f'<circle cx="{x+14}" cy="28" r="4.5" fill="{dot}"/>')
        text(x + 26, 32.5, label, size=11, fill=C["text"] if active else C["dim"], font=MONO, weight=700)
        text(x + 26 + len(label) * 7.2 + 8, 32.5, val, size=11, fill=C["sub"] if active else C["dim"], font=MONO)
    x = 1170
    chip(x, 236, C["green"], "OUT1", "DISP2 1080p59.94"); x += 246
    chip(x, 78, C["dim"], "NDI", "—", False); x += 88
    chip(x, 78, C["dim"], "SDI", "—", False); x += 88
    chip(x, 92, C["green"], "DROP", "0"); x += 102
    # ロック
    if locked:
        rect(x, 11, 154, 34, C["yellow"], rx=6)
        text(x + 77, 33, "LOCKED", size=13, fill="#1a1200", weight=700, anchor="middle")
    else:
        rect(x, 11, 154, 34, "none", C["line2"], 1.5, rx=6)
        text(x + 77, 33, "Show Lock  Ctrl+L", size=13, fill=C["sub"], anchor="middle")

# ---------- キューリスト ----------
CUES = [
    # num, name, note, kind, dur, end, endlabel, fade, vol, state
    ("1", "Pre-show Loop", "Runs during house open", "logo", "00:00:30.00", "loop", "Loop", "—/0.5", "-6.0", "done"),
    ("2", "Countdown 10s", "GO on MC 'starting soon'", "count", "00:00:10.00", "next", "Next", "—/—", "0.0", "done"),
    ("3", "Opening Video", "", "open", "00:01:30.00", "hold", "Hold last", "—/0.5", "0.0", "program"),
    ("4", "Guest Intro BG Loop", "Name supers on separate system", "guest", "00:00:20.00", "loop", "Loop", "1.0/1.0", "-12.0", "standby"),
    ("5", "Awards BGM", "Audio only, picture black", "bgm", "00:02:45.12", "black", "Black", "0.5/2.0", "-3.0", ""),
    ("6", "Award Winner Photos", "8 stills, 5s each", "award", "00:00:40.00", "next", "Next", "0.5/0.5", "—", ""),
    ("7", "Intermission Loop", "", "break", "00:00:45.00", "loop", "Loop", "1.0/1.0", "-9.0", ""),
    ("8", "Sponsor Reel", "", "sponsor", "00:01:00.00", "goto", "Go to 7", "0.5/0.5", "-6.0", ""),
    ("9", "Closing Credits", "GO when applause settles", "end", "00:02:10.05", "black", "Black", "—/3.0", "0.0", ""),
]

END_ICON = {
    "loop": "M-6,-2 A6,6 0 1 1 -2,5 M-6,-2 L-8,-7 M-6,-2 L-1,-3",
    "next": "M-7,0 L5,0 M1,-5 L6,0 L1,5",
    "hold": "M-5,-6 L-5,6 M5,-6 L5,6",
    "black": "M-6,-6 L6,-6 L6,6 L-6,6 Z",
    "goto": "M-7,4 L-7,-2 L5,-2 M1,-7 L6,-2 L1,3",
}

def cue_list(x0=0, y0=56, w=1110, h=704):
    rect(x0, y0, w, h, C["bg"])
    # タイトル
    text(x0 + 20, y0 + 30, "CUE LIST", size=13, font=MONO, weight=700, fill=C["sub"], extra='letter-spacing="1.5"')
    text(x0 + 104, y0 + 30, "9 cues · total 00:09:50.17", size=12, fill=C["dim"])
    bx = x0 + w - 20
    for label, wd, prim in [("Preflight ✓ All OK", 160, False), ("+ Add Cue", 110, False)]:
        bx -= wd
        rect(bx, y0 + 10, wd, 28, C["panel2"], C["line2"], 1, rx=5)
        text(bx + wd / 2, y0 + 29, label, size=12, fill=C["green"] if "OK" in label else C["text"], anchor="middle")
        bx -= 8
    # 列見出し
    hy = y0 + 48
    rect(x0, hy, w, 30, C["panel"])
    line(x0, hy + 30, x0 + w, hy + 30)
    cols = [(24, "#"), (72, "MEDIA"), (186, "NAME / NOTE"), (620, "DURATION"), (760, "AT END"), (900, "FADE IN/OUT"), (1030, "GAIN dB")]
    for cx, lb in cols:
        text(x0 + cx, hy + 20, lb, size=11, fill=C["sub"])
    rh = 68
    y = hy + 31
    for i, (num, name, note, kind, dur, end, endl, fade, vol, st) in enumerate(CUES):
        bgc = C["bg"] if i % 2 == 0 else "#0E1114"
        if st == "program":
            bgc = "#2A1416"
        rect(x0, y, w, rh, bgc)
        line(x0, y + rh, x0 + w, y + rh, C["line"])
        if st == "program":
            rect(x0, y, 6, rh, C["red"])
        if st == "standby":
            rect(x0 + 1.5, y + 1.5, w - 3, rh - 3, "none", C["yellow"], 2.5, rx=3)
            rect(x0, y, 6, rh, C["yellow"])
        o = "0.42" if st == "done" else "1"
        a(f'<g opacity="{o}">')
        text(x0 + 24, y + 41, num, size=20, font=MONO, weight=700,
             fill=C["red"] if st == "program" else C["yellow"] if st == "standby" else C["text"])
        thumb(x0 + 72, y + 7, 96, 54, kind)
        rect(x0 + 72, y + 7, 96, 54, "none", C["line2"], 1)
        text(x0 + 186, y + 30, name, size=16, weight=700 if st in ("program", "standby") else 500)
        if note:
            text(x0 + 186, y + 52, note, size=12, fill=C["sub"])
        # 状態バッジ
        if st == "program":
            rect(x0 + 470, y + 14, 92, 22, C["red"], rx=4)
            text(x0 + 516, y + 30, "● ON AIR", size=11, font=MONO, weight=700, anchor="middle", fill="#fff")
        elif st == "standby":
            rect(x0 + 470, y + 14, 92, 22, C["yellow"], rx=4)
            text(x0 + 516, y + 30, "STANDBY", size=11, font=MONO, weight=700, anchor="middle", fill="#1a1200")
        elif st == "done":
            text(x0 + 516, y + 30, "✓ Played", size=12, fill=C["sub"], anchor="middle")
        text(x0 + 620, y + 40, dur, size=15, font=MONO)
        # 終了動作
        a(f'<g transform="translate({x0+772},{y+35})"><path d="{END_ICON[end]}" fill="none" stroke="{C["blue"] if end in ("loop","next","goto") else C["sub"]}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></g>')
        text(x0 + 788, y + 40, endl, size=13)
        text(x0 + 900, y + 40, fade, size=14, font=MONO, fill=C["sub"])
        text(x0 + 1030, y + 40, vol, size=14, font=MONO, fill=C["sub"])
        a("</g>")
        if kind not in ("award",):  # 行の下端に小さな波形（音のない静止画は出さない）
            wave(x0 + 186, y + rh - 13, 900, 10, seed=i + 1, step=4,
                 color=C["line2"] if st != "done" else C["line"],
                 played=0.0763 if st == "program" else (1.0 if st == "done" else 0.0),
                 pcolor=C["red"] if st == "program" else C["line"])
        y += rh
    # スクロールバー
    rect(x0 + w - 8, hy + 34, 4, 300, C["line2"], rx=2)

# ---------- 右：モニター＋送出 ----------
def monitor(x, y, w, h, label, sub, color, kind, name, tc_left, tc_right):
    rect(x - 3, y - 30, w + 6, 27, color)
    text(x + 8, y - 11, label, size=14, font=MONO, weight=700, fill="#0E0F10", extra='letter-spacing="1"')
    text(x + w - 6, y - 11, sub.upper(), size=11, font=MONO, weight=600, fill="#0E0F10", anchor="end")
    rect(x - 3, y - 3, w + 6, h + 6, "#000", color, 3)
    thumb(x, y, w, h, kind)
    # セーフエリア
    rect(x + w * .05, y + h * .05, w * .9, h * .9, "none", "#ffffff", 0.8, extra='opacity="0.25" stroke-dasharray="4 4"')
    text(x, y + h + 26, name, size=15, weight=700)
    text(x, y + h + 48, tc_left, size=13, font=MONO, fill=C["sub"])
    text(x + w, y + h + 48, tc_right, size=13, font=MONO, fill=C["sub"], anchor="end")

def button(x, y, w, h, label, sub=None, fill=C["panel2"], stroke=C["line2"], fg=C["text"], size=20, keycap=None, sw=1.5):
    rect(x, y, w, h, fill, stroke, sw, rx=8)
    ty = y + h / 2 + size * .36 - (8 if sub else 0)
    text(x + w / 2, ty, label, size=size, font=MONO, weight=700, fill=fg, anchor="middle")
    if sub:
        text(x + w / 2, ty + 22, sub, size=11, fill=fg, anchor="middle", extra='opacity="0.75"')
    if keycap:
        kw = len(keycap) * 7 + 10
        rect(x + w - kw - 6, y + 6, kw, 16, "#000000", rx=3, extra='opacity="0.35"')
        text(x + w - kw / 2 - 6, y + 18, keycap, size=10, font=MONO, fill=fg, anchor="middle")

def playout(x0=1110, y0=56, w=810, h=704, locked=False):
    rect(x0, y0, w, h, C["panel"])
    line(x0, y0, x0, y0 + h, C["line2"])
    mw, mh = 372, 209
    mx1, mx2, my = x0 + 24, x0 + 24 + mw + 18, y0 + 56
    monitor(mx1, my, mw, mh, "PREVIEW", "Cued · frame 0", C["green"], "guest", "4  Guest Intro BG Loop", "IN 00:00:00.00", "DUR 00:00:20.00")
    monitor(mx2, my, mw, mh, "PROGRAM", "On air", C["red"], "open", "3  Opening Video", "00:00:06.52", "/ 00:01:30.00")
    # REMAIN
    ry = y0 + 330
    rect(x0 + 24, ry, w - 48, 128, "#0B0D10", C["line2"], 1, rx=10)
    text(x0 + 48, ry + 30, "REMAIN", size=13, font=MONO, weight=700, fill=C["sub"], extra='letter-spacing="2"')
    text(x0 + 48, ry + 106, "00:01:23.08", size=70, font=MONO, weight=700, fill=C["text"])
    # 右側の小さな数値
    sx = x0 + w - 48
    for i, (lb, val, col) in enumerate([("ELAPSED", "00:00:06.52", C["sub"]), ("AT END", "Hold last", C["text"]), ("NEXT", "4 Guest Intro", C["yellow"])]):
        yy = ry + 34 + i * 34
        text(sx - 176, yy, lb, size=11, font=MONO, fill=C["dim"])
        text(sx, yy, val, size=14, font=MONO if ":" in val else JP, fill=col, anchor="end")
    # 進行バー
    py = ry + 150
    bx, bw = x0 + 24, w - 48
    BH = 48
    rect(bx, py, bw, BH, "#08090A", C["line2"], 1)
    prog = 0.0724
    rect(bx + 1, py + 1, (bw - 2) * prog, BH - 2, C["red"], extra='opacity="0.18"')
    wave(bx + 3, py + 4, bw - 6, BH - 8, seed=3, step=2, color="#6E757C", played=prog, pcolor=C["red"])
    line(bx, py + BH / 2, bx + bw, py + BH / 2, C["line2"], 0.6)
    # フェードアウト区間（最後 0.5 秒）
    fo = (bw - 2) * (0.5 / 90)
    rect(bx + bw - 1 - fo, py + 1, fo, BH - 2, "url(#hatch)")
    text(bx + bw - 8, py + 14, "L+R", size=9, font=MONO, fill=C["dim"], anchor="end")
    # 残り10/20/30 秒の目盛り
    for s in (30, 20, 10):
        mxp = bx + bw * (1 - s / 90)
        line(mxp, py - 4, mxp, py + BH, C["yellow"], 1.2, 'opacity="0.7"')
        text(mxp, py - 9, f"-{s}s", size=10, font=MONO, fill=C["yellow"], anchor="middle", extra='opacity="0.8"')
    a(f'<path d="M{bx+(bw-2)*prog+1},{py-6} l-5,-7 h10 z" fill="{C["text"]}"/>')
    line(bx + (bw - 2) * prog + 1, py, bx + (bw - 2) * prog + 1, py + BH, "#ffffff", 1.5)
    text(bx, py + BH + 16, "IN 00:00:00.00", size=10, font=MONO, fill=C["dim"])
    text(bx + bw, py + BH + 16, "OUT 00:01:30.00", size=10, font=MONO, fill=C["dim"], anchor="end")
    # 送出ボタン
    ty = py + 74
    gw = 300
    button(x0 + 24, ty, gw, 116, "GO", "Play STANDBY cue", fill=C["green"], stroke=C["green"], fg="#04140b", size=44, keycap="SPACE")
    bw2 = (w - 48 - gw - 3 * 12) / 3
    bx2 = x0 + 24 + gw + 12
    button(bx2, ty, bw2, 52, "TAKE", "Play selected", keycap="ENTER", size=17)
    button(bx2 + bw2 + 12, ty, bw2, 52, "PAUSE", None, keycap="P", size=17)
    button(bx2 + 2 * (bw2 + 12), ty, bw2, 52, "STOP", "Hold", keycap="S", size=17,
           fg=C["dim"] if locked else C["text"])
    sw_ = (w - 48 - gw - 12 - 4 * 10) / 5
    for i, (lb, k) in enumerate([("-30", "3"), ("-20", "2"), ("-10", "1"), ("LOOP", "L")]):
        button(bx2 + i * (sw_ + 10), ty + 64, sw_, 52, lb, None, keycap=k, size=15,
               fg=C["yellow"] if lb.startswith("-") else C["text"])
    button(bx2 + 4 * (sw_ + 10), ty + 64, sw_ + 2, 52, "PANIC", "Esc×2", fill="#3A1214", stroke=C["red"], fg=C["red"], size=15, sw=2)

# ---------- 下段 ----------
BANK = [
    ("F1", "Pre-show Loop", "logo", "0:30", ""), ("F2", "Color Bars", "bars", "∞", ""),
    ("F3", "Applause SFX", "bgm", "0:08", ""), ("F4", "Intermission", "break", "0:45", ""),
    ("F5", "Notice Still", "photo", "∞", ""), ("F6", "Sponsor Logo", "sponsor", "∞", ""),
    ("F7", "Black", "black", "—", ""), ("F8", None, None, None, ""),
    ("F9", "Please Stand By", "end", "∞", "warn"), ("F10", "Jingle A", "open", "0:05", ""),
    ("F11", "Jingle B", "award", "0:05", ""), ("F12", None, None, None, ""),
    ("⇧F1", None, None, None, ""), ("⇧F2", None, None, None, ""), ("⇧F3", None, None, None, ""), ("⇧F4", None, None, None, ""),
]

def tabs(active):
    y0 = 760
    rect(0, y0, W, H - y0, C["panel"])
    line(0, y0, W, y0, C["line2"])
    x = 16
    for t in ["Clip Bank", "Cue Inspector", "Media", "Log"]:
        tw = len(t) * 8 + 40
        if t == active:
            rect(x, y0 + 6, tw, 40, C["panel2"])
            rect(x, y0 + 6, tw, 3, C["yellow"])
        text(x + tw / 2, y0 + 30, t, size=14, fill=C["text"] if t == active else C["sub"], anchor="middle",
             weight=700 if t == active else 400)
        x += tw + 6
    line(0, y0 + 46, 1680, y0 + 46, C["line"])

def bank_tab():
    tabs("Clip Bank")
    text(1664, 790, "Press to take to PROGRAM · transition: Dissolve 0.5s", size=12, fill=C["sub"], anchor="end")
    x0, y0, gw, gh, gap = 16, 818, 196, 122, 10
    for i, (k, name, kind, dur, flag) in enumerate(BANK):
        cx = x0 + (i % 8) * (gw + gap)
        cy = y0 + (i // 8) * (gh + gap)
        if name is None:
            rect(cx, cy, gw, gh, "none", C["line2"], 1.2, rx=8, extra='stroke-dasharray="5 5"')
            text(cx + gw / 2, cy + gh / 2 + 5, "Empty · drag a cue here", size=12, fill=C["dim"], anchor="middle")
            text(cx + 10, cy + 20, k, size=11, font=MONO, fill=C["dim"], weight=700)
            continue
        stroke = C["yellow"] if flag == "warn" else C["line2"]
        rect(cx, cy, gw, gh, C["panel2"], stroke, 1.5 if flag else 1, rx=8)
        a(f'<clipPath id="cb{i}"><rect x="{cx+1}" y="{cy+1}" width="{gw-2}" height="78" rx="7"/></clipPath>')
        a(f'<g clip-path="url(#cb{i})">')
        thumb(cx, cy, gw, 80, kind)
        a("</g>")
        rect(cx + 6, cy + 6, len(k) * 8 + 12, 18, "#000000", rx=4, extra='opacity="0.7"')
        text(cx + 12, cy + 19, k, size=11, font=MONO, fill="#fff", weight=700)
        text(cx + 10, cy + 100, name if len(name) <= 20 else name[:20] + "…", size=13, weight=500)
        text(cx + gw - 10, cy + 100, dur, size=11, font=MONO, fill=C["sub"], anchor="end")
        if kind not in ("bars", "black", "photo", "sponsor", "end", "logo"):
            wave(cx + 10, cy + 108, gw - 20, 9, seed=i + 2, step=2, color=C["line2"])
        if flag == "warn":
            text(cx + gw - 10, cy + 19, "EMERGENCY", size=11, fill=C["yellow"], anchor="end", weight=700)

def inspector_tab():
    tabs("Cue Inspector")
    text(1664, 790, "Non-destructive: source files are never modified", size=12, fill=C["sub"], anchor="end")
    # 左：素材情報
    thumb(16, 818, 232, 130, "guest")
    rect(16, 818, 232, 130, "none", C["line2"])
    text(16, 974, "4", size=22, font=MONO, weight=700, fill=C["yellow"])
    text(42, 973, "Guest Intro BG Loop", size=15, weight=700)
    for k, (lb, v) in enumerate([("File", "guest_loop.mov"), ("Codec", "ProRes 422 HQ"), ("Format", "1920×1080 59.94p"), ("Audio", "2 ch · −18.4 LUFS")]):
        text(16, 1000 + k * 19, lb, size=11, fill=C["dim"])
        text(248, 1000 + k * 19, v, size=11, fill=C["sub"], anchor="end", font=MONO if k else JP)

    def section(x, title, w):
        text(x, 828, title, size=11, font=MONO, weight=700, fill=C["sub"], extra='letter-spacing="1.5"')
        line(x + len(title) * 8 + 10, 824, x + w, 824, C["line"])
    def field(x, y, label, value, w=120, mono=True):
        text(x, y, label, size=11, fill=C["sub"])
        rect(x, y + 7, w, 30, "#0B0D10", C["line2"], 1.2, rx=5)
        text(x + 10, y + 27, value, size=13, font=MONO if mono else JP, fill=C["text"])
    def seg(x, y, label, opts, sel):
        text(x, y, label, size=11, fill=C["sub"])
        cx = x
        for o in opts:
            ow = len(o) * 7.4 + 18
            on = o == sel
            rect(cx, y + 7, ow, 30, C["blue"] if on else "#0B0D10", C["blue"] if on else C["line2"], 1.2, rx=5)
            text(cx + ow / 2, y + 27, o, size=12, fill="#0E0F10" if on else C["sub"], anchor="middle", weight=600 if on else 400)
            cx += ow + 4
    def slider(x, y, label, val, pos, w=300, center=False, unit=""):
        text(x, y, label, size=11, fill=C["sub"])
        text(x + w + (70 if label else 28), (y if label else y + 20), val + unit, size=12, font=MONO, fill=C["text"], anchor="end")
        ty = y + 16
        rect(x, ty, w, 4, C["line2"], rx=2)
        if center:
            c = x + w / 2
            line(c, ty - 4, c, ty + 8, C["dim"])
            x1, x2 = sorted([c, x + w * pos])
            rect(x1, ty, x2 - x1, 4, C["blue"], rx=2)
        else:
            rect(x, ty, w * pos, 4, C["blue"], rx=2)
        rect(x + w*pos - 4, ty - 6, 8, 16, "#E8EBEE", C["bg"], 1)
    # 1列目：TIMING
    c1 = 276
    section(c1, "TIMING", 440)
    field(c1, 850, "In", "00:00:00.00", w=136)
    field(c1 + 148, 850, "Out", "00:00:20.00", w=136)
    field(c1 + 296, 850, "Duration", "00:00:20.00", w=136)
    seg(c1, 906, "On start", ["Play", "Pause at first frame"], "Play")
    seg(c1, 962, "At end", ["Black", "Hold last", "Next", "Loop", "Go to…"], "Loop")
    field(c1, 1018, "Loops (0 = ∞)", "0", w=96)
    field(c1 + 108, 1018, "Speed", "1.00×", w=86)
    seg(c1 + 206, 1018, "Transition in", ["Cut", "Dissolve"], "Dissolve")
    field(c1 + 380, 1018, "Sec", "0.50", w=60)
    # 2列目：AUDIO
    c2 = 748
    section(c2, "AUDIO", 420)
    slider(c2, 852, "Gain", "−12.0", 0.30, w=330, center=True, unit=" dB")
    rect(c2, 884, 18, 18, C["blue"], rx=4)
    a(f'<path d="M{c2+4},{893} l4,4 l7,-8" fill="none" stroke="#0E0F10" stroke-width="2.2"/>')
    text(c2 + 26, 898, "Normalize loudness to", size=12)
    rect(c2 + 168, 881, 92, 24, "#0B0D10", C["line2"], 1.2, rx=4)
    text(c2 + 178, 898, "−23 LUFS", size=12, font=MONO)
    text(c2 + 270, 898, "→ applied +4.6 dB", size=11, fill=C["sub"])
    field(c2, 918, "Fade in", "1.0 s", w=96)
    field(c2 + 108, 918, "Fade out", "1.0 s", w=96)
    seg(c2 + 216, 918, "Mute", ["Off", "On"], "Off")
    text(c2, 988, "Channel map", size=11, fill=C["sub"])
    for k, (src, dst) in enumerate([("Src 1", "Out L"), ("Src 2", "Out R")]):
        yy = 996 + k * 36
        rect(c2, yy, 70, 28, "#0B0D10", C["line2"], 1.2, rx=4)
        text(c2 + 35, yy + 19, src, size=12, font=MONO, anchor="middle", fill=C["sub"])
        a(f'<path d="M{c2+78},{yy+14} h34 m-6,-5 l6,5 l-6,5" fill="none" stroke="{C["dim"]}" stroke-width="1.6"/>')
        rect(c2 + 120, yy, 92, 28, "#0B0D10", C["line2"], 1.2, rx=4)
        text(c2 + 130, yy + 19, dst + "  ▾", size=12, font=MONO)
    text(c2 + 228, 1012, "Pan", size=11, fill=C["sub"])
    slider(c2 + 228, 1028, "", "C", 0.5, w=120, center=True)
    # 3列目：PICTURE
    c3 = 1196
    section(c3, "PICTURE", 468)
    seg(c3, 850, "Fit", ["Fit", "Fill", "Stretch", "1:1"], "Fit")
    field(c3 + 230, 850, "Scale", "100 %", w=74)
    field(c3 + 314, 850, "X", "0", w=66)
    field(c3 + 390, 850, "Y", "0", w=66)
    text(c3, 920, "Crop  T / B / L / R", size=11, fill=C["sub"])
    for k, v in enumerate(["0", "0", "0", "0"]):
        rect(c3 + k * 58, 927, 50, 28, "#0B0D10", C["line2"], 1.2, rx=4)
        text(c3 + k * 58 + 10, 946, v, size=12, font=MONO)
    field(c3 + 240, 912, "Opacity", "100 %", w=86)
    seg(c3 + 338, 912, "Flip", ["H", "V"], "")
    slider(c3, 978, "Brightness", "0", 0.5, w=150, center=True)
    slider(c3 + 240, 978, "Contrast", "+6", 0.56, w=150, center=True)
    slider(c3, 1028, "Saturation", "0", 0.5, w=150, center=True)
    rect(c3 + 240, 1022, 110, 32, "none", C["line2"], 1.2, rx=5)
    text(c3 + 295, 1043, "Reset picture", size=12, fill=C["sub"], anchor="middle")
    rect(c3 + 358, 1022, 110, 32, "none", C["line2"], 1.2, rx=5)
    text(c3 + 413, 1043, "Preview ▸ PVW", size=12, fill=C["green"], anchor="middle")

def meters():
    x0 = 1680
    rect(x0, 760, 240, 320, C["panel"])
    line(x0, 760, x0, H, C["line2"])
    text(x0 + 20, 790, "MASTER", size=12, font=MONO, weight=700, fill=C["sub"], extra='letter-spacing="1.5"')
    text(x0 + 220, 790, "WASAPI default", size=10, fill=C["dim"], anchor="end")
    top, hgt = 812, 210
    for i, (lv, pk) in enumerate([(0.66, 0.74), (0.61, 0.70)]):
        mx = x0 + 34 + i * 34
        rect(mx, top, 22, hgt, "#0B0D10", C["line2"], 1, rx=3)
        rect(mx + 2, top + hgt * (1 - lv), 18, hgt * lv - 2, "url(#meter)", rx=2)
        line(mx + 2, top + hgt * (1 - pk), mx + 20, top + hgt * (1 - pk), "#ffffff", 2)
        text(mx + 11, top + hgt + 18, "LR"[i], size=11, font=MONO, fill=C["sub"], anchor="middle")
    for db, f in [("0", 0), ("-6", .14), ("-12", .28), ("-18", .42), ("-30", .64), ("-48", .88)]:
        yy = top + hgt * f
        line(x0 + 96, yy, x0 + 102, yy, C["dim"])
        text(x0 + 108, yy + 4, db, size=10, font=MONO, fill=C["dim"])
    # フェーダー値とミュート
    text(x0 + 150, 838, "LEVEL", size=11, fill=C["sub"])
    text(x0 + 150, 866, "0.0", size=24, font=MONO, weight=700)
    text(x0 + 150, 884, "dB", size=11, font=MONO, fill=C["sub"])
    rect(x0 + 150, 904, 70, 34, "none", C["line2"], 1.2, rx=5)
    text(x0 + 185, 926, "MUTE", size=12, font=MONO, weight=700, fill=C["sub"], anchor="middle")
    rect(x0 + 150, 948, 70, 34, "none", C["line2"], 1.2, rx=5)
    text(x0 + 185, 970, "BLACK", size=12, weight=700, fill=C["sub"], anchor="middle")

def build(variant):
    out.clear()
    a(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">')
    a(f"<title>TRAPDOOR main window（{'Clip Bank' if variant=='bank' else 'Cue Inspector'}）</title>")
    defs()
    rect(0, 0, W, H, C["bg"])
    header()
    cue_list()
    playout()
    bank_tab() if variant == "bank" else inspector_tab()
    meters()
    a("</svg>")
    return "\n".join(out)

if __name__ == "__main__":
    import pathlib
    d = pathlib.Path(sys.argv[1]); d.mkdir(parents=True, exist_ok=True)
    for v, fn in [("bank", "main-bank.svg"), ("inspector", "main-inspector.svg")]:
        (d / fn).write_text(build(v), encoding="utf-8")
        print("wrote", d / fn)
