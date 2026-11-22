# qkz_read.py — READ-ONLY register dump (cmd 0x52 only)
import hid, struct, time, sys

VID, PID = 0x31B2, 0x0111
RPT, CMD_READ = 0x4B, 0x52

def open_dev():
    for d in hid.enumerate(VID, PID):
        if d['interface_number'] == 3:
            h = hid.device(); h.open_path(d['path']); return h
    sys.exit("Interface 3 tidak ditemukan")

def read_reg(h, addr):
    tx = bytes([RPT]) + struct.pack('<I', addr) + bytes([CMD_READ, 0, 0, 0, 0, 0])
    assert tx[5] == CMD_READ and len(tx) == 11   # safety: never anything but READ
    h.write(tx)
    t0 = time.time()
    while time.time() - t0 < 0.5:
        rx = h.read(64, 100)
        if rx and rx[0] == RPT and len(rx) >= 11 and rx[5] == CMD_READ:
            return struct.unpack('<I', bytes(rx[7:11]))[0], rx
    return None, None

h = open_dev()

# Uji satu register dulu, tampilkan balasan mentah
val, raw = read_reg(h, 0x00)
print("Test 0x00 raw:", None if raw is None else " ".join(f"{b:02X}" for b in raw))
if val is None:
    sys.exit("Tidak ada balasan valid — protokol mungkin beda. Berhenti di sini.")

KNOWN = {0x18: "ADC EQ enable", 0x24: "DAC EQ enable", 0x3A: "PGA L", 0x3B: "PGA R",
         0x65: "Digital ADC gain", 0x66: "Digital DAC gain L/R"}
for i in range(5):
    KNOWN[0x26 + i*2] = f"DAC EQ band{i+1} A (freq<<16|gain*10)"
    KNOWN[0x27 + i*2] = f"DAC EQ band{i+1} B (type<<16|Q*1000)"

with open("qkz_dump.txt", "w") as f:
    for addr in range(0x00, 0x100):
        v, _ = read_reg(h, addr)
        line = f"0x{addr:02X} = " + ("timeout" if v is None else f"0x{v:08X}")
        if addr in KNOWN: line += f"   <- {KNOWN[addr]}"
        print(line); f.write(line + "\n")
h.close()
print("Tersimpan di qkz_dump.txt")