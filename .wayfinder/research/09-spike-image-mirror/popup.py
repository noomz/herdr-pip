# Throwaway spike: kitty graphics image in a herdr popup.
# Probes a=q / CSI 16t / winsize, then re-transmits one RGBA image (fixed id) at PIP_FPS.
# Env: PIP_FPS (10), PIP_SCALE (1.0 px scale), PIP_CLEAN (1 = explicit delete on exit), PIP_ZLIB (1),
#      PIP_ALT (1 = double-buffer: alternate ids 7/8, place new then delete old).
import os, re, sys, time, zlib, base64, select, termios, tty, fcntl, struct, signal

D = os.path.dirname(os.path.abspath(__file__))
LOG = open(os.path.join(D, "spike.log"), "a", buffering=1)
def log(*a): LOG.write("%d %s\n" % (int(time.time() * 1000), " ".join(str(x) for x in a)))

FPS = float(os.environ.get("PIP_FPS", "10"))
SCALE = float(os.environ.get("PIP_SCALE", "1"))
CLEAN = os.environ.get("PIP_CLEAN") == "1"
ZLIB = os.environ.get("PIP_ZLIB", "1") == "1"
IMG = 7
ALT = os.environ.get("PIP_ALT") == "1"
fd_in, out = sys.stdin.fileno(), sys.stdout.buffer
def w(b): out.write(b); out.flush()

def winsize():
    r, c, xp, yp = struct.unpack("HHHH", fcntl.ioctl(1, termios.TIOCGWINSZ, b"\0" * 8))
    return r, c, xp, yp

def probe():
    w(b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[16t\x1b[c")
    buf, t0 = b"", time.time()
    while time.time() - t0 < 1.0 and not re.search(rb"\x1b\[\?[\d;]*c", buf):
        if select.select([fd_in], [], [], 0.05)[0]:
            buf += os.read(fd_in, 4096)
    ms = int((time.time() - t0) * 1000)
    kitty = buf.split(b"\x1b_G", 1)[1].split(b"\x1b\\", 1)[0].decode(errors="replace") if b"\x1b_G" in buf else None
    cell = None
    if b"\x1b[6;" in buf:
        h, wd = buf.split(b"\x1b[6;", 1)[1].split(b"t", 1)[0].split(b";")
        cell = (int(wd), int(h))
    log("probe ms=%d raw=%r kitty=%s 16t=%s" % (ms, buf, kitty, cell))
    return kitty, cell

def frame_base(pw, ph, cw, ch):
    img = bytearray(pw * ph * 4)
    a, b = bytes((30, 40, 60, 255)), bytes((50, 70, 100, 255))
    for y in range(ph):
        cy = int(y / ch) if ch else 0
        row = bytearray()
        for x0 in range(0, pw, max(1, int(cw))):
            n = min(max(1, int(cw)), pw - x0)
            row += (a if ((x0 // max(1, int(cw))) + cy) % 2 else b) * n
        img[y * pw * 4:(y + 1) * pw * 4] = row[:pw * 4]
    red = bytes((255, 0, 0, 255))
    for y in range(ph):
        for x in (0, 1, pw - 2, pw - 1):
            img[(y * pw + x) * 4:(y * pw + x + 1) * 4] = red
    img[0:pw * 8] = red * (pw * 2)
    img[(ph - 2) * pw * 4:] = red * (pw * 2)
    return img

def send(img, pw, ph, cols, rows, iid=IMG, prev=None):
    t0 = time.time()
    data = zlib.compress(bytes(img), 1) if ZLIB else bytes(img)
    b64 = base64.standard_b64encode(data)
    t1 = time.time()
    parts = [b64[i:i + 4096] for i in range(0, len(b64), 4096)]
    ctl = "a=T,i=%d,p=1,q=2,f=32,s=%d,v=%d,c=%d,r=%d,C=1%s" % (iid, pw, ph, cols, rows, ",o=z" if ZLIB else "")
    msg = bytearray(b"\x1b[2;1H")
    for k, p in enumerate(parts):
        head = (ctl + ",") if k == 0 else ""
        msg += b"\x1b_G" + ("%sm=%d" % (head, 0 if k == len(parts) - 1 else 1)).encode() + b";" + p + b"\x1b\\"
    if prev: msg += b"\x1b_Ga=d,d=I,i=%d,q=2\x1b\\" % prev
    w(bytes(msg))
    t2 = time.time()
    return len(data), len(msg), (t1 - t0) * 1000, (t2 - t1) * 1000

def main():
    old = termios.tcgetattr(fd_in)
    tty.setraw(fd_in)
    w(b"\x1b[?25l\x1b[2J")
    kitty, cell = probe()
    resized = [True]
    signal.signal(signal.SIGWINCH, lambda *_: resized.__setitem__(0, True))
    n, stats, t_last = 0, [], time.time()
    try:
        while True:
            if resized[0]:
                resized[0] = False
                rows, cols, xp, yp = winsize()
                if cell: cw, ch, src = cell[0], cell[1], "16t"
                elif xp and yp: cw, ch, src = xp / cols, yp / rows, "winsize"
                else: cw, ch, src = 9, 18, "default"
                ir = rows - 1
                pw, ph = max(4, int(cols * cw * SCALE)), max(4, int(ir * ch * SCALE))
                base = frame_base(pw, ph, cw * SCALE, ch * SCALE)
                log("size rows=%d cols=%d winpx=%dx%d cell=%sx%s src=%s img=%dx%d raw=%dKB" % (rows, cols, xp, yp, cw, ch, src, pw, ph, len(base) // 1024))
                w(b"\x1b[2J")
            img = bytearray(base)
            bx = (n * 8) % max(1, pw - 20)
            bar = bytes((255, 220, 0, 255)) * 20
            for y in range(2, ph - 2):
                o = (y * pw + bx) * 4
                img[o:o + 80] = bar
            iid = (IMG + n % 2) if ALT else IMG
            prev = (IMG + (n + 1) % 2) if ALT and n else None
            comp, wire, enc_ms, wr_ms = send(img, pw, ph, cols, ir, iid, prev)
            stats.append((comp, wire, enc_ms, wr_ms))
            n += 1
            if n % 10 == 0:
                k = len(stats)
                log("frames=%d avg comp=%dKB wire=%dKB enc=%.1fms write=%.1fms" % (n, sum(s[0] for s in stats) // k // 1024, sum(s[1] for s in stats) // k // 1024, sum(s[2] for s in stats) / k, sum(s[3] for s in stats) / k))
                stats = []
            hdr = " pipimg  kitty=%s cell=%sx%s(%s) img=%dx%d %dKB/f  q quit" % (kitty, cw, ch, src, pw, ph, wire // 1024)
            w(b"\x1b[1;1H\x1b[7m" + hdr[:cols].ljust(cols).encode() + b"\x1b[0m")
            t_next = t_last + 1 / FPS
            while True:
                dt = t_next - time.time()
                if dt <= 0: break
                if select.select([fd_in], [], [], dt)[0]:
                    k = os.read(fd_in, 64)
                    if k[:1] in (b"q", b"\x1b"):
                        log("quit frames=%d clean=%s" % (n, CLEAN)); return
            t_last = time.time()
    finally:
        if CLEAN: w(b"\x1b_Ga=d,d=I,i=%d\x1b\\" % IMG)
        w(b"\x1b[?25h")
        termios.tcsetattr(fd_in, termios.TCSADRAIN, old)

log("start pid=%d env TERM=%s alt=%s" % (os.getpid(), os.environ.get("TERM"), ALT))
main()
