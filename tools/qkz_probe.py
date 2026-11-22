# qkz_probe.py — read-only enumeration
import hid

VID, PID = 0x31B2, 0x0111

print("=== HID interfaces ===")
found = False
for d in hid.enumerate(VID, PID):
    found = True
    print(f"path={d['path']}")
    print(f"  product={d['product_string']}  manufacturer={d['manufacturer_string']}")
    print(f"  interface={d['interface_number']}  usage_page=0x{d['usage_page']:04X}  usage=0x{d['usage']:04X}")
if not found:
    print("Tidak ada HID interface. Kontrol mungkin lewat UAC atau vendor control transfer.")

# Coba baca report descriptor (read-only)
for d in hid.enumerate(VID, PID):
    try:
        h = hid.device()
        h.open_path(d['path'])
        desc = h.get_report_descriptor()
        print(f"\nReport descriptor iface {d['interface_number']} ({len(desc)} bytes):")
        print(" ".join(f"{b:02X}" for b in desc))
        h.close()
    except Exception as e:
        print(f"  gagal membaca descriptor: {e}")